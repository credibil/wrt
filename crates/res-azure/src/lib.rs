#![cfg(not(target_arch = "wasm32"))]

//! Azure Key Vault Secrets Client.

mod identity;
mod key_vault;

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use azure_core::credentials::Secret;
use azure_core::credentials::TokenCredential;
use azure_identity::ClientSecretCredential;
use azure_identity::DeveloperToolsCredential;
use azure_security_keyvault_secrets::SecretClient;
use fromenv::FromEnv;
use runtime::Resource;
use tracing::instrument;

#[derive(Clone)]
pub struct Client {
    key_vault: Option<Arc<SecretClient>>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzClient").finish()
    }
}

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        // connect to Azure API
        let credential: Arc<dyn TokenCredential> = if let Some(cred) = &options.credential {
            ClientSecretCredential::new(
                &cred.tenant_id,
                cred.client_id.clone(),
                Secret::new(cred.client_secret.clone()),
                None,
            )?
        } else {
            DeveloperToolsCredential::new(None)
                .map_err(|e| anyhow!("could not create credential: {e}"))?
        };
        tracing::info!("connected to azure api");

        let Some(url) = &options.keyvault_url else {
            tracing::info!("no azure keyvault url provided");
            return Ok(Self { key_vault: None });
        };

        let client = SecretClient::new(url, credential, None)
            .map_err(|e| anyhow!("failed to connect to azure keyvault: {e}"))?;
        tracing::info!("connected to azure keyvault");

        Ok(Self {
            key_vault: Some(Arc::new(client)),
        })
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(nested)]
    pub credential: Option<CredentialOptions>,
    #[env(from = "AZURE_KEYVAULT_URL")]
    pub keyvault_url: Option<String>,
}

#[derive(Debug, Clone, FromEnv)]
pub struct CredentialOptions {
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
