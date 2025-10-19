use anyhow::{Context, Result, anyhow};
use bytes::{Bytes, BytesMut};
use wasmtime::component::Resource;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;

use crate::host::Host;
use crate::host::generated::wasi::blobstore::container::{self, ContainerMetadata, ObjectMetadata};
use crate::host::resource::ContainerProxy;

pub type IncomingValue = Bytes;
pub type OutgoingValue = MemoryOutputPipe;
pub type StreamObjectNames = Vec<String>;

impl container::Host for Host<'_> {}

impl container::HostContainer for Host<'_> {
    async fn name(&mut self, self_: Resource<ContainerProxy>) -> Result<String> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        container.name().await.context("getting name")
    }

    async fn info(&mut self, self_: Resource<ContainerProxy>) -> Result<ContainerMetadata> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        container.info().await.context("getting info")
    }

    async fn get_data(
        &mut self, self_: Resource<ContainerProxy>, name: String, start: u64, end: u64,
    ) -> Result<Resource<IncomingValue>> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };

        let data_opt = container.get_data(name, start, end).await.context("getting data")?;
        let Some(data) = data_opt else {
            return Err(anyhow!("object not found"));
        };
        let buf = BytesMut::from(&*data);

        Ok(self.table.push(buf.into())?)
    }

    async fn write_data(
        &mut self, self_: Resource<ContainerProxy>, name: String, data: Resource<OutgoingValue>,
    ) -> Result<()> {
        let Ok(value) = self.table.get(&data) else {
            return Err(anyhow!("OutgoingValue not found"));
        };
        let bytes = value.contents();

        let Ok(container) = self.table.get_mut(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        container.write_data(name, bytes.to_vec()).await.context("writing data")?;

        Ok(())
    }

    async fn list_objects(
        &mut self, self_: Resource<ContainerProxy>,
    ) -> Result<Resource<StreamObjectNames>> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };

        let names = container.list_objects().await.context("listing objects")?;
        Ok(self.table.push(names)?)
    }

    async fn delete_object(&mut self, self_: Resource<ContainerProxy>, name: String) -> Result<()> {
        let Ok(container) = self.table.get_mut(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        container.delete_object(name).await.context("deleting object")
    }

    async fn delete_objects(
        &mut self, self_: Resource<ContainerProxy>, names: Vec<String>,
    ) -> Result<()> {
        let Ok(container) = self.table.get_mut(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        for name in names {
            container.delete_object(name).await.context("deleting object")?;
        }

        Ok(())
    }

    async fn has_object(&mut self, self_: Resource<ContainerProxy>, name: String) -> Result<bool> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        container.has_object(name).await.context("checking object exists")
    }

    async fn object_info(
        &mut self, self_: Resource<ContainerProxy>, name: String,
    ) -> Result<ObjectMetadata> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };
        container.object_info(name).await.context("getting object info")
    }

    async fn clear(&mut self, self_: Resource<ContainerProxy>) -> Result<()> {
        let Ok(container) = self.table.get(&self_) else {
            return Err(anyhow!("ContainerProxy not found"));
        };

        let all_objects = container.list_objects().await.context("listing objects")?;
        for name in all_objects {
            container.delete_object(name).await.context("deleting object")?;
        }

        Ok(())
    }

    async fn drop(&mut self, rep: Resource<ContainerProxy>) -> Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl container::HostStreamObjectNames for Host<'_> {
    async fn read_stream_object_names(
        &mut self, _self_: Resource<StreamObjectNames>, _len: u64,
    ) -> Result<(Vec<String>, bool)> {
        unimplemented!()
    }

    async fn skip_stream_object_names(
        &mut self, _names_ref: Resource<StreamObjectNames>, _num: u64,
    ) -> Result<(u64, bool)> {
        unimplemented!()
    }

    async fn drop(&mut self, rep: Resource<StreamObjectNames>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}
