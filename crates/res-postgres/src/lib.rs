#![cfg(not(target_arch = "wasm32"))]

//! Postgres client builder for runtime.
//!
//! TODO: This attempt uses an enum in the wit for data types as a way to map
//! a few of the many Postgres types to Rust types. Investigate if using any low
//! level tools inside an ORM crate like Diesel would get us better type
//! coverage with less effort.

mod sql;

use std::str;

use anyhow::{Context as _, Result, anyhow};
use deadpool_postgres::{Config, Pool, PoolConfig, Runtime};
use fromenv::FromEnv;
use runtime::Resource;
use rustls::crypto::ring;
use rustls::{ClientConfig, RootCertStore};
use tokio_postgres::config::{Host, SslMode};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::instrument;
use webpki_roots::TLS_SERVER_ROOTS;

/// Postgres client
#[derive(Clone, Debug)]
pub struct Client(Pool);

/// Postgres resource builder
impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    /// Connect to `PostgreSQL` with provided options and return a connection pool
    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let pool_config = Config::try_from(&options)?;
        let runtime = Some(Runtime::Tokio1);

        if pool_config.ssl_mode.is_none() {
            // Non-TLS mode
            let pool = pool_config
                .create_pool(runtime, tokio_postgres::NoTls)
                .context("failed to create postgres pool")?;
            tracing::info!("connected to Postgres without TLS");
            return Ok(Self(pool));
        }

        // TLS mode
        ring::default_provider()
            .install_default()
            .map_err(|e| anyhow!("Failed to install rustls crypto provider: {e:?}"))?;

        let mut store = RootCertStore::empty();
        store.extend(TLS_SERVER_ROOTS.iter().cloned());

        let client_config =
            ClientConfig::builder().with_root_certificates(store).with_no_client_auth();
        let pool = pool_config
            .create_pool(runtime, MakeRustlsConnect::new(client_config))
            .context("failed to create postgres pool")?;

        // Check pool is usable
        if pool.get().await.is_err() {
            return Err(anyhow!("failed to get connection from pool"));
        }

        tracing::info!("connected to Postgres");

        Ok(Self(pool))
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "POSTGRES_URL")]
    pub uri: String,
    #[env(from = "POSTGRES_POOL_SIZE", default = "10")]
    pub pool_size: usize,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

impl TryFrom<&ConnectOptions> for deadpool_postgres::Config {
    type Error = anyhow::Error;

    fn try_from(options: &ConnectOptions) -> Result<Self> {
        // parse postgres uri
        let tokio: tokio_postgres::Config = options.uri.parse().context("parsing Postgres URI")?;
        let host = tokio
            .get_hosts()
            .first()
            .map(|h| match h {
                Host::Tcp(name) => name.to_owned(),
                Host::Unix(path) => path.to_string_lossy().to_string(),
            })
            .unwrap_or_default();
        let port = tokio.get_ports().first().copied().ok_or_else(|| anyhow!("Port is missing"))?;
        let username = tokio.get_user().ok_or_else(|| anyhow!("Username is missing"))?;
        let password = tokio.get_password().ok_or_else(|| anyhow!("Password is missing"))?;
        let database = tokio.get_dbname().ok_or_else(|| anyhow!("Database is missing"))?;
        let password =
            str::from_utf8(password).map_err(|_e| anyhow!("Password contains invalid UTF-8"))?;

        // convert tokio_postgres::Config to deadpool_postgres::Config
        let mut deadpool = Self::new();
        deadpool.host = Some(host);
        deadpool.dbname = Some(database.to_string());
        deadpool.port = Some(port);
        deadpool.user = Some(username.to_string());
        deadpool.password = Some(password.to_owned());
        deadpool.pool = Some(PoolConfig {
            max_size: options.pool_size,
            ..PoolConfig::default()
        });
        deadpool.ssl_mode = match tokio.get_ssl_mode() {
            SslMode::Require => Some(deadpool_postgres::SslMode::Require),
            SslMode::Prefer => Some(deadpool_postgres::SslMode::Prefer),
            _ => None,
        };

        Ok(deadpool)
    }
}
