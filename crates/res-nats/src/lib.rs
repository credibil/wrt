//! NATS Client.

mod blobstore;
mod keyvalue;
mod messaging;

use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_nats::{AuthError, ConnectOptions};
use runtime::ResourceBuilder;
use tracing::instrument;

const DEF_NATS_ADDR: &str = "demo.nats.io";
const CLIENT_NAME: &str = "nats";

#[derive(Debug, Clone)]
pub struct Nats {
    attributes: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct NatsClient(async_nats::Client);

impl ResourceBuilder<NatsClient> for Nats {
    fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    #[instrument(name = "Nats::connect", skip(self))]
    async fn connect(self) -> Result<NatsClient> {
        let addr = env::var("NATS_ADDR").unwrap_or_else(|_| DEF_NATS_ADDR.into());

        let options = connect_options().map_err(|e| {
            tracing::error!("failed to connect to nats: {e}");
            anyhow!("failed to connect to nats: {e}")
        })?;

        // connect to NATS server
        let client = options.connect(addr).await.map_err(|e| anyhow!("{e}"))?;
        tracing::info!("connected to nats");

        Ok(NatsClient(client))
    }
}

impl IntoFuture for Nats {
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<NatsClient>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}

fn connect_options() -> Result<ConnectOptions> {
    let mut opts = ConnectOptions::new();

    let jwt = env::var("NATS_JWT").ok();
    let seed = env::var("NATS_SEED").ok();

    // if there is a JWT, use it to authenticate
    if let Some(jwt) = jwt {
        let key_pair =
            nkeys::KeyPair::from_seed(&seed.unwrap_or_default()).context("creating KeyPair")?;
        let key_pair = Arc::new(key_pair);

        opts = opts.jwt(jwt, move |nonce| {
            let key_pair = Arc::clone(&key_pair);
            async move { key_pair.sign(&nonce).map_err(AuthError::new) }
        });
    }

    Ok(opts)
}
