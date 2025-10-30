use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use bson::{Bson, Document, doc};
use chrono::Utc;
use futures::{FutureExt, StreamExt};
use mongodb::{Collection, bson};
use serde::{Deserialize, Serialize};
use wasi_blobstore_next::{
    Container, ContainerMetadata, FutureResult, ObjectMetadata, WasiBlobstoreCtx,
};

use crate::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    name: String,
    data: Document,
    size: u64,
    created_at: u64,
}

impl WasiBlobstoreCtx for Client {
    fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        tracing::trace!("creating container: {name}");
        let client = self.0.clone();

        async move {
            let db = client.default_database().ok_or_else(|| anyhow!("No default database"))?;
            let collection = db.collection::<Blob>(&name);
            Ok(Arc::new(MongoDbContainer { name, collection }) as Arc<dyn Container>)
        }
        .boxed()
    }

    fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        tracing::trace!("getting container: {name}");
        let client = self.0.clone();

        async move {
            let db = client.default_database().ok_or_else(|| anyhow!("No default database"))?;
            let collection = db.collection::<Blob>(&name);
            Ok(Arc::new(MongoDbContainer { name, collection }) as Arc<dyn Container>)
        }
        .boxed()
    }

    fn delete_container(&self, name: String) -> FutureResult<()> {
        tracing::trace!("deleting container: {name}");
        let client = self.0.clone();

        async move {
            let db = client.default_database().ok_or_else(|| anyhow!("No default database"))?;
            db.collection::<Blob>(&name).drop().await.context("deleting container")
        }
        .boxed()
    }

    fn container_exists(&self, name: String) -> FutureResult<bool> {
        tracing::trace!("checking existence of container: {name}");
        async move { Ok(true) }.boxed()
    }
}

#[derive(Debug)]
pub struct MongoDbContainer {
    name: String,
    collection: Collection<Blob>,
}

impl Container for MongoDbContainer {
    fn name(&self) -> FutureResult<String> {
        tracing::trace!("getting container name");
        let name = self.name.clone();
        async move { Ok(name) }.boxed()
    }

    fn info(&self) -> FutureResult<ContainerMetadata> {
        tracing::trace!("getting container info");
        let metadata = ContainerMetadata {
            name: self.name.clone(),
            created_at: 0,
        };
        async move { Ok(metadata) }.boxed()
    }

    /// Get the value associated with the key.
    fn get_data(&self, name: String, _start: u64, _end: u64) -> FutureResult<Option<Vec<u8>>> {
        tracing::trace!("getting object data: {name}");
        let collection = self.collection.clone();

        async move {
            // let mut object = collection.get(name).await.context("getting object")?;
            // let mut bytes = vec![];
            // object.read_to_end(&mut bytes).await?;

            let Some(blob) = collection.find_one(doc! { "name": name }).await? else {
                return Err(anyhow!("Object not found"));
            };

            // HACK: blob data is a string not an object
            let data = match blob.data.get("_string") {
                Some(Bson::String(s)) => {
                    serde_json::to_vec(&s).context("deserializing Document")?
                }
                _ => serde_json::to_vec(&blob.data).context("deserializing Document")?,
            };
            Ok(Some(data))
        }
        .boxed()
    }

    /// Set the value associated with the key.
    fn write_data(&self, name: String, data: Vec<u8>) -> FutureResult<()> {
        tracing::trace!("writing object data: {name}");
        let collection = self.collection.clone();

        async move {
            // `put` should update any previous value, so delete first
            collection.delete_one(doc! { "name": &name }).await?;

            let bytes = data;

            let data = match bytes.first() {
                Some(b) if *b == b'{' => serde_json::from_slice::<Document>(&bytes)
                    .context("deserializing into Document")?,
                Some(_) => {
                    // HACK: blob data is a string not an object
                    let stringified = serde_json::from_slice::<String>(&bytes)
                        .context("deserializing into String")?;
                    doc! {"_string": stringified}
                }
                None => return Err(anyhow!("OutgoingValue is empty")),
            };

            let blob = Blob {
                name,
                data,
                size: bytes.len() as u64,
                #[allow(clippy::cast_sign_loss)]
                created_at: Utc::now().timestamp_millis() as u64,
            };
            collection.insert_one(blob).await?;

            Ok(())
        }
        .boxed()
    }

    /// List all objects in the container.
    fn list_objects(&self) -> FutureResult<Vec<String>> {
        tracing::trace!("listing objects");
        let collection = self.collection.clone();

        async move {
            let mut list = collection.find(doc! {}).await?;
            let mut names = vec![];

            while let Some(n) = list.next().await {
                match n {
                    Ok(blob) => names.push(blob.name),
                    Err(e) => tracing::warn!("issue listing object: {e}"),
                }
            }

            Ok(names)
        }
        .boxed()
    }

    /// Delete the value associated with the key.
    fn delete_object(&self, name: String) -> FutureResult<()> {
        let collection = self.collection.clone();

        async move {
            collection.delete_one(doc! { "name": name }).await.context("deleting object")?;
            Ok(())
        }
        .boxed()
    }

    /// Check if the object exists.
    fn has_object(&self, name: String) -> FutureResult<bool> {
        tracing::trace!("checking existence of object: {name}");
        let collection = self.collection.clone();

        async move { Ok(collection.find_one(doc! { "name": name }).await?.is_some()) }.boxed()
    }

    fn object_info(&self, name: String) -> FutureResult<ObjectMetadata> {
        tracing::trace!("getting object info: {name}");
        let collection = self.collection.clone();

        async move {
            let Some(blob) = collection.find_one(doc! { "name": name }).await? else {
                return Err(anyhow!("Object not found"));
            };

            Ok(ObjectMetadata {
                name: blob.name,
                container: collection.name().to_string(),
                size: blob.size,
                created_at: blob.created_at,
            })
        }
        .boxed()
    }
}
