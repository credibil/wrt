use anyhow::{Context, Result, anyhow};
use bytes::{Bytes, BytesMut};
use wasmtime::component::{Accessor, Resource};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;

use crate::host::generated::wasi::blobstore::container::{
    ContainerMetadata, Host, HostContainer, HostContainerWithStore, HostStreamObjectNames,
    HostStreamObjectNamesWithStore, ObjectMetadata,
};
use crate::host::resource::ContainerProxy;
use crate::host::{WasiBlobstore, WasiBlobstoreCtxView};

pub type IncomingValue = Bytes;
pub type OutgoingValue = MemoryOutputPipe;
pub type StreamObjectNames = Vec<String>;

// impl container::Host for Host<'_> {}

impl HostContainerWithStore for WasiBlobstore {
    async fn name<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>,
    ) -> Result<String> {
        let container = get_container(accessor, &self_)?;
        container.name().await.context("getting name")
    }

    async fn info<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>,
    ) -> Result<ContainerMetadata> {
        let container = get_container(accessor, &self_)?;
        container.info().await.context("getting info")
    }

    async fn get_data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>, name: String, start: u64,
        end: u64,
    ) -> Result<Resource<IncomingValue>> {
        let container = get_container(accessor, &self_)?;

        let data_opt = container.get_data(name, start, end).await.context("getting data")?;
        let Some(data) = data_opt else {
            return Err(anyhow!("object not found"));
        };
        let buf = BytesMut::from(&*data);

        Ok(accessor.with(|mut store| store.get().table.push(buf.into()))?)
    }

    async fn write_data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>, name: String,
        data: Resource<OutgoingValue>,
    ) -> Result<()> {
        let bytes = accessor.with(|mut store| {
            let value = store.get().table.get(&data)?;
            Ok::<Vec<u8>, anyhow::Error>(value.contents().to_vec())
        })?;

        let container = get_container(accessor, &self_)?;
        container.write_data(name, bytes).await.context("writing data")?;

        Ok(())
    }

    async fn list_objects<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>,
    ) -> Result<Resource<StreamObjectNames>> {
        let container = get_container(accessor, &self_)?;
        let names = container.list_objects().await.context("listing objects")?;
        Ok(accessor.with(|mut store| store.get().table.push(names))?)
    }

    async fn delete_object<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>, name: String,
    ) -> Result<()> {
        let container = get_container(accessor, &self_)?;
        container.delete_object(name).await.context("deleting object")
    }

    async fn delete_objects<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>, names: Vec<String>,
    ) -> Result<()> {
        let container = get_container(accessor, &self_)?;
        for name in names {
            container.delete_object(name).await.context("deleting object")?;
        }

        Ok(())
    }

    async fn has_object<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>, name: String,
    ) -> Result<bool> {
        let container = get_container(accessor, &self_)?;
        container.has_object(name).await.context("checking object exists")
    }

    async fn object_info<T>(
        accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>, name: String,
    ) -> Result<ObjectMetadata> {
        let container = get_container(accessor, &self_)?;
        container.object_info(name).await.context("getting object info")
    }

    async fn clear<T>(accessor: &Accessor<T, Self>, self_: Resource<ContainerProxy>) -> Result<()> {
        let container = get_container(accessor, &self_)?;

        let all_objects = container.list_objects().await.context("listing objects")?;
        for name in all_objects {
            container.delete_object(name).await.context("deleting object")?;
        }

        Ok(())
    }

    async fn drop<T>(accessor: &Accessor<T, Self>, rep: Resource<ContainerProxy>) -> Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl HostStreamObjectNamesWithStore for WasiBlobstore {
    async fn read_stream_object_names<T>(
        _: &Accessor<T, Self>, _self_: Resource<StreamObjectNames>, _len: u64,
    ) -> Result<(Vec<String>, bool)> {
        unimplemented!()
    }

    async fn skip_stream_object_names<T>(
        _: &Accessor<T, Self>, _names_ref: Resource<StreamObjectNames>, _num: u64,
    ) -> Result<(u64, bool)> {
        unimplemented!()
    }

    async fn drop<T>(accessor: &Accessor<T, Self>, rep: Resource<StreamObjectNames>) -> Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl Host for WasiBlobstoreCtxView<'_> {}
impl HostContainer for WasiBlobstoreCtxView<'_> {}
impl HostStreamObjectNames for WasiBlobstoreCtxView<'_> {}

pub fn get_container<T>(
    accessor: &Accessor<T, WasiBlobstore>, self_: &Resource<ContainerProxy>,
) -> Result<ContainerProxy> {
    accessor.with(|mut store| {
        let container =
            store.get().table.get(self_).context("Container not found")?;
        Ok(container.clone())
    })
}
