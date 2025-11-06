#![cfg(not(target_arch = "wasm32"))]

//! MongoDB Client.

mod blobstore;

use anyhow::{Result, anyhow};
use fromenv::FromEnv;
use runtime::Resource;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Client(mongodb::Client);

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "MongoDb::connect")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let client = mongodb::Client::with_uri_str(&options.uri)
            .await
            .map_err(|e| anyhow!("failed to connect to mongo: {e}"))?;
        tracing::info!("connected to mongo");

        Ok(Self(client))
    }
}

#[derive(Clone, Debug, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "MONGODB_URL")]
    pub uri: String,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}
