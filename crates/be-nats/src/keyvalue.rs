use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::jetstream::kv::Config;
use async_nats::jetstream::{self, kv};
use futures::TryStreamExt;
use futures::future::FutureExt;
use wasi_keyvalue::{Bucket, FutureResult, WasiKeyValueCtx};

use crate::Client;

impl WasiKeyValueCtx for Client {
    fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>> {
        tracing::trace!("opening bucket: {identifier}");
        let client = self.inner.clone();

        async move {
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

                result.context("failed to create bucket")?
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
        Box::leak(self.0.name.clone().into_boxed_str())
    }

    fn get(&self, key: String) -> FutureResult<Option<Vec<u8>>> {
        tracing::trace!("getting key: {key}");
        let store = self.0.clone();

        async move {
            let entry = store.get(key).await.context("getting key")?;
            Ok(entry.map(Into::into))
        }
        .boxed()
    }

    fn set(&self, key: String, value: Vec<u8>) -> FutureResult<()> {
        tracing::trace!("setting key: {key}");
        let store = self.0.clone();

        async move {
            store.put(key, value.into()).await.context("setting key")?;
            Ok(())
        }
        .boxed()
    }

    fn delete(&self, key: String) -> FutureResult<()> {
        tracing::trace!("deleting key: {key}");
        let store = self.0.clone();

        async move {
            store.delete(key).await.context("deleting key")?;
            Ok(())
        }
        .boxed()
    }

    fn exists(&self, key: String) -> FutureResult<bool> {
        tracing::trace!("checking existence of key: {key}");
        let store = self.0.clone();

        async move {
            let entry = store.get(key).await.context("checking key")?;
            Ok(entry.is_some())
        }
        .boxed()
    }

    fn keys(&self) -> FutureResult<Vec<String>> {
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
