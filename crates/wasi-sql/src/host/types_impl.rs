use anyhow::Result;
use wasmtime::component::Resource;

use crate::host::Host;
use crate::host::generated::wasi::sql::types::{self, Connection, Error, Statement};

impl types::Host for Host<'_> {
    fn convert_error(&mut self, err: anyhow::Error) -> Result<Error> {
        tracing::error!("{err}");
        // Ok(Error::from_string(err.to_string()))
        todo!()
    }
}

impl types::HostConnection for Host<'_> {
    async fn open(
        &mut self, name: String,
    ) -> Result<Result<Resource<Connection>, Resource<Error>>> {
        todo!()
    }

    async fn drop(&mut self, rep: Resource<Connection>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}

impl types::HostStatement for Host<'_> {
    async fn prepare(
        &mut self, query: String, params: Vec<String>,
    ) -> Result<Result<Resource<Statement>, Resource<Error>>> {
        todo!()
    }

    async fn drop(&mut self, rep: Resource<Statement>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}

impl types::HostError for Host<'_> {
    async fn trace(&mut self, rep: Resource<Error>) -> Result<String> {
        todo!()
    }

    async fn drop(&mut self, rep: Resource<Error>) -> Result<()> {
        Ok(self.table.delete(rep).map(|_| ())?)
    }
}
