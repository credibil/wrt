//! Redis Client.
#![allow(missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

mod keyvalue;

use std::fmt::Debug;
use std::time::Duration;

use anyhow::{Context, Result};
use fromenv::FromEnv;
use kernel::Backend;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use tracing::instrument;

#[derive(Clone)]
pub struct Client(ConnectionManager);

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Backend for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "Redis::connect_with")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let client =
            redis::Client::open(options.url.clone()).context("failed to create redis client")?;
        let config = ConnectionManagerConfig::new()
            .set_number_of_retries(options.max_retries)
            .set_max_delay(Duration::from_millis(options.max_delay));

        let conn = client
            .get_connection_manager_with_config(config)
            .await
            .context("issue getting redis connection")?;

        tracing::info!("connected to redis");
        Ok(Self(conn))
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "REDIS_URL", default = "redis://localhost:6379")]
    pub url: String,
    #[env(from = "REDIS_MAX_RETRIES", default = "3")]
    pub max_retries: usize,
    #[env(from = "REDIS_MAX_DELAY", default = "1000")]
    pub max_delay: u64,
}

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().context("issue loading connection options")
    }
}
