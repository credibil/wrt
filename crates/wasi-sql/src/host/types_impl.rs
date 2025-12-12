use anyhow::Result;
use wasmtime::component::{Access, Accessor, Resource};

use crate::host::generated::wasi::sql::types::{
    Connection, DataType, Error, Host, HostConnection, HostConnectionWithStore, HostError,
    HostErrorWithStore, HostStatement, HostStatementWithStore, Statement,
};
use crate::host::resource::ConnectionProxy;
use crate::host::{WasiSql, WasiSqlCtxView};

impl HostConnectionWithStore for WasiSql {
    async fn open<T>(
        accessor: &Accessor<T, Self>, name: String,
    ) -> Result<Result<Resource<Connection>, Resource<Error>>> {
        let open_conn = accessor.with(|mut store| store.get().ctx.open(name)).await;

        let result = match open_conn {
            Ok(conn) => {
                let proxy = ConnectionProxy(conn);
                Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
            }
            Err(err) => Err(accessor.with(|mut store| store.get().table.push(err))?),
        };

        Ok(result)
    }

    fn drop<T>(
        mut accessor: Access<'_, T, Self>, rep: Resource<ConnectionProxy>,
    ) -> anyhow::Result<()> {
        accessor.get().table.delete(rep).map(|_| Ok(()))?
    }
}

impl HostStatementWithStore for WasiSql {
    async fn prepare<T>(
        accessor: &Accessor<T, Self>, query: String, params: Vec<DataType>,
    ) -> Result<Result<Resource<Statement>, Resource<Error>>> {
        let statement = Statement { query, params };
        Ok(Ok(accessor.with(|mut store| store.get().table.push(statement))?))
    }

    fn drop<T>(mut accessor: Access<'_, T, Self>, rep: Resource<Statement>) -> anyhow::Result<()> {
        Ok(accessor.get().table.delete(rep).map(|_| ())?)
    }
}

impl HostErrorWithStore for WasiSql {
    fn trace<T>(mut host: Access<'_, T, Self>, self_: Resource<Error>) -> Result<String> {
        let err = host.get().table.get(&self_)?;
        Ok(err.to_string())
    }

    fn drop<T>(mut accessor: Access<'_, T, Self>, rep: Resource<Error>) -> anyhow::Result<()> {
        Ok(accessor.get().table.delete(rep).map(|_| ())?)
    }
}

impl Host for WasiSqlCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> Result<Error, anyhow::Error> {
        Ok(err)
    }
}

impl HostConnection for WasiSqlCtxView<'_> {}
impl HostStatement for WasiSqlCtxView<'_> {}
impl HostError for WasiSqlCtxView<'_> {}
