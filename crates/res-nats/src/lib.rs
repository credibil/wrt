#![cfg(not(target_arch = "wasm32"))]

//! NATS Client.

mod blobstore;
mod keyvalue;
mod messaging;

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_nats::AuthError;
use runtime::Resource;
use tracing::instrument;

const DEF_NATS_ADDR: &str = "demo.nats.io";

#[derive(Debug, Clone)]
pub struct Client(async_nats::Client);

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect() -> Result<Self> {
        let options = ConnectOptions::from_env()?;
        Self::connect_with(&options).await
    }

    async fn connect_with(options: &Self::ConnectOptions) -> Result<Self> {
        let mut nats_opts = async_nats::ConnectOptions::new();

        if let Some(jwt) = &options.jwt
            && let Some(seed) = &options.seed
        {
            let key_pair = nkeys::KeyPair::from_seed(seed).context("creating KeyPair")?;
            let key_pair = Arc::new(key_pair);

            nats_opts = nats_opts.jwt(jwt.clone(), move |nonce| {
                let key_pair = Arc::clone(&key_pair);
                async move { key_pair.sign(&nonce).map_err(AuthError::new) }
            });
        }

        let client = nats_opts.connect(&options.address).await.map_err(|e| anyhow!("{e}"))?;

        Ok(Self(client))
    }
}

pub struct ConnectOptions {
    pub address: String,
    pub topics: Vec<String>,
    pub jwt: Option<String>,
    pub seed: Option<String>,
}

impl ConnectOptions {
    /// Create connection options from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self> {
        let address = env::var("NATS_ADDRESS").unwrap_or_else(|_| DEF_NATS_ADDR.into());
        let topics_env = env::var("NATS_TOPICS")?;
        let topics = topics_env.split(',').map(Into::into).collect();
        let jwt = env::var("NATS_JWT").ok();
        let seed = env::var("NATS_SEED").ok();

        Ok(Self {
            address,
            topics,
            jwt,
            seed,
        })
    }
}
