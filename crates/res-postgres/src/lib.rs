//! Postgres client builder for runtime.

mod sql;

use std::env;

use anyhow::{Context as _, Result};
use deadpool_postgres::{Config, Pool, PoolConfig};
use runtime::Resource;
use tokio_postgres::config::SslMode;
use tracing::{instrument, warn};

/// Default Postgres connection parameters
const DEF_URI: &str = "postgres://postgres:pass@localhost:5432/postgres?sslmode=disable";
const DEF_POOL_SIZE: usize = 10;

/// Postgres resource
#[derive(Debug)]
pub struct Client(Pool);

/// Postgres resource builder
impl Resource for Client {
    /// Connect to `PostgreSQL` and return a connection pool
    #[instrument(name = "Postgres::connect")]
    async fn connect() -> Result<Self> {
        let uri = env::var("POSTGRES_URI").unwrap_or_else(|_| DEF_URI.into());
        let pg_cfg: tokio_postgres::Config =
            uri.parse().with_context(|| format!("invalid POSTGRES_URI: {uri}"))?;

        let host = pg_cfg
            .get_hosts()
            .first()
            .map(|h| match h {
                tokio_postgres::config::Host::Tcp(name) => name.clone(),
                tokio_postgres::config::Host::Unix(path) => path.to_string_lossy().into_owned(),
            })
            .unwrap_or_default();
        let port = pg_cfg
            .get_ports()
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Port is missing"))?;
        let username = pg_cfg.get_user().ok_or_else(|| anyhow::anyhow!("Username is missing"))?;
        let password =
            pg_cfg.get_password().ok_or_else(|| anyhow::anyhow!("Password is missing"))?;
        let database = pg_cfg.get_dbname().ok_or_else(|| anyhow::anyhow!("Database is missing"))?;

        let tls_required = matches!(pg_cfg.get_ssl_mode(), SslMode::Require);

        let pool_size = Some(
            env::var("POSTGRES_POOL_SIZE")
                .unwrap_or_else(|_e| DEF_POOL_SIZE.to_string())
                .parse::<usize>()
                .unwrap_or_else(|_e| {
                    warn!("invalid pool size, using {DEF_POOL_SIZE}");
                    DEF_POOL_SIZE
                }),
        );

        let cfg = ConnectOptions {
            host,
            port,
            username: username.to_string(),
            password: String::from_utf8_lossy(password).into_owned(),
            database: database.to_string(),
            tls_required,
            pool_size,
        };

        let tls_required = cfg.tls_required;
        let pool_cfg = Config::from(cfg);

        let pool = create_connection_pool(&pool_cfg, tls_required)?;
        tracing::info!("connected to Postgres");

        Ok(Self(pool))
    }
}

/// Creation options for a Postgres connection
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectOptions {
    /// Hostname of the Postgres cluster to connect to
    pub host: String,
    /// Port on which to connect to the Postgres cluster
    pub port: u16,
    /// Username used when accessing the Postgres cluster
    pub username: String,
    /// Password used when accessing the Postgres cluster
    pub password: String,
    /// Database to connect to
    pub database: String,
    /// Whether TLS is required for the connection
    pub tls_required: bool,
    /// Optional connection pool size
    pub pool_size: Option<usize>,
}

impl From<ConnectOptions> for Config {
    fn from(opts: ConnectOptions) -> Self {
        let mut cfg = Self::new();
        cfg.host = Some(opts.host);
        cfg.user = Some(opts.username);
        cfg.password = Some(opts.password);
        cfg.dbname = Some(opts.database);
        cfg.port = Some(opts.port);
        if let Some(pool_size) = opts.pool_size {
            cfg.pool = Some(PoolConfig {
                max_size: pool_size,
                ..PoolConfig::default()
            });
        }
        cfg
    }
}

/// Method creates connection pool with enabled or disabled TLS
///
/// # Errors
///
/// Will return `Err` if not possible to create pool
pub fn create_connection_pool(cfg: &Config, tls_required: bool) -> Result<Pool> {
    let runtime = Some(deadpool_postgres::Runtime::Tokio1);
    if tls_required {
        create_tls_pool(cfg, runtime)
    } else {
        cfg.create_pool(runtime, tokio_postgres::NoTls)
            .context("failed to create non-TLS postgres pool")
    }
}

/// Method creates connection pool with enabled TLS
fn create_tls_pool(
    cfg: &deadpool_postgres::Config, runtime: Option<deadpool_postgres::Runtime>,
) -> Result<Pool> {
    //setting default crypto provider for tls connection
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    let mut store = rustls::RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    cfg.create_pool(
        runtime,
        tokio_postgres_rustls::MakeRustlsConnect::new(
            rustls::ClientConfig::builder().with_root_certificates(store).with_no_client_auth(),
        ),
    )
    .context("failed to create TLS-enabled connection pool")
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------
    // Test conversion from ConnectOptions to Config
    // -------------------------
    #[test]
    fn test_connection_options_to_config() {
        let opts = ConnectOptions {
            host: "localhost".into(),
            port: 5432,
            username: "user".into(),
            password: "pass".into(),
            database: "db".into(),
            tls_required: false,
            pool_size: Some(5),
        };

        let cfg: Config = opts.into();

        assert_eq!(cfg.host, Some("localhost".into()));
        assert_eq!(cfg.port, Some(5432));
        assert_eq!(cfg.user, Some("user".into()));
        assert_eq!(cfg.password, Some("pass".into()));
        assert_eq!(cfg.dbname, Some("db".into()));
        assert_eq!(cfg.pool.unwrap().max_size, 5);
    }
}
