//! Messaging implementation for the Redis resource.
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Context;
use futures::FutureExt;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use wasi_keyvalue::{Bucket, FutureResult, WasiKeyValueCtx};

use crate::Client;

const TTL_DAY: u64 = 24 * 60 * 60; // 1 day

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
        let key = format!("{}:{key}", self.identifier);
        let mut conn = self.conn.0.clone();
        async move {
            conn.get(key.clone()).await.with_context(|| format!("failed to get value for {key}"))
        }
        .boxed()
    }

    fn set(&self, key: String, value: Vec<u8>) -> FutureResult<()> {
        let key = format!("{}:{key}", self.identifier);
        let mut conn = self.conn.0.clone();

        async move {
            conn.set_ex(&key, value, TTL_DAY)
                .await
                .with_context(|| format!("failed to set value for {key}"))
        }
        .boxed()
    }

    fn delete(&self, key: String) -> FutureResult<()> {
        let key = format!("{}:{key}", self.identifier);
        let mut conn = self.conn.0.clone();
        async move {
            conn.del(key.clone()).await.with_context(|| format!("failed to delete value for {key}"))
        }
        .boxed()
    }

    fn exists(&self, key: String) -> FutureResult<bool> {
        let key = format!("{}:{key}", self.identifier);
        let mut conn = self.conn.0.clone();
        async move {
            conn.exists(key.clone())
                .await
                .with_context(|| format!("failed to check existence of key {key}"))
        }
        .boxed()
    }

    fn keys(&self) -> FutureResult<Vec<String>> {
        let mut conn = self.conn.0.clone();
        let pattern = format!("{}:*", self.identifier);
        async move {
            conn.keys(pattern.clone())
                .await
                .with_context(|| format!("failed to list keys for {pattern}"))
        }
        .boxed()
    }
}
