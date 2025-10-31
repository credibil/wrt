use anyhow::Result;
use wasmtime::component::{Accessor, Resource};

use crate::host::generated::wasi::blobstore::blobstore::{Host, HostWithStore, ObjectId};
use crate::host::resource::ContainerProxy;
use crate::host::{WasiBlobstore, WasiBlobstoreCtxView};

impl HostWithStore for WasiBlobstore {
    async fn create_container<T>(
        accessor: &Accessor<T, Self>, name: String,
    ) -> Result<Resource<ContainerProxy>> {
        tracing::trace!("create_container: {name}");
        let container = accessor.with(|mut store| store.get().ctx.create_container(name)).await?;
        let proxy = ContainerProxy(container);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }

    async fn get_container<T>(
        accessor: &Accessor<T, Self>, name: String,
    ) -> Result<Resource<ContainerProxy>> {
        tracing::trace!("get_container: {name}");
        let container = accessor.with(|mut store| store.get().ctx.get_container(name)).await?;
        let proxy = ContainerProxy(container);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }

    async fn delete_container<T>(accessor: &Accessor<T, Self>, name: String) -> Result<()> {
        tracing::trace!("delete_container: {name}");
        accessor.with(|mut store| store.get().ctx.delete_container(name)).await
    }

    async fn container_exists<T>(accessor: &Accessor<T, Self>, name: String) -> Result<bool> {
        tracing::trace!("container_exists: {name}");
        accessor.with(|mut store| store.get().ctx.container_exists(name)).await
    }

    async fn copy_object<T>(_: &Accessor<T, Self>, _src: ObjectId, _dest: ObjectId) -> Result<()> {
        unimplemented!("copy_object not implemented yet")
    }

    async fn move_object<T>(_: &Accessor<T, Self>, _src: ObjectId, _dest: ObjectId) -> Result<()> {
        unimplemented!("move_object not implemented yet")
    }
}

impl Host for WasiBlobstoreCtxView<'_> {}
