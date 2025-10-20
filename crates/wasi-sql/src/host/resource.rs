use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

pub use crate::host::generated::wasi::sql::readwrite::Row;

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

/// SQL providers implement the [`Client`] trait to allow the host to
/// connect to a backend (Azure Table Storage, Postgres, etc) and open
/// containers.
pub trait Client: Debug + Send + Sync + 'static {
    /// The name of the backend this client is implemented for.
    fn name(&self) -> &'static str;

    /// Open a connection.
    fn open(&self, name: String) -> FutureResult<Arc<dyn Connection>>;
}

/// [`ClientProxy`] provides a concrete wrapper around a `dyn Connection` object.
/// It is used to store connection resources in the resource table.
#[derive(Clone, Debug)]
pub struct ClientProxy(pub Arc<dyn Client>);

impl Deref for ClientProxy {
    type Target = Arc<dyn Client>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// SQL providers implement the [`Connection`] trait to allow the host to
/// connect to a backend (Azure Table Storage, Postgres, etc) and execute SQL
/// statements.
pub trait Connection: Debug + Send + Sync + 'static {
    /// The name of the backend this client is implemented for.
    fn name(&self) -> &'static str;

    fn query(&self, query: String, params: Vec<String>) -> FutureResult<Vec<Row>>;

    fn exec(&self, query: String, params: Vec<String>) -> FutureResult<u32>;
}

/// [`ConnectionProxy`] provides a concrete wrapper around a `dyn Connection` object.
/// It is used to store connection resources in the resource table.
#[derive(Clone, Debug)]
pub struct ConnectionProxy(pub Arc<dyn Connection>);

impl Deref for ConnectionProxy {
    type Target = Arc<dyn Connection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents a statement resource in the WASI SQL host.
pub struct Statement {
    /// SQL query string.
    pub query: String,

    /// Query parameters.
    pub params: Vec<String>,
}
