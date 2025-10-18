use anyhow::{Result, anyhow};
use async_nats::jetstream;
use async_nats::jetstream::object_store::{Config, ObjectStore};
use wasmtime::component::Resource;

use crate::host::Host;
use crate::host::generated::wasi::blobstore::blobstore::{self, ObjectId};

pub type Container = ObjectStore;

fn nats() -> Result<&'static async_nats::Client> {
    todo!()
}

// Implement the [`wasi_sql::ReadWriteView`]` trait for Host<'_>.
impl blobstore::Host for Host<'_> {
    async fn create_container(&mut self, name: String) -> Result<Resource<Container>> {
        let jetstream = jetstream::new(nats()?.clone());
        let store = jetstream
            .create_object_store(Config {
                bucket: name,
                ..Config::default()
            })
            .await?;

        Ok(self.table.push(store)?)
    }

    async fn get_container(&mut self, name: String) -> Result<Resource<Container>> {
        let jetstream = jetstream::new(nats()?.clone());
        let store = jetstream
            .get_object_store(&name)
            .await
            .map_err(|e| anyhow!("issue getting object store: {e}"))?;

        Ok(self.table.push(store)?)
    }

    async fn delete_container(&mut self, name: String) -> Result<()> {
        let jetstream = jetstream::new(nats()?.clone());
        jetstream
            .delete_object_store(&name)
            .await
            .map_err(|e| anyhow!("issue deleting object store: {e}"))?;

        Ok(())
    }

    async fn container_exists(&mut self, name: String) -> Result<bool> {
        let jetstream = jetstream::new(nats()?.clone());
        let exists = jetstream.get_object_store(&name).await.is_ok();
        Ok(exists)
    }

    async fn copy_object(&mut self, _src: ObjectId, _dest: ObjectId) -> Result<()> {
        unimplemented!("copy_object not implemented yet")
    }

    async fn move_object(&mut self, _src: ObjectId, _dest: ObjectId) -> Result<()> {
        unimplemented!("move_object not implemented yet")
    }
}
