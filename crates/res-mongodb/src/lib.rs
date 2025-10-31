#![cfg(not(target_arch = "wasm32"))]

//! MongoDB Client.

mod blobstore;

use std::env;

use anyhow::{Context, Result, anyhow};
use runtime::Resource;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Client(mongodb::Client);

impl Resource for Client {
    #[instrument(name = "MongoDb::connect")]
    async fn connect() -> Result<Self> {
        let uri = env::var("MONGODB_URI").context("fetching MONGODB_URI env var")?;

        let client = mongodb::Client::with_uri_str(uri.clone()).await.map_err(|e| {
            let err = format!("failed to connect to mongo at: {e}");
            tracing::error!(err);
            anyhow!(err)
        })?;
        tracing::info!("connected to mongo");

        Ok(Self(client))
    }
}

// impl IntoFuture for Client {
//     type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
//     type Output = Result<Client>;

//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(self.connect())
//     }
// }
