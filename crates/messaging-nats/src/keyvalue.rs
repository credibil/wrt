use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_nats::jetstream::kv::Config;
use async_nats::jetstream::{self, kv};
use futures::TryStreamExt;
use futures::future::{BoxFuture, FutureExt};
use wasi_keyvalue::{Bucket, Client};

use crate::{CLIENT_NAME, NatsClient};

impl Client for NatsClient {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn open(&self, identifier: String) -> BoxFuture<'static, Result<Arc<dyn Bucket>>> {
        let client = self.0.clone();

        async move {
            tracing::trace!("opening bucket: {identifier}");

            let jetstream = jetstream::new(client);

            let store = if let Ok(store) = jetstream.get_key_value(&identifier).await {
                store
            } else {
                let result = jetstream
                    .create_key_value(Config {
                        bucket: identifier,
                        history: 1,
                        max_age: Duration::from_secs(10 * 60),
                        max_bytes: 100 * 1024 * 1024, // 100 MiB
                        ..Config::default()
                    })
                    .await;

                result.map_err(|e| anyhow!("failed to create bucket: {e}"))?
            };

            Ok(Arc::new(KvBucket(store)) as Arc<dyn Bucket>)
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct KvBucket(pub kv::Store);

impl Bucket for KvBucket {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    fn get(&self, key: String) -> BoxFuture<'static, Result<Option<Vec<u8>>>> {
        let store = self.0.clone();

        async move {
            tracing::trace!("getting key: {key}");

            let entry = store.get(key).await.context("getting key")?;
            Ok(entry.map(Into::into))
        }
        .boxed()
    }

    fn set(&self, key: String, value: Vec<u8>) -> BoxFuture<'static, Result<()>> {
        let store = self.0.clone();

        async move {
            tracing::trace!("setting key: {key}");
            store.put(key, value.into()).await.context("setting key")?;
            Ok(())
        }
        .boxed()
    }

    fn delete(&self, key: String) -> BoxFuture<'static, Result<()>> {
        let store = self.0.clone();

        async move {
            tracing::trace!("deleting key: {key}");

            store.delete(key).await.context("deleting key")?;
            Ok(())
        }
        .boxed()
    }

    fn exists(&self, key: String) -> BoxFuture<'static, Result<bool>> {
        let store = self.0.clone();

        async move {
            tracing::trace!("checking existence of key: {key}");

            let entry = store.get(key).await.context("checking key")?;
            Ok(entry.is_some())
        }
        .boxed()
    }

    fn keys(&self) -> BoxFuture<'static, Result<Vec<String>>> {
        let store = self.0.clone();

        async move {
            tracing::trace!("listing keys");

            let key_results = store.keys().await.context("listing keys")?;
            let keys =
                key_results.try_filter_map(|k| async move { Ok(Some(k)) }).try_collect().await?;

            Ok(keys)
        }
        .boxed()
    }
}
