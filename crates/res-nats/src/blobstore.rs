use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use async_nats::jetstream;
use async_nats::jetstream::object_store::{Config, ObjectStore};
use chrono::Utc;
use futures::{FutureExt, StreamExt};
use tokio::io::AsyncReadExt;
use wasi_blobstore::{Client, Container, ContainerMetadata, FutureResult, ObjectMetadata};

use crate::{CLIENT_NAME, NatsClient};

impl Client for NatsClient {
    fn name(&self) -> &'static str {
        CLIENT_NAME
    }

    /// Open a container.
    fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        tracing::trace!("creating container: {name}");
        let client = self.0.clone();

        async move {
            let store = jetstream::new(client)
                .create_object_store(Config {
                    bucket: name.clone(),
                    ..Default::default()
                })
                .await
                .context("creating object store")?;

            let metadata = metadata(name);

            Ok(Arc::new(NatsContainer { metadata, store }) as Arc<dyn Container>)
        }
        .boxed()
    }

    /// Get a container.
    fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        tracing::trace!("getting container: {name}");
        let client = self.0.clone();

        async move {
            let store = jetstream::new(client)
                .get_object_store(&name)
                .await
                .context("getting object store")?;
            let metadata = metadata(name);

            Ok(Arc::new(NatsContainer { metadata, store }) as Arc<dyn Container>)
        }
        .boxed()
    }

    /// Delete a container.
    fn delete_container(&self, name: String) -> FutureResult<()> {
        tracing::trace!("deleting container: {name}");
        let client = self.0.clone();

        async move {
            jetstream::new(client)
                .delete_object_store(&name)
                .await
                .map_err(|e| anyhow!("issue deleting object store: {e}"))?;
            Ok(())
        }
        .boxed()
    }

    /// Check if a container exists.
    fn container_exists(&self, name: String) -> FutureResult<bool> {
        tracing::trace!("checking existence of container: {name}");
        let client = self.0.clone();

        async move {
            let exists = jetstream::new(client).get_object_store(&name).await.is_ok();
            Ok(exists)
        }
        .boxed()
    }
}

fn metadata(name: String) -> ContainerMetadata {
    #[allow(clippy::cast_sign_loss)]
    ContainerMetadata {
        name,
        created_at: Utc::now().timestamp() as u64,
    }
}

pub struct NatsContainer {
    pub metadata: ContainerMetadata,
    pub store: ObjectStore,
}

impl Debug for NatsContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NatsContainer").finish()
    }
}

impl Container for NatsContainer {
    fn name(&self) -> FutureResult<String> {
        tracing::trace!("getting container name");
        let name = self.metadata.name.clone();
        async move { Ok(name) }.boxed()
    }

    fn info(&self) -> FutureResult<ContainerMetadata> {
        let metadata = self.metadata.clone();
        async move { Ok(metadata) }.boxed()
    }

    /// Get the value associated with the key.
    fn get_data(&self, name: String, _start: u64, _end: u64) -> FutureResult<Option<Vec<u8>>> {
        tracing::trace!("getting object data: {name}");
        let store = self.store.clone();

        async move {
            let mut object = store.get(name).await.context("getting object")?;
            let mut bytes = vec![];
            object.read_to_end(&mut bytes).await?;
            Ok(Some(bytes))
        }
        .boxed()
    }

    /// Set the value associated with the key.
    fn write_data(&self, name: String, data: Vec<u8>) -> FutureResult<()> {
        tracing::trace!("writing object data: {name}");
        let store = self.store.clone();

        async move {
            store.put(name.as_str(), &mut data.as_slice()).await.context("writing object")?;
            Ok(())
        }
        .boxed()
    }

    /// List all objects in the container.
    fn list_objects(&self) -> FutureResult<Vec<String>> {
        tracing::trace!("listing objects");
        let store = self.store.clone();

        async move {
            let mut objects = store.list().await.context("listing objects")?;

            let mut names = vec![];
            while let Some(n) = objects.next().await {
                let Ok(obj_info) = n else {
                    tracing::warn!("issue listing object");
                    continue;
                };
                names.push(obj_info.name);
            }

            Ok(names)
        }
        .boxed()
    }

    /// Delete the value associated with the key.
    fn delete_object(&self, name: String) -> FutureResult<()> {
        let store = self.store.clone();

        async move { store.delete(name).await.context("deleting object") }.boxed()
    }

    /// Check if the object exists.
    fn has_object(&self, name: String) -> FutureResult<bool> {
        tracing::trace!("checking existence of object: {name}");
        let store = self.store.clone();

        async move {
            let _ = store.get(name).await.context("checking object")?;
            Ok(true)
        }
        .boxed()
    }

    fn object_info(&self, name: String) -> FutureResult<ObjectMetadata> {
        tracing::trace!("getting object info: {name}");
        let store = self.store.clone();
        let metadata = self.metadata.clone();

        async move {
            let info = store.info(&name).await.context("getting object info")?;

            Ok(ObjectMetadata {
                container: metadata.name.clone(),
                name: info.name,
                size: info.size as u64,
                created_at: metadata.created_at,
            })
        }
        .boxed()
    }
}
