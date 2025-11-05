//! Redis Client.
#![allow(missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

mod keyvalue;

use std::fmt::Debug;

use anyhow::{Result, anyhow};
use fromenv::FromEnv;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use runtime::Resource;
use tracing::instrument;

#[derive(Clone)]
pub struct Client(ConnectionManager);

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument(name = "Redis::connect_with")]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let client = redis::Client::open(options.address.clone())
            .map_err(|e| anyhow!("failed to create redis client: {e}"))?;
        let config = ConnectionManagerConfig::new()
            .set_number_of_retries(options.max_retries)
            .set_max_delay(options.max_delay_ms);

        let conn = client
            .get_connection_manager_with_config(config)
            .await
            .map_err(|e| anyhow!("issue getting redis connection: {e}"))?;

        tracing::info!("connected to redis");
        Ok(Self(conn))
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "REDIS_ADDR", default = "redis://localhost:6379")]
    pub address: String,
    #[env(from = "REDIS_MAX_RETRIES", default = "3")]
    pub max_retries: usize,
    #[env(from = "REDIS_MAX_DELAY", default = "1000")]
    pub max_delay_ms: u64,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}
