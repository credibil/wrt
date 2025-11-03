#![cfg(not(target_arch = "wasm32"))]

//! MongoDB Client.

mod blobstore;

use std::env;

use anyhow::{Result, anyhow};
use runtime::Resource;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Client(mongodb::Client);

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "MongoDb::connect")]
    async fn connect() -> Result<Self> {
        let options = ConnectOptions::from_env()?;
        Self::connect_with(&options).await
    }

    async fn connect_with(options: &Self::ConnectOptions) -> Result<Self> {
        let client = mongodb::Client::with_uri_str(&options.uri)
            .await
            .map_err(|e| anyhow!("failed to connect to mongo: {e}"))?;
        tracing::info!("connected to mongo");

        Ok(Self(client))
    }
}

pub struct ConnectOptions {
    pub uri: String,
}

impl ConnectOptions {
    /// Create connection options from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self> {
        let uri = env::var("MONGODB_URI")?;
        Ok(Self { uri })
    }
}
