use anyhow::{Context, Result};
use wasmtime::component::Resource;

use crate::host::generated::wasi::sql::readwrite;
use crate::host::generated::wasi::sql::readwrite::{Connection, Error, Row, Statement};

use crate::host::Host;

impl readwrite::Host for Host<'_> {
    async fn query(
        &mut self, c: Resource<Connection>, q: Resource<Statement>,
    ) -> Result<Result<Vec<Row>, Resource<Error>>> {
        let connection = self.table.get(&c).context("getting connection")?;
        let statement = self.table.get(&q).context("getting statement")?;

        // get statement from resource table
        let (query, params) = (statement.query.clone(), statement.params.clone());

        // execute query
        match connection.query(query, params).await.context("executing query") {
            Ok(rows) => Ok(Ok(rows)),
            Err(err) => Ok(Err(self.table.push(err)?)),
        }
    }

    async fn exec(
        &mut self, c: Resource<Connection>, q: Resource<Statement>,
    ) -> Result<Result<u32, Resource<Error>>> {
        let connection = self.table.get(&c).context("getting connection")?;
        let statement = self.table.get(&q).context("getting statement")?;

        // get statement from resource table
        let (query, params) = (statement.query.clone(), statement.params.clone());

        // execute query
        match connection.exec(query, params).await.context("executing query") {
            Ok(rows) => Ok(Ok(rows)),
            Err(err) => Ok(Err(self.table.push(err)?)),
        }
    }
}
