#![cfg(not(target_arch = "wasm32"))]

//! Azure Key Vault Secrets Client.

mod vault;

use std::env;
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
    async fn connect() -> Result<Self> {
        let options = ConnectOptions::from_env()?;
        Self::connect_with(&options).await
    }

    async fn connect_with(options: &Self::ConnectOptions) -> Result<Self> {
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

#[cfg(not(debug_assertions))]
pub struct ConnectOptions {
    pub address: String,
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}

#[cfg(debug_assertions)]
pub struct ConnectOptions {
    pub address: String,
}

impl ConnectOptions {
    /// Create connection options from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self> {
        let address = env::var("AZURE_KEYVAULT_ADDR")?;

        #[cfg(debug_assertions)]
        {
            Ok(Self { address })
        }

        #[cfg(not(debug_assertions))]
        {
            let tenant_id = env::var("AZURE_TENANT_ID")?;
            let client_id = env::var("AZURE_CLIENT_ID")?;
            let client_secret = env::var("AZURE_CLIENT_SECRET")?;

            Ok(Self {
                address,
                tenant_id,
                client_id,
                client_secret,
            })
        }
    }
}
