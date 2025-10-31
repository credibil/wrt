#![cfg(not(target_arch = "wasm32"))]

//! Azure Key Vault Secrets Client.

mod vault;

use std::env;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use azure_core::credentials::{Secret, TokenCredential};
use azure_identity::{ClientSecretCredential, DeveloperToolsCredential};
use azure_security_keyvault_secrets::SecretClient;
use runtime::Resource;
use tracing::instrument;

const DEF_KV_ADDR: &str = "https://kv-credibil-demo.vault.azure.net";

#[derive(Clone)]
pub struct Client(Arc<SecretClient>);

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzClient").finish()
    }
}

impl Resource for Client {
    // type Connection = Self;

    #[instrument]
    async fn connect() -> Result<Self> {
        let addr = env::var("KV_ADDR").unwrap_or_else(|_| DEF_KV_ADDR.into());

        let credential: Arc<dyn TokenCredential> = if cfg!(debug_assertions) {
            DeveloperToolsCredential::new(None)
                .map_err(|e| anyhow!("could not create credential: {e}"))?
        } else {
            let tenant_id = env::var("AZURE_TENANT_ID")?;
            let client_id = env::var("AZURE_CLIENT_ID")?;
            let client_secret = env::var("AZURE_CLIENT_SECRET")?;
            let secret = Secret::new(client_secret);
            ClientSecretCredential::new(&tenant_id, client_id, secret, None)?
        };

        let client = SecretClient::new(&addr, credential, None)
            .map_err(|e| anyhow!("failed to connect to azure keyvault: {e}"))?;
        tracing::info!("connected to azure keyvault");

        Ok(Self(Arc::new(client)))
    }
}

// impl IntoFuture for AzKeyVault {
//     type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
//     type Output = Result<Client>;

//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(self.connect())
//     }
// }
