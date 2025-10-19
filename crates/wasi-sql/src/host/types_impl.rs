use std::sync::Arc;

use anyhow::{Result, anyhow};
use wasmtime::component::Resource;

use crate::host::generated::wasi::sql::types::{self, Connection, Error, Statement};
use crate::host::resource::{ClientProxy, ConnectionProxy};
use crate::host::{CLIENTS, Host};

impl ClientProxy {
    async fn try_from(_name: &str) -> anyhow::Result<Self> {
        let clients = CLIENTS.lock().await;
        let Some((_, client)) = clients.iter().next() else {
            return Err(anyhow!("no client registered"));
        };
        Ok(Self(Arc::clone(client)))
    }
}

impl types::Host for Host<'_> {
    fn convert_error(&mut self, err: anyhow::Error) -> Result<Error> {
        Ok(err)
    }
}

impl types::HostConnection for Host<'_> {
    async fn open(
        &mut self, name: String,
    ) -> Result<Result<Resource<Connection>, Resource<Error>>> {
        let proxy = ClientProxy::try_from(&name).await?;
        match proxy.open(name).await {
            Ok(conn) => Ok(Ok(self.table.push(ConnectionProxy(conn))?)),
            Err(err) => Ok(Err(self.table.push(err)?)),
        }
    }

    async fn drop(&mut self, rep: Resource<Connection>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}

impl types::HostStatement for Host<'_> {
    async fn prepare(
        &mut self, query: String, params: Vec<String>,
    ) -> Result<Result<Resource<Statement>, Resource<Error>>> {
        let statement = Statement { query, params };
        let res = self.table.push(statement)?;
        Ok(Ok(res))
    }

    async fn drop(&mut self, rep: Resource<Statement>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}

impl types::HostError for Host<'_> {
    async fn trace(&mut self, self_: Resource<Error>) -> Result<String> {
        let err = self.table.get(&self_)?;
        tracing::error!("Guest error: {err}",);
        Ok(err.to_string())
    }

    async fn drop(&mut self, rep: Resource<Error>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}
