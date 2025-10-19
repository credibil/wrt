use std::sync::Arc;

use anyhow::{Context, anyhow};
use wasmtime::component::Resource;

use crate::host::generated::wasi::vault::vault;
use crate::host::generated::wasi::vault::vault::Error;
use crate::host::resource::LockerProxy;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

use crate::host::resource::ClientProxy;
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

impl vault::Host for Host<'_> {
    // Open locker specified by identifier, save to state and return as a resource.
    async fn open(&mut self, locker_id: String) -> Result<Resource<LockerProxy>> {
        let client = ClientProxy::try_from("").await?;
        let locker = client.open(locker_id).await.context("opening locker")?;
        let proxy = LockerProxy(locker);
        Ok(self.table.push(proxy)?)
    }

    fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
        tracing::error!("{err}");
        Ok(err)
    }
}

impl vault::HostLocker for Host<'_> {
    async fn get(
        &mut self, self_: Resource<LockerProxy>, secret_id: String,
    ) -> Result<Option<Vec<u8>>> {
        let Ok(locker) = self.table.get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        let value = locker.get(secret_id).await.context("issue getting value")?;
        Ok(value)
    }

    async fn set(
        &mut self, self_: Resource<LockerProxy>, secret_id: String, value: Vec<u8>,
    ) -> Result<(), Error> {
        let Ok(locker) = self.table.get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        Ok(locker.set(secret_id, value).await.context("setting value")?)
    }

    async fn delete(&mut self, self_: Resource<LockerProxy>, secret_id: String) -> Result<()> {
        let Ok(locker) = self.table.get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        Ok(locker.delete(secret_id).await.context("deleting value")?)
    }

    async fn exists(&mut self, self_: Resource<LockerProxy>, secret_id: String) -> Result<bool> {
        vault::HostLocker::get(self, self_, secret_id).await.map(|opt| opt.is_some())
    }

    async fn list_ids(&mut self, self_: Resource<LockerProxy>) -> Result<Vec<String>> {
        let Ok(locker) = self.table.get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        let secret_ids = locker.list_ids().await.context("listing keys")?;
        Ok(secret_ids)
    }

    async fn drop(&mut self, rep: Resource<LockerProxy>) -> anyhow::Result<()> {
        tracing::trace!("vault::HostLocker::drop");
        self.table.delete(rep).map(|_| Ok(()))?
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
