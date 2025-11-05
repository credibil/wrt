#![cfg(not(target_arch = "wasm32"))]

//! NATS Client.

mod blobstore;
mod keyvalue;
mod messaging;

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_nats::AuthError;
use fromenv::{FromEnv, ParseResult};
use runtime::Resource;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Client {
    inner: async_nats::Client,
    topics: Vec<String>,
}

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
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

        Ok(Self {
            inner: client,
            topics: options.topics,
        })
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "NATS_ADDRESS", default = "demo.nats.io")]
    pub address: String,
    #[env(from = "NATS_TOPICS", with = split)]
    pub topics: Vec<String>,
    #[env(from = "NATS_JWT")]
    pub jwt: Option<String>,
    #[env(from = "NATS_SEED")]
    pub seed: Option<String>,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

#[allow(clippy::unnecessary_wraps)]
fn split(s: &str) -> ParseResult<Vec<String>> {
    Ok(s.split(',').map(ToOwned::to_owned).collect())
}
