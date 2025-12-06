#![cfg(not(target_arch = "wasm32"))]

//! MongoDB Client.

mod blobstore;

use anyhow::{Context, Result};
use fromenv::FromEnv;
use kernel::Backend;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Client(mongodb::Client);

impl Backend for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "MongoDb::connect")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let client = mongodb::Client::with_uri_str(&options.uri)
            .await
            .context("failed to connect to mongo")?;
        tracing::info!("connected to mongo");

        Ok(Self(client))
    }
}

#[derive(Clone, Debug, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "MONGODB_URL")]
    pub uri: String,
}

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().context("issue loading connection options")
    }
}
