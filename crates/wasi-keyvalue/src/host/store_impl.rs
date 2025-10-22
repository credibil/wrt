use std::sync::Arc;

use anyhow::{Context, anyhow};
use wasmtime::component::Resource;

use crate::WasiKeyValueView;
use crate::host::generated::wasi::keyvalue::store;
use crate::host::generated::wasi::keyvalue::store::{Error, KeyResponse};
use crate::host::resource::{BucketProxy, ClientProxy,Client};
use crate::host::{CLIENTS, Result, WasiKeyValueImpl};

// impl ClientProxy {
//     async fn try_from(_name: &str) -> anyhow::Result<Self> {
//         let clients = CLIENTS.lock().await;
//         let Some((_, client)) = clients.iter().next() else {
//             return Err(anyhow!("no client registered"));
//         };
//         Ok(Self(Arc::clone(client)))
//     }
// }

impl<T: WasiKeyValueView> store::Host for WasiKeyValueImpl<T> {
    // Open bucket specified by identifier, save to state and return as a resource.
    async fn open(&mut self, identifier: String) -> Result<Resource<BucketProxy>> {
        tracing::trace!("store::Host::open: identifier {identifier:?}");
        // let client = ClientProxy::try_from("").await?;
        let client = self.client();
        let bucket = client.open(identifier).await.context("failed to open bucket")?;
        let proxy = BucketProxy(bucket);
        Ok(self.table().push(proxy)?)
    }

    fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
        tracing::error!("{err}");
        Ok(err)
    }
}

impl<T: WasiKeyValueView> store::HostBucket for WasiKeyValueImpl<T> {
    async fn get(&mut self, self_: Resource<BucketProxy>, key: String) -> Result<Option<Vec<u8>>> {
        let Ok(bucket) = self.table().get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        let value = bucket.get(key).await.context("issue getting value")?;
        Ok(value)
    }

    async fn set(
        &mut self, self_: Resource<BucketProxy>, key: String, value: Vec<u8>,
    ) -> Result<(), Error> {
        let Ok(bucket) = self.table().get_mut(&self_) else {
            return Err(Error::NoSuchStore);
        };
        Ok(bucket.set(key, value).await.context("setting value")?)
    }

    async fn delete(&mut self, self_: Resource<BucketProxy>, key: String) -> Result<()> {
        let Ok(bucket) = self.table().get_mut(&self_) else {
            return Err(Error::NoSuchStore);
        };
        Ok(bucket.delete(key).await.context("deleting value")?)
    }

    async fn exists(&mut self, self_: Resource<BucketProxy>, key: String) -> Result<bool> {
        let Ok(bucket) = self.table().get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        let value = bucket.get(key).await.context("checking whether entry exists")?;
        Ok(value.is_some())
    }

    async fn list_keys(
        &mut self, self_: Resource<BucketProxy>, cursor: Option<String>,
    ) -> Result<KeyResponse> {
        tracing::trace!("store::HostBucket::list_keys {cursor:?}");

        let Ok(bucket) = self.table().get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        let keys = bucket.keys().await.context("listing keys")?;

        Ok(KeyResponse { keys, cursor })
    }

    async fn drop(&mut self, rep: Resource<BucketProxy>) -> anyhow::Result<()> {
        self.table().delete(rep).map(|_| Ok(()))?
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
