//! Redis Client.
#![allow(missing_docs)]
#![cfg(not(target_arch = "wasm32"))]
mod keyvalue;

use std::env;
use std::fmt::Debug;

use anyhow::{Result, anyhow};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use runtime::Resource;
use tracing::instrument;

const DEF_REDIS_ADDR: &str = "redis://localhost:6379";
const DEF_MAX_RETRIES: usize = 3;
const DEF_MAX_DELAY: u64 = 1000; // milliseconds

#[derive(Clone)]
pub struct Client(ConnectionManager);

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Resource for Client {
    #[instrument(name = "Redis::connect")]
    async fn connect() -> Result<Self> {
        let addr = env::var("REDIS_ADDR").unwrap_or_else(|_| DEF_REDIS_ADDR.into());
        let client =
            redis::Client::open(addr).map_err(|e| anyhow!("failed to create redis client: {e}"))?;
        let max_retries = env::var("REDIS_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEF_MAX_RETRIES);
        let max_delay =
            env::var("REDIS_MAX_DELAY").ok().and_then(|s| s.parse().ok()).unwrap_or(DEF_MAX_DELAY);
        let config = ConnectionManagerConfig::new()
            .set_number_of_retries(max_retries)
            .set_max_delay(max_delay);

        let conn = client
            .get_connection_manager_with_config(config)
            .await
            .map_err(|e| anyhow!("issue getting redis connection: {e}"))?;

        tracing::info!("connected to redis");
        Ok(Self(conn))
    }
}

// impl IntoFuture for Redis {
//     type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
//     type Output = Result<Client>;

//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(self.connect())
//     }
// }
