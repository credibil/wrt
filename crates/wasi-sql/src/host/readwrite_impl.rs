use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use wasmtime::component::Resource;

use crate::host::generated::wasi::sql::readwrite;
use crate::host::generated::wasi::sql::readwrite::{Connection, Error, Row, Statement};
use crate::host::resource::LockerProxy;

// pub type Result<T, E = Error> = anyhow::Result<T, E>;

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

impl readwrite::Host for Host<'_> {
    async fn query(
        &mut self, c: Resource<Connection>, q: Resource<Statement>,
    ) -> Result<Result<Vec<Row>, Resource<Error>>> {
        let client = ClientProxy::try_from("").await?;
        // let locker = client.open(locker_id).await.context("opening locker")?;
        // let proxy = LockerProxy(locker);
        // Ok(self.table.push(proxy)?)

        todo!()
    }

    async fn exec(
        &mut self, c: Resource<Connection>, q: Resource<Statement>,
    ) -> Result<Result<u32, Resource<Error>>> {
        let client = ClientProxy::try_from("").await?;
        // let locker = client.open(locker_id).await.context("opening locker")?;
        // let proxy = LockerProxy(locker);
        // Ok(self.table.push(proxy)?)

        todo!()
    }

    // fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
    //     tracing::error!("{err}");
    //     Ok(err)
    // }
}

// impl readwrite::HostLocker for Host<'_> {
//     async fn get(
//         &mut self, self_: Resource<LockerProxy>, secret_id: String,
//     ) -> Result<Option<Vec<u8>>> {
//         let Ok(locker) = self.table.get(&self_) else {
//             return Err(Error::NoSuchStore);
//         };
//         let value = locker.get(secret_id).await.context("issue getting value")?;
//         Ok(value)
//     }

//     async fn set(
//         &mut self, self_: Resource<LockerProxy>, secret_id: String, value: Vec<u8>,
//     ) -> Result<(), Error> {
//         let Ok(locker) = self.table.get(&self_) else {
//             return Err(Error::NoSuchStore);
//         };
//         Ok(locker.set(secret_id, value).await.context("setting value")?)
//     }

//     async fn delete(&mut self, self_: Resource<LockerProxy>, secret_id: String) -> Result<()> {
//         let Ok(locker) = self.table.get(&self_) else {
//             return Err(Error::NoSuchStore);
//         };
//         Ok(locker.delete(secret_id).await.context("deleting value")?)
//     }

//     async fn exists(&mut self, self_: Resource<LockerProxy>, secret_id: String) -> Result<bool> {
//         readwrite::HostLocker::get(self, self_, secret_id).await.map(|opt| opt.is_some())
//     }

//     async fn list_ids(&mut self, self_: Resource<LockerProxy>) -> Result<Vec<String>> {
//         let Ok(locker) = self.table.get(&self_) else {
//             return Err(Error::NoSuchStore);
//         };
//         let secret_ids = locker.list_ids().await.context("listing keys")?;
//         Ok(secret_ids)
//     }

//     async fn drop(&mut self, rep: Resource<LockerProxy>) -> anyhow::Result<()> {
//         tracing::trace!("readwrite::HostLocker::drop");
//         self.table.delete(rep).map(|_| Ok(()))?
//     }
// }

// impl From<anyhow::Error> for Error {
//     fn from(err: anyhow::Error) -> Self {
//         Self::Other(err.to_string())
//     }
// }
