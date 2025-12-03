//! WebAssembly Component Runtime
//!
//! This module provides the runtime infrastructure for executing WebAssembly
//! components with WASI (WebAssembly System Interface) capabilities. It supports
//! various backend services through feature flags:
//!
//! ## Backend Features
//! - `azure` - Azure services (Key Vault, Identity)
//! - `kafka` - Apache Kafka messaging (mutually exclusive with `nats` for messaging)
//! - `mongodb` - MongoDB blob storage
//! - `nats` - NATS messaging, key-value, and blob storage
//! - `postgres` - `PostgreSQL` database
//! - `redis` - Redis key-value storage
//!
//! ## WASI Interface Features
//! - `blobstore` - Object/blob storage interface
//! - `http` - HTTP client and server interface
//! - `identity` - Identity and authentication interface
//! - `keyvalue` - Key-value storage interface
//! - `messaging` - Pub/sub messaging interface
//! - `otel` - `OpenTelemetry` observability interface
//! - `sql` - SQL database interface
//! - `vault` - Secrets management interface
//! - `websockets` - `WebSocket` interface

#![cfg(not(target_arch = "wasm32"))]

pub mod env;

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use fromenv::FromEnv;
use futures::future::{BoxFuture, try_join_all};
// Backend clients
#[cfg(feature = "azure")]
use res_azure::Client as Azure;
#[cfg(all(feature = "kafka", not(feature = "nats")))]
use res_kafka::Client as Kafka;
#[cfg(feature = "mongodb")]
use res_mongodb::Client as MongoDb;
#[cfg(feature = "nats")]
use res_nats::Client as Nats;
#[cfg(feature = "postgres")]
use res_postgres::Client as Postgres;
#[cfg(all(feature = "redis", not(feature = "nats")))]
use res_redis::Client as Redis;
#[cfg(any(
    feature = "azure",
    feature = "kafka",
    feature = "mongodb",
    feature = "nats",
    feature = "redis",
    feature = "postgres",
    feature = "otel"
))]
use runtime::Resource;
use runtime::{Compiled, Runtime, Server, State};
#[cfg(feature = "blobstore")]
use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxView, WasiBlobstoreView};
#[cfg(feature = "http")]
use wasi_http::{WasiHttp, WasiHttpCtx, WasiHttpCtxView, WasiHttpView};
#[cfg(all(feature = "identity", not(feature = "azure")))]
use wasi_identity::DefaultIdentity;
#[cfg(feature = "identity")]
use wasi_identity::{WasiIdentity, WasiIdentityCtxView, WasiIdentityView};
#[cfg(feature = "keyvalue")]
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxView, WasiKeyValueView};
#[cfg(feature = "messaging")]
use wasi_messaging::{WasiMessaging, WasiMessagingCtxView, WasiMessagingView};
#[cfg(feature = "otel")]
use wasi_otel::{DefaultOtelCtx as Otel, WasiOtel, WasiOtelCtxView, WasiOtelView};
#[cfg(feature = "sql")]
use wasi_sql::{WasiSql, WasiSqlCtxView, WasiSqlView};
#[cfg(feature = "vault")]
use wasi_vault::{WasiVault, WasiVaultCtxView, WasiVaultView};
#[cfg(feature = "websockets")]
use wasi_websockets::{
    DefaultWebSocketsCtx, WasiWebSockets, WasiWebSocketsCtxView, WebSocketsView,
};
use wasmtime::component::InstancePre;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Run the specified wasm guest using the specified top-level feature set.
///
/// # Errors
///
/// Returns an error if the wasm file cannot be run, or if the runtime fails to
/// instantiate the component.
pub async fn run(wasm: PathBuf) -> Result<()> {
    RuntimeConfig::from_wasm(&wasm)?;

    let mut compiled =
        Runtime::new().build(&wasm).with_context(|| format!("compiling {}", wasm.display()))?;
    tracing::info!("wasm component compiled successfully");
    let run_state = Context::new(&mut compiled).await.context("preparing runtime state")?;
    run_state.start().await.context("starting runtime services")
}

/// Runtime configuration loaded from environment variables.
///
/// This configuration is used to customize runtime behavior and is
/// automatically populated from the environment at startup.
#[derive(Debug, Clone, FromEnv)]
pub struct RuntimeConfig {
    /// Deployment environment (e.g., "dev", "staging", "prod").
    #[env(from = "ENV", default = "dev")]
    pub environment: String,

    /// Component name, derived from the wasm filename if not specified.
    #[env(from = "COMPONENT")]
    pub component: Option<String>,

    /// OpenTelemetry collector gRPC endpoint URL.
    #[env(from = "OTEL_GRPC_URL")]
    pub otel_grpc_url: Option<String>,
}

impl RuntimeConfig {
    /// Creates a runtime configuration from a wasm file path.
    ///
    /// Loads configuration from environment variables and derives the
    /// component name from the wasm filename if not already set.
    fn from_wasm(wasm: &Path) -> Result<Self> {
        let mut config = Self::from_env().finalize()?;
        let component = wasm.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // SAFETY: Environment variable modification is safe here because:
        // 1. This runs during single-threaded initialization
        // 2. No other threads have been spawned yet
        // 3. Resource clients that depend on these vars are created after this
        unsafe {
            if std::env::var("COMPONENT").is_err() {
                config.component = Some(component.clone());
                std::env::set_var("COMPONENT", &component);
            }
            #[cfg(feature = "kafka")]
            std::env::set_var("KAFKA_CLIENT_ID", format!("{component}-{}", &config.environment));
        };

        Ok(config)
    }
}

/// Runtime state holding pre-instantiated components and backend connections.
///
/// This struct is cloneable and shared across request handlers. Each backend
/// context is connected during initialization and cloned for each request.
#[derive(Clone)]
struct Context {
    instance_pre: InstancePre<StoreCtx>,

    #[cfg(feature = "azure")]
    azure: Azure,
    #[cfg(all(feature = "identity", not(feature = "azure")))]
    identity: DefaultIdentity,
    #[cfg(all(feature = "kafka", not(feature = "nats")))]
    kafka: Kafka,
    #[cfg(feature = "mongodb")]
    mongodb: MongoDb,
    #[cfg(feature = "nats")]
    nats: Nats,
    #[cfg(feature = "postgres")]
    postgres: Postgres,
    #[cfg(all(feature = "redis", not(feature = "nats")))]
    redis: Redis,
    #[cfg(feature = "otel")]
    otel: Otel,
}

impl Context {
    /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
    async fn new(compiled: &mut Compiled<StoreCtx>) -> Result<Self> {
        // Link all enabled WASI interfaces to the component
        #[cfg(feature = "blobstore")]
        compiled.link(WasiBlobstore)?;
        #[cfg(feature = "http")]
        compiled.link(WasiHttp)?;
        #[cfg(feature = "identity")]
        compiled.link(WasiIdentity)?;
        #[cfg(feature = "keyvalue")]
        compiled.link(WasiKeyValue)?;
        #[cfg(feature = "messaging")]
        compiled.link(WasiMessaging)?;
        #[cfg(feature = "otel")]
        compiled.link(WasiOtel)?;
        #[cfg(feature = "sql")]
        compiled.link(WasiSql)?;
        #[cfg(feature = "vault")]
        compiled.link(WasiVault)?;
        #[cfg(feature = "websockets")]
        compiled.link(WasiWebSockets)?;
        tracing::info!("linking okay");

        Ok(Self {
            instance_pre: compiled.pre_instantiate()?,
            #[cfg(feature = "azure")]
            azure: Azure::connect().await?,
            #[cfg(all(feature = "identity", not(feature = "azure")))]
            identity: DefaultIdentity::connect().await?,
            #[cfg(all(feature = "kafka", not(feature = "nats")))]
            kafka: Kafka::connect().await?,
            #[cfg(feature = "mongodb")]
            mongodb: MongoDb::connect().await?,
            #[cfg(feature = "nats")]
            nats: Nats::connect().await?,
            #[cfg(feature = "postgres")]
            postgres: Postgres::connect().await?,
            #[cfg(all(feature = "redis", not(feature = "nats")))]
            redis: Redis::connect().await?,
            #[cfg(feature = "otel")]
            otel: Otel::connect().await?,
        })
    }

    /// Starts all enabled server interfaces (HTTP, messaging, websockets).
    ///
    /// All servers run concurrently and the function returns when any server fails.
    #[allow(clippy::vec_init_then_push)]
    async fn start(&self) -> Result<()> {
        let futures: Vec<BoxFuture<'_, Result<()>>> = vec![
            #[cfg(feature = "http")]
            Box::pin(WasiHttp.run(self)),
            #[cfg(feature = "messaging")]
            Box::pin(WasiMessaging.run(self)),
            #[cfg(feature = "websockets")]
            Box::pin(WasiWebSockets.run(self)),
        ];
        try_join_all(futures).await?;
        Ok(())
    }
}

impl State for Context {
    type StoreCtx = StoreCtx;

    fn instance_pre(&self) -> &InstancePre<Self::StoreCtx> {
        &self.instance_pre
    }

    fn new_store(&self) -> Self::StoreCtx {
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_args()
            .inherit_env()
            .inherit_stdin()
            .stdout(tokio::io::stdout())
            .stderr(tokio::io::stderr())
            .build();

        StoreCtx {
            table: ResourceTable::new(),
            wasi: wasi_ctx,

            // Blobstore (favouring MongoDB over NATS)
            #[cfg(all(feature = "blobstore", feature = "nats", not(feature = "mongodb")))]
            blobstore: self.nats.clone(),
            #[cfg(all(feature = "blobstore", feature = "mongodb"))]
            blobstore: self.mongodb.clone(),

            #[cfg(feature = "http")]
            http: WasiHttpCtx,

            // Identity (favouring Azure over default)
            #[cfg(all(feature = "identity", not(feature = "azure")))]
            identity: self.identity.clone(),
            #[cfg(all(feature = "identity", feature = "azure"))]
            identity: self.azure.clone(),

            // Key-value (favouring NATS over Redis)
            #[cfg(all(feature = "keyvalue", feature = "redis", not(feature = "nats")))]
            keyvalue: self.redis.clone(),
            #[cfg(all(feature = "keyvalue", feature = "nats"))]
            keyvalue: self.nats.clone(),

            // Messaging (favouring NATS over Kafka)
            #[cfg(all(feature = "messaging", feature = "kafka", not(feature = "nats")))]
            messaging: self.kafka.clone(),
            #[cfg(all(feature = "messaging", feature = "nats"))]
            messaging: self.nats.clone(),

            #[cfg(feature = "otel")]
            otel: self.otel.clone(),

            #[cfg(all(feature = "sql", feature = "postgres"))]
            sql: self.postgres.clone(),

            #[cfg(all(feature = "vault", feature = "azure"))]
            vault: self.azure.clone(),

            #[cfg(feature = "websockets")]
            websockets: DefaultWebSocketsCtx,
        }
    }
}

/// Per-instance data shared between the WebAssembly runtime and host functions.
///
/// Each component instantiation receives its own `StoreCtx` with cloned backend
/// contexts. The `table` field manages WASI resource handles, while individual
/// `*_ctx` fields provide access to the corresponding WASI interface backends.
pub struct StoreCtx {
    /// Resource table for managing WASI handles.
    pub table: ResourceTable,

    /// Core WASI context (filesystem, environment, stdio).
    pub wasi: WasiCtx,

    /// Blobstore context (NATS or MongoDB backend).
    #[cfg(all(feature = "blobstore", feature = "nats", not(feature = "mongodb")))]
    pub blobstore: Nats,
    #[cfg(all(feature = "blobstore", feature = "mongodb"))]
    pub blobstore: MongoDb,

    /// HTTP client/server context.
    #[cfg(feature = "http")]
    pub http: WasiHttpCtx,

    /// Identity/authentication context.
    #[cfg(all(feature = "identity", not(feature = "azure")))]
    pub identity: DefaultIdentity,
    #[cfg(all(feature = "identity", feature = "azure"))]
    pub identity: Azure,

    /// Key-value storage context (NATS or Redis backend).
    #[cfg(all(feature = "keyvalue", feature = "redis", not(feature = "nats")))]
    pub keyvalue: Redis,
    #[cfg(all(feature = "keyvalue", feature = "nats"))]
    pub keyvalue: Nats,

    /// Messaging context (Kafka or NATS backend).
    #[cfg(all(feature = "messaging", feature = "kafka", not(feature = "nats")))]
    pub messaging: Kafka,
    #[cfg(all(feature = "messaging", feature = "nats"))]
    pub messaging: Nats,

    /// OpenTelemetry observability context.
    #[cfg(feature = "otel")]
    pub otel: Otel,

    /// SQL database context (`PostgreSQL` backend).
    #[cfg(all(feature = "sql", feature = "postgres"))]
    pub sql: Postgres,

    /// Secrets vault context (Azure Key Vault backend).
    #[cfg(all(feature = "vault", feature = "azure"))]
    pub vault: Azure,

    /// `WebSocket` context.
    #[cfg(feature = "websockets")]
    pub websockets: DefaultWebSocketsCtx,
}

// ============================================================================
// WASI View Implementations
// ============================================================================
// Generate the trait implementations to connect WASI interfaces to their
// corresponding context in `StoreCtx`.

/// Implements a WASI view trait for `StoreCtx`.
///
/// This macro generates the boilerplate for connecting WASI interface traits
/// to their corresponding context fields in `StoreCtx`. Each WASI interface
/// requires a view trait that provides access to the interface context and
/// the resource table.
///
/// # Arguments
/// - `$trait` - The view trait to implement (e.g., `WasiHttpView`)
/// - `$method` - The method name that returns the context view
/// - `$ctx_view` - The context view type to construct
/// - `$field` - The field in `StoreCtx` holding the context
macro_rules! wasi_view {
    ($trait:ident, $method:ident, $ctx_view:ident, $field:ident) => {
        impl $trait for StoreCtx {
            fn $method(&mut self) -> $ctx_view<'_> {
                $ctx_view {
                    ctx: &mut self.$field,
                    table: &mut self.table,
                }
            }
        }
    };
}

wasi_view!(WasiView, ctx, WasiCtxView, wasi);

#[cfg(feature = "blobstore")]
wasi_view!(WasiBlobstoreView, blobstore, WasiBlobstoreCtxView, blobstore);
#[cfg(feature = "http")]
wasi_view!(WasiHttpView, http, WasiHttpCtxView, http);
#[cfg(feature = "identity")]
wasi_view!(WasiIdentityView, identity, WasiIdentityCtxView, identity);
#[cfg(feature = "keyvalue")]
wasi_view!(WasiKeyValueView, keyvalue, WasiKeyValueCtxView, keyvalue);
#[cfg(feature = "messaging")]
wasi_view!(WasiMessagingView, messaging, WasiMessagingCtxView, messaging);
#[cfg(feature = "otel")]
wasi_view!(WasiOtelView, otel, WasiOtelCtxView, otel);
#[cfg(feature = "sql")]
wasi_view!(WasiSqlView, sql, WasiSqlCtxView, sql);
#[cfg(feature = "vault")]
wasi_view!(WasiVaultView, vault, WasiVaultCtxView, vault);
#[cfg(feature = "websockets")]
wasi_view!(WebSocketsView, start, WasiWebSocketsCtxView, websockets);
