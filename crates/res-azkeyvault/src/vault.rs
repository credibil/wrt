use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use azure_security_keyvault_secrets::SecretClient;
use azure_security_keyvault_secrets::models::{Secret, SetSecretParameters};
use base64ct::{Base64UrlUnpadded, Encoding};
use futures::TryStreamExt;
use futures::future::FutureExt;
use http::StatusCode;
use wasi_vault::{Client, FutureResult, Locker};

use crate::{AzClient, CLIENT_NAME};

impl Client for AzClient {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn open(&self, identifier: String) -> FutureResult<Arc<dyn Locker>> {
        tracing::trace!("opening locker: {identifier}");
        let client = Arc::clone(&self.0);

        async move {
            let locker = AzLocker {
                vault: client,
                identifier,
            };
            Ok(Arc::new(locker) as Arc<dyn Locker>)
        }
        .boxed()
    }
}

pub struct AzLocker {
    identifier: String,
    vault: Arc<SecretClient>,
}

impl Debug for AzLocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzLocker").finish()
    }
}

impl Locker for AzLocker {
    fn identifier(&self) -> String {
        self.identifier.clone()
    }

    fn get(&self, secret_id: String) -> FutureResult<Option<Vec<u8>>> {
        tracing::debug!("getting secret: {secret_id}");
        let vault = Arc::clone(&self.vault);
        let identifier = self.identifier.clone();

        async move {
            let secret_name = format!("{identifier}-{secret_id}");
            let secret_id = Base64UrlUnpadded::encode_string(secret_name.as_bytes());

            let result = vault.get_secret(&secret_id, None).await;
            let response = match result {
                Ok(resp) => resp,
                Err(e) => {
                    if let Some(code) = e.http_status()
                        && code == StatusCode::NOT_FOUND.as_u16()
                    {
                        return Ok(None);
                    }
                    return Err(anyhow!("issue getting secret: {e}"));
                }
            };

            let secret: Secret = response.into_body().context("issue deserializing secret")?;
            let Some(value) = secret.value else {
                return Ok(None);
            };
            let decoded = Base64UrlUnpadded::decode_vec(&value).context("issue decoding secret")?;

            Ok(Some(decoded))
        }
        .boxed()
    }

    fn set(&self, secret_id: String, value: Vec<u8>) -> FutureResult<()> {
        tracing::debug!("setting secret: {secret_id}");
        let vault = Arc::clone(&self.vault);
        let identifier = self.identifier.clone();

        async move {
            let secret_name = format!("{identifier}-{secret_id}");
            let secret_id = Base64UrlUnpadded::encode_string(secret_name.as_bytes());

            let params = SetSecretParameters {
                value: Some(Base64UrlUnpadded::encode_string(&value)),
                ..SetSecretParameters::default()
            };
            let content = params.try_into().context("converting params to content")?;
            vault.set_secret(&secret_id, content, None).await.context("issue setting secret")?;

            Ok(())
        }
        .boxed()
    }

    fn delete(&self, secret_id: String) -> FutureResult<()> {
        tracing::trace!("deleting secret: {secret_id}");
        let vault = Arc::clone(&self.vault);
        let identifier = self.identifier.clone();

        async move {
            let secret_name = format!("{identifier}-{secret_id}");
            let secret_id = Base64UrlUnpadded::encode_string(secret_name.as_bytes());
            vault.delete_secret(&secret_id, None).await.context("issue deleting secret")?;

            Ok(())
        }
        .boxed()
    }

    fn exists(&self, secret_id: String) -> FutureResult<bool> {
        tracing::trace!("checking existence of {secret_id}");
        let future_result = self.get(secret_id);
        async move { Ok(future_result.await?.is_some()) }.boxed()
    }

    fn list_ids(&self) -> FutureResult<Vec<String>> {
        tracing::trace!("listing keys");
        let vault = Arc::clone(&self.vault);
        let identifier = self.identifier.clone();

        async move {
            let iter = vault.list_secret_properties(None).context("issue listing secrets")?;

            // filter and collect secret IDs for this 'locker'
            let secret_ids: Vec<String> = iter
                .try_filter_map(|props| async {
                    let Some(id) = props.id else {
                        return Ok(None);
                    };
                    Ok(id.strip_prefix(&format!("{identifier}-")).map(ToString::to_string))
                })
                .try_collect()
                .await
                .context("issue collecting secrets")?;

            Ok(secret_ids)
        }
        .boxed()
    }
}
