//! Azure Key Vault Secrets Client.

use std::collections::HashMap;
use std::env;
use std::pin::Pin;

use anyhow::{Context, Result, anyhow};
use mongodb::Client;
use runtime::ResourceBuilder;
use tracing::instrument;

pub struct MongoDb {
    attributes: HashMap<String, String>,
}

impl ResourceBuilder<Client> for MongoDb {
    fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    #[instrument(name = "MongoDb::connect", skip(self))]
    async fn connect(self) -> Result<Client> {
        let uri = env::var("MONGODB_URI").context("fetching MONGODB_URI env var")?;

        let client = mongodb::Client::with_uri_str(uri.clone()).await.map_err(|e| {
            let err = format!("failed to connect to mongo at: {e}");
            tracing::error!(err);
            anyhow!(err)
        })?;
        tracing::info!("connected to mongo");

        Ok(client)
    }
}

impl IntoFuture for MongoDb {
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<Client>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}
