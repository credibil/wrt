use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use wasmtime::component::Resource;

use crate::host::generated::wasi::blobstore::blobstore::{self, ObjectId};
use crate::host::resource::{ClientProxy, ContainerProxy};
use crate::host::{CLIENTS, Host};

impl ClientProxy {
    async fn try_from(_name: &str) -> anyhow::Result<Self> {
        let clients = CLIENTS.lock().await;
        let Some((_, client)) = clients.iter().next() else {
            return Err(anyhow!("no client registered"))?;
        };
        Ok(Self(Arc::clone(client)))
    }
}

impl blobstore::Host for Host<'_> {
    async fn create_container(&mut self, name: String) -> Result<Resource<ContainerProxy>> {
        tracing::trace!("create_container: {name}");

        let client = ClientProxy::try_from("").await?;
        let container = client.create_container(name).await.context("creating container")?;
        let proxy = ContainerProxy(container);
        Ok(self.table.push(proxy)?)
    }

    async fn get_container(&mut self, name: String) -> Result<Resource<ContainerProxy>> {
        tracing::trace!("get_container: {name}");

        let client = ClientProxy::try_from("").await?;
        let container = client.get_container(name).await.context("getting container")?;
        let proxy = ContainerProxy(container);
        Ok(self.table.push(proxy)?)
    }

    async fn delete_container(&mut self, name: String) -> Result<()> {
        tracing::trace!("delete_container: {name}");

        let client = ClientProxy::try_from("").await?;
        client.delete_container(name).await.context("deleting container")
    }

    async fn container_exists(&mut self, name: String) -> Result<bool> {
        tracing::trace!("container_exists: {name}");

        let client = ClientProxy::try_from("").await?;
        client.container_exists(name).await.context("checking container exists")
    }

    async fn copy_object(&mut self, _src: ObjectId, _dest: ObjectId) -> Result<()> {
        unimplemented!("copy_object not implemented yet")
    }

    async fn move_object(&mut self, _src: ObjectId, _dest: ObjectId) -> Result<()> {
        unimplemented!("move_object not implemented yet")
    }
}
