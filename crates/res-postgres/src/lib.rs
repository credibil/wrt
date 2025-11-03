//! Postgres client builder for runtime.

mod sql;

use std::env;
use std::str;

use anyhow::{Context as _, Result, anyhow};
use deadpool_postgres::{Config, Pool, PoolConfig, Runtime};
use runtime::Resource;
use rustls::crypto::ring;
use rustls::{ClientConfig, RootCertStore};
use tokio_postgres::config::{Host, SslMode};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::instrument;
use webpki_roots::TLS_SERVER_ROOTS;

/// Default Postgres connection parameters
const DEF_URI: &str = "postgres://postgres:pass@localhost:5432/postgres?sslmode=disable";
const DEF_POOL_SIZE: &str = "10";

/// Postgres client
#[derive(Debug)]
pub struct Client(Pool);

/// Postgres resource builder
impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    /// Connect to `PostgreSQL` and return a connection pool
    #[instrument(name = "Postgres::connect")]
    async fn connect() -> Result<Self> {
        let options = ConnectOptions::from_env()?;
        Self::connect_with(&options).await
    }

    /// Connect to `PostgreSQL` with provided options and return a connection pool
    async fn connect_with(options: &Self::ConnectOptions) -> Result<Self> {
        let pool_config = Config::try_from(options)?;

        let runtime = Some(Runtime::Tokio1);

        if pool_config.ssl_mode.is_none() {
            // Non-TLS mode
            let pool = pool_config
                .create_pool(runtime, tokio_postgres::NoTls)
                .context("failed to create non-TLS postgres pool")?;
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
            .context("failed to create non-TLS postgres pool")?;

        tracing::info!("connected to Postgres");

        Ok(Self(pool))
    }
}

pub struct ConnectOptions {
    pub uri: String,
    pub pool_size: usize,
}

impl ConnectOptions {
    /// Create connection options from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing or invalid.
    pub fn from_env() -> Result<Self> {
        let uri = env::var("POSTGRES_URI").unwrap_or_else(|_| DEF_URI.into());
        let pool_size = env::var("POSTGRES_POOL_SIZE")
            .unwrap_or_else(|_| DEF_POOL_SIZE.into())
            .parse::<usize>()?;

        Ok(Self { uri, pool_size })
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
