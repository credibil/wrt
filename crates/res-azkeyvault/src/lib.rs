#![cfg(not(target_arch = "wasm32"))]

//! Azure Key Vault Secrets Client.

mod vault;

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Result, anyhow};
#[cfg(not(debug_assertions))]
use azure_core::credentials::Secret;
use azure_core::credentials::TokenCredential;
#[cfg(not(debug_assertions))]
use azure_identity::ClientSecretCredential;
use azure_identity::DeveloperToolsCredential;
use azure_security_keyvault_secrets::SecretClient;
#[cfg(debug_assertions)]
use fromenv::FromEnv;
use runtime::Resource;
use tracing::instrument;

#[derive(Clone)]
pub struct Client(Arc<SecretClient>);

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzClient").finish()
    }
}

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        #[cfg(debug_assertions)]
        let credential: Arc<dyn TokenCredential> = {
            DeveloperToolsCredential::new(None)
                .map_err(|e| anyhow!("could not create credential: {e}"))?
        };

        #[cfg(not(debug_assertions))]
        let credential: Arc<dyn TokenCredential> = {
            let secret = Secret::new(options.client_secret.clone());
            ClientSecretCredential::new(
                &options.tenant_id,
                options.client_id.clone(),
                secret,
                None,
            )?
        };

        let client = SecretClient::new(&options.address, credential, None)
            .map_err(|e| anyhow!("failed to connect to azure keyvault: {e}"))?;
        tracing::info!("connected to azure keyvault");

        Ok(Self(Arc::new(client)))
    }
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "AZURE_KEYVAULT_ADDR")]
    pub address: String,
}

#[cfg(not(debug_assertions))]
#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "AZURE_KEYVAULT_ADDR")]
    pub address: String,
    #[env(from = "AZURE_TENANT_ID")]
    pub tenant_id: String,
    #[env(from = "AZURE_CLIENT_ID")]
    pub client_id: String,
    #[env(from = "AZURE_CLIENT_SECRET")]
    pub client_secret: String,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}
