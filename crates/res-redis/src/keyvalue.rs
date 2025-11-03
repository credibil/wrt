//! Messaging implementation for the Redis resource.
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::anyhow;
use futures::FutureExt;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use wasi_keyvalue::{Bucket, FutureResult, TtlValue, WasiKeyValueCtx};

use crate::Client;

impl WasiKeyValueCtx for Client {
    fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>> {
        tracing::trace!("opening redis bucket: {}", identifier);
        let conn = self.0.clone();

        async move {
            let bucket = RedisBucket {
                identifier,
                conn: Conn(conn.clone()),
            };
            Ok(Arc::new(bucket) as Arc<dyn Bucket>)
        }
        .boxed()
    }
}

pub struct Conn(ConnectionManager);

impl Debug for Conn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionManager").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct RedisBucket {
    pub identifier: String,
    pub conn: Conn,
}

impl Bucket for RedisBucket {
    fn name(&self) -> &'static str {
        Box::leak(self.identifier.clone().into_boxed_str())
    }

    fn get(&self, key: String) -> FutureResult<Option<Vec<u8>>> {
        let key = format!("{}:{}", self.identifier, key);
        let mut conn = self.conn.0.clone();
        async move {
            conn.get(key.clone()).await.map_err(|e| anyhow!("failed to get value for {key}: {e}"))
        }
        .boxed()
    }

    fn set(&self, key: String, value: Vec<u8>) -> FutureResult<()> {
        let key = format!("{}:{}", self.identifier, key);
        let mut conn = self.conn.0.clone();
        async move {
            let ttl =
                TtlValue::try_from(value.clone()).ok().and_then(|ttl_value| ttl_value.ttl_seconds);

            if let Some(ttl) = ttl {
                tracing::trace!("setting key with TTL: {key}, {} seconds", ttl);
                conn.set_ex(key.clone(), value, ttl)
                    .await
                    .map_err(|e| anyhow!("failed to set value for {key}: {e}"))
            } else {
                conn.set(key.clone(), value)
                    .await
                    .map_err(|e| anyhow!("failed to set value for {key}: {e}"))
            }
        }
        .boxed()
    }

    fn delete(&self, key: String) -> FutureResult<()> {
        let key = format!("{}:{}", self.identifier, key);
        let mut conn = self.conn.0.clone();
        async move {
            conn.del(key.clone())
                .await
                .map_err(|e| anyhow!("failed to delete value for {key}: {e}"))
        }
        .boxed()
    }

    fn exists(&self, key: String) -> FutureResult<bool> {
        let key = format!("{}:{}", self.identifier, key);
        let mut conn = self.conn.0.clone();
        async move {
            conn.exists(key.clone())
                .await
                .map_err(|e| anyhow!("failed to check existence of key {key}: {e}"))
        }
        .boxed()
    }

    fn keys(&self) -> FutureResult<Vec<String>> {
        let mut conn = self.conn.0.clone();
        let pattern = format!("{}:*", self.identifier);
        async move {
            conn.keys(pattern.clone())
                .await
                .map_err(|e| anyhow!("failed to list keys for {pattern}: {e}"))
        }
        .boxed()
    }
}
