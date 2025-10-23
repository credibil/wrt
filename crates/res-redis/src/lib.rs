//! Redis Client.
#![allow(missing_docs)]
#![cfg(not(target_arch = "wasm32"))]
mod keyvalue;

use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use anyhow::{Result, anyhow};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use runtime::ResourceBuilder;
use tracing::instrument;

const DEF_REDIS_ADDR: &str = "redis://localhost:6379";
const CLIENT_NAME: &str = "res-redis";
const DEF_MAX_RETRIES: usize = 3;
const DEF_MAX_DELAY: u64 = 1000; // milliseconds

pub struct Redis {
    attributes: HashMap<String, String>,
}

#[derive(Clone)]
pub struct RedisClient(ConnectionManager);

impl Debug for RedisClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisClient").finish_non_exhaustive()
    }
}

impl ResourceBuilder<RedisClient> for Redis {
    fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    #[instrument(name = "Redis::connect", skip(self))]
    async fn connect(self) -> Result<RedisClient> {
        let addr = env::var("REDIS_ADDR").unwrap_or_else(|_| DEF_REDIS_ADDR.into());
        let client = connect(addr).map_err(|e| anyhow!("failed to create redis client: {e}"))?;
        let max_retries = std::env::var("REDIS_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEF_MAX_RETRIES);
        let max_delay = std::env::var("REDIS_MAX_DELAY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEF_MAX_DELAY);
        let config = ConnectionManagerConfig::new()
            .set_number_of_retries(max_retries)
            .set_max_delay(max_delay);
        let conn = client
            .get_connection_manager_with_config(config)
            .await
            .map_err(|e| anyhow!("issue getting connection: {e}"))?;
        tracing::info!("connected to redis");
        Ok(RedisClient(conn))
    }
}

impl IntoFuture for Redis {
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<RedisClient>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}

fn connect(addr: String) -> Result<redis::Client> {
    let client = redis::Client::open(addr).map_err(|e| anyhow!("{e}"))?;
    Ok(client)
}
