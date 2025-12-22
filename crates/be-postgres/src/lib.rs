#![cfg(not(target_arch = "wasm32"))]

//! Postgres client builder for runtime.
//!
//! TODO: This attempt uses an enum in the wit for data types as a way to map
//! a few of the many Postgres types to Rust types. Investigate if using any low
//! level tools inside an ORM crate like Diesel would get us better type
//! coverage with less effort.

mod sql;
mod types;

use std::collections::HashMap;
use std::str;

use anyhow::{Context as _, Result, anyhow};
use deadpool_postgres::{Pool, PoolConfig, Runtime};
use fromenv::FromEnv;
use kernel::Backend;
use rustls::crypto::ring;
use rustls::{ClientConfig, RootCertStore};
use tokio_postgres::config::{Host, SslMode};
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::instrument;
use webpki_roots::TLS_SERVER_ROOTS;

/// Postgres client
#[derive(Clone, Debug)]
pub struct Client(HashMap<String, Pool>);

/// Postgres resource builder
impl Backend for Client {
    type ConnectOptions = ConnectOptions;

    /// Connect to `PostgreSQL` with provided options and return a connection pool
    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let mut pools = HashMap::new();
        let runtime = Some(Runtime::Tokio1);
        let mut tls_factory: Option<MakeRustlsConnect> = None; // factory is cheaper to clone

        for entry in std::iter::once(&options.default_pool).chain(&options.additional_pools) {
            let pool_config = deadpool_postgres::Config::try_from(entry)?;

            let pool = if pool_config.ssl_mode.is_none() {
                // Non-TLS mode
                pool_config
                    .create_pool(runtime, tokio_postgres::NoTls)
                    .context(format!("failed to create postgres pool: '{}'", entry.name))?
            } else {
                // TLS mode
                let factory = if let Some(f) = &tls_factory {
                    f.clone()
                } else {
                    ring::default_provider()
                        .install_default()
                        .map_err(|_e| anyhow!("Failed to install rustls crypto provider"))?;

                    let mut cert_store = RootCertStore::empty();
                    cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());

                    let client_config = ClientConfig::builder()
                        .with_root_certificates(cert_store)
                        .with_no_client_auth();

                    let factory = MakeRustlsConnect::new(client_config);
                    tls_factory = Some(factory.clone());

                    factory
                };

                pool_config
                    .create_pool(runtime, factory) // unwrap is safe here
                    .context(format!("failed to create postgres pool: '{}'", entry.name))?
            };

            // Check pool is usable
            let cnn = pool.get().await;
            if cnn.is_err() {
                return Err(anyhow!("failed to get connection from pool: {:?}", cnn.err()));
            }

            tracing::info!(
                "connected to Postgres database {:?}, with pool name '{}', tls '{}'",
                pool_config.dbname.unwrap_or_default(),
                entry.name,
                pool_config.ssl_mode.is_none()
            );
            pools.insert(entry.name.clone(), pool);
        }

        Ok(Self(pools))
    }
}

#[derive(Debug, Clone)]
pub struct PoolEntry {
    pub name: String, // e.g. "eventstore"
    pub uri: String,
    pub pool_size: usize,
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    pub default_pool: PoolEntry,
    pub additional_pools: Vec<PoolEntry>,
}

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        // default pool (required)
        let default_uri = std::env::var("POSTGRES_URL").context("POSTGRES_URL must be set");
        let default_size =
            std::env::var("POSTGRES_POOL_SIZE").unwrap_or_default().parse().unwrap_or(10);

        let default = PoolEntry {
            name: "default".to_ascii_uppercase(),
            uri: default_uri?,
            pool_size: default_size,
        };

        // optional extra pools: POSTGRES_POOLS=eventstore
        let extras = std::env::var("POSTGRES_POOLS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| -> anyhow::Result<PoolEntry> {
                let name = name.to_ascii_uppercase();
                let uri_key = format!("POSTGRES_URL__{name}");
                let size_key = format!("POSTGRES_POOL_SIZE__{name}");

                let uri = std::env::var(&uri_key)
                    .with_context(|| format!("missing {uri_key} for pool {name}"))?;
                let pool_size = std::env::var(&size_key)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(default.pool_size);

                Ok(PoolEntry { name, uri, pool_size })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            default_pool: default,
            additional_pools: extras,
        })
        // Self::from_env().finalize().context("issue loading connection options")
    }
}

impl TryFrom<&PoolEntry> for deadpool_postgres::Config {
    type Error = anyhow::Error;

    fn try_from(options: &PoolEntry) -> Result<Self> {
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
        let password = str::from_utf8(password).context("Password contains invalid UTF-8")?;
        let cli_options = tokio.get_options().unwrap_or_default();

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
        deadpool.options = Some(cli_options.to_string());

        Ok(deadpool)
    }
}
