#![cfg(not(target_arch = "wasm32"))]

//! NATS Client.

mod blobstore;
mod keyvalue;
mod messaging;

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_nats::{AuthError, ConnectOptions};
use tracing::instrument;

const DEF_NATS_ADDR: &str = "demo.nats.io";
const CLIENT_NAME: &str = "nats";

#[derive(Debug, Clone)]
pub struct NatsClient(async_nats::Client);

impl NatsClient {
    #[instrument]
    pub async fn connect() -> Result<Self> {
        let addr = env::var("NATS_ADDR").unwrap_or_else(|_| DEF_NATS_ADDR.into());

        let options = connect_options().map_err(|e| {
            tracing::error!("failed to connect to nats: {e}");
            anyhow!("failed to connect to nats: {e}")
        })?;

        // connect to NATS server
        let client = options.connect(addr).await.map_err(|e| anyhow!("{e}"))?;
        tracing::info!("connected to nats");

        Ok(Self(client))
    }
}

// impl IntoFuture for NatsClient {
//     type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
//     type Output = Result<Self>;

//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(self.connect())
//     }
// }

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
