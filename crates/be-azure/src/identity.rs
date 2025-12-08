use std::fmt::Debug;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use azure_core::credentials::TokenCredential;
use azure_identity::{ManagedIdentityCredential, ManagedIdentityCredentialOptions, UserAssignedId};
use futures::future::FutureExt;
use wasi_identity::{AccessToken, FutureResult, Identity, WasiIdentityCtx};

use crate::Client;

impl WasiIdentityCtx for Client {
    fn get_identity(&self, name: String) -> FutureResult<Arc<dyn Identity>> {
        tracing::trace!("opening identity: {name}");

        async move {
            let identity = AzIdentity { name };
            Ok(Arc::new(identity) as Arc<dyn Identity>)
        }
        .boxed()
    }
}

pub struct AzIdentity {
    name: String,
}

impl Debug for AzIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzIdentity").finish()
    }
}

impl Identity for AzIdentity {
    fn get_token(&self, scopes: Vec<String>) -> FutureResult<AccessToken> {
        tracing::debug!("getting token for identity: {}", self.name);
        let name = self.name.clone();

        async move {
            let options = ManagedIdentityCredentialOptions {
                user_assigned_id: Some(UserAssignedId::ClientId(name)),
                ..Default::default()
            };
            let credential = ManagedIdentityCredential::new(Some(options))?;
            let scope = scopes.iter().map(AsRef::as_ref).collect::<Vec<_>>();
            let access_token = credential.get_token(&scope, None).await?;

            let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let expires_ts = access_token.expires_on.to_utc().unix_timestamp().unsigned_abs();
            let expires_in = expires_ts.saturating_sub(now_ts);

            Ok(AccessToken {
                token: access_token.token.secret().into(),
                expires_in,
            })
        }
        .boxed()
    }
}
