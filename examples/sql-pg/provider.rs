use std::any::Any;
use std::error::Error;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use fromenv::FromEnv;
use futures::FutureExt;
use futures::future::BoxFuture;
use http::{Request, Response};
use http_body::Body;
use wasi_sql::readwrite;
use wasi_sql::types::{Connection, DataType, Row, Statement};

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>;
}

/// The `Config` trait is used by implementers to provide configuration from
/// WASI-guest to dependent crates.
pub trait Config: Send + Sync {
    /// Request configuration setting.
    fn get(&self, key: &str) -> impl Future<Output = Result<String>> + Send;
}

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

pub trait SqlDb: Send + Sync {
    fn query(
        &self, pool_name: String, query: String, params: Vec<DataType>,
    ) -> FutureResult<Vec<Row>>;
    fn exec(&self, pool_name: String, query: String, params: Vec<DataType>) -> FutureResult<u32>;
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConfigSettings {
    #[env(from = "ENVIRONMENT", default = "dev")]
    pub environment: String,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        // we panic here to ensure configuration is always loaded
        // i.e. guest should not start without proper configuration
        Self::from_env().finalize().expect("should load configuration")
    }
}

#[derive(Clone, Default)]
pub struct Provider {
    pub config: ConfigSettings,
}

impl Provider {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Implement the HttpRequest trait for Provider
impl HttpRequest for Provider {
    async fn fetch<T>(&self, request: Request<T>) -> Result<Response<Bytes>>
    where
        T: http_body::Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        tracing::debug!("request: {:?}", request.uri());
        wasi_http::handle(request).await
    }
}

/// Implement the Config trait for Provider
impl Config for Provider {
    async fn get(&self, key: &str) -> Result<String> {
        Ok(match key {
            "ENVIRONMENT" => &self.config.environment,
            _ => return Err(anyhow::anyhow!("unknown config key: {key}")),
        }
        .clone())
    }
}

/// Implement the SqlDb trait for Provider
impl SqlDb for Provider {
    fn query(
        &self, pool_name: String, query: String, params: Vec<DataType>,
    ) -> FutureResult<Vec<Row>> {
        async {
            let cnn = Connection::open(pool_name)
                .await
                .map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

            let stmt = Statement::prepare(query, params)
                .await
                .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

            let res =
                readwrite::query(&cnn, &stmt).await.map_err(|e| anyhow!("query failed: {e:?}"))?;

            Ok(res)
        }
        .boxed()
    }

    fn exec(
        &self, _pool_name: String, _query: String, _params: Vec<DataType>,
    ) -> FutureResult<u32> {
        todo!()
    }
}

pub trait GuestProvider: HttpRequest + Config + SqlDb + Send + Sync {}
impl<T> GuestProvider for T where T: HttpRequest + Config + SqlDb + Send + Sync {}
