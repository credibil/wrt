use anyhow::{Result, anyhow};
use async_nats::jetstream::object_store::ObjectStore;
use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;
use wasmtime::component::Resource;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;

use crate::host::Host;
use crate::host::generated::wasi::blobstore::container::{self, ContainerMetadata, ObjectMetadata};

pub type Container = ObjectStore;
pub type IncomingValue = Bytes;
pub type OutgoingValue = MemoryOutputPipe;
pub type StreamObjectNames = Vec<String>;

impl container::Host for Host<'_> {}

impl container::HostContainer for Host<'_> {
    async fn name(&mut self, self_: Resource<Container>) -> Result<String> {
        let Ok(store) = self.table.get(&self_) else {
            return Err(anyhow!("Container not found"));
        };

        // HACK: get the store name from the first object in the store
        // TODO: wrap NATS ObjectStore with a custom type
        let mut list = store.list().await.map_err(|e| anyhow!("issue listing objects: {e}"))?;
        let Some(Ok(n)) = list.next().await else {
            return Err(anyhow!("No objects found in the store"));
        };

        Ok(n.bucket)
    }

    async fn info(&mut self, _self_: Resource<Container>) -> Result<ContainerMetadata> {
        todo!()
    }

    async fn get_data(
        &mut self, self_: Resource<Container>, name: String, _start: u64, _end: u64,
    ) -> Result<Resource<IncomingValue>> {
        let Ok(store) = self.table.get(&self_) else {
            return Err(anyhow!("Container not found"));
        };

        // read the object data from the store
        let mut data = store.get(&name).await.map_err(|e| anyhow!("issue getting object: {e}"))?;
        let mut buf = BytesMut::new();
        data.read_buf(&mut buf).await?;

        Ok(self.table.push(buf.into())?)
    }

    async fn write_data(
        &mut self, self_: Resource<Container>, name: String, data: Resource<OutgoingValue>,
    ) -> Result<()> {
        let Ok(value) = self.table.get(&data) else {
            return Err(anyhow!("OutgoingValue not found"));
        };
        let bytes = value.contents();

        let Ok(store) = self.table.get_mut(&self_) else {
            return Err(anyhow!("Container not found"));
        };

        // write the data to the store
        store
            .put(name.as_str(), &mut bytes.to_vec().as_slice())
            .await
            .map_err(|e| anyhow!("issue writing object: {e}"))?;

        Ok(())
    }

    async fn list_objects(
        &mut self, self_: Resource<Container>,
    ) -> Result<Resource<StreamObjectNames>> {
        let Ok(store) = self.table.get(&self_) else {
            return Err(anyhow!("Container not found"));
        };
        let mut list = store.list().await.map_err(|e| anyhow!("issue listing objects: {e}"))?;

        let mut names = vec![];
        while let Some(n) = list.next().await {
            match n {
                Ok(obj_info) => names.push(obj_info.name),
                Err(e) => tracing::warn!("issue listing object: {e}"),
            }
        }

        Ok(self.table.push(names)?)
    }

    async fn delete_object(&mut self, self_: Resource<Container>, name: String) -> Result<()> {
        let Ok(store) = self.table.get_mut(&self_) else {
            return Err(anyhow!("Container not found"));
        };
        store.delete(&name).await.map_err(|e| anyhow!("issue deleting: {e}"))?;

        Ok(())
    }

    async fn delete_objects(
        &mut self, self_: Resource<Container>, names: Vec<String>,
    ) -> Result<()> {
        let Ok(store) = self.table.get_mut(&self_) else {
            return Err(anyhow!("Container not found"));
        };
        for name in names {
            store.delete(&name).await.map_err(|e| anyhow!("issue deleting '{name}': {e}"))?;
        }

        Ok(())
    }

    async fn has_object(&mut self, self_: Resource<Container>, name: String) -> Result<bool> {
        let Ok(store) = self.table.get(&self_) else {
            return Err(anyhow!("Container not found"));
        };
        Ok(store.info(&name).await.is_ok())
    }

    async fn object_info(
        &mut self, self_: Resource<Container>, name: String,
    ) -> Result<ObjectMetadata> {
        let Ok(store) = self.table.get(&self_) else {
            return Err(anyhow!("Container not found"));
        };
        let info = store.info(&name).await?;

        #[allow(clippy::cast_sign_loss)]
        let metadata = ObjectMetadata {
            name: info.name,
            container: name,
            size: info.size as u64,
            created_at: info.modified.unwrap_or(OffsetDateTime::UNIX_EPOCH).unix_timestamp() as u64,
        };
        Ok(metadata)
    }

    async fn clear(&mut self, self_: Resource<Container>) -> Result<()> {
        let Ok(store) = self.table.get(&self_) else {
            return Err(anyhow!("Container not found"));
        };
        let mut list = store.list().await.map_err(|e| anyhow!("issue listing objects: {e}"))?;

        while let Some(n) = list.next().await {
            match n {
                Ok(obj_info) => store.delete(obj_info.name).await?,
                Err(e) => tracing::warn!("issue listing object: {e}"),
            }
        }

        Ok(())
    }

    async fn drop(&mut self, rep: Resource<Container>) -> Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl container::HostStreamObjectNames for Host<'_> {
    async fn read_stream_object_names(
        &mut self, _self_: Resource<StreamObjectNames>, _len: u64,
    ) -> Result<(Vec<String>, bool)> {
        todo!()
    }

    async fn skip_stream_object_names(
        &mut self, _names_ref: Resource<StreamObjectNames>, _num: u64,
    ) -> Result<(u64, bool)> {
        todo!()
    }

    async fn drop(&mut self, rep: Resource<StreamObjectNames>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}
