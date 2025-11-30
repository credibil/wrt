#![cfg(not(target_arch = "wasm32"))]

// compile-time feature combination checks
// #[cfg(all(feature = "keyvalue", feature = "nats", feature = "redis"))]
// compile_error!("features \"nats\" and \"redis\" cannot be enabled for keyvalue at the same time");

pub mod env;

use std::path::PathBuf;

use anyhow::Result;
use fromenv::FromEnv;
use futures::future::{BoxFuture, TryJoinAll};
#[cfg(feature = "azure")]
use res_azure::Client as AzureCtx;
#[cfg(all(feature = "kafka", not(feature = "nats")))]
use res_kafka::Client as KafkaCtx;
#[cfg(feature = "mongodb")]
use res_mongodb::Client as MongoDbCtx;
#[cfg(feature = "nats")]
use res_nats::Client as NatsCtx;
#[cfg(feature = "postgres")]
use res_postgres::Client as PostgresCtx;
#[cfg(feature = "redis")]
use res_redis::Client as RedisCtx;
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
#[cfg(any(feature = "http", feature = "messaging", feature = "websockets"))]
use runtime::Server;
use runtime::{Runtime, State};
#[cfg(feature = "blobstore")]
use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxView, WasiBlobstoreView};
#[cfg(feature = "http")]
use wasi_http::{WasiHttp, WasiHttpCtx, WasiHttpCtxView, WasiHttpView};
#[cfg(feature = "identity")]
use wasi_identity::{
    DefaultIdentityCtx as IdentityCtx, WasiIdentity, WasiIdentityCtxView, WasiIdentityView,
};
#[cfg(feature = "keyvalue")]
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxView, WasiKeyValueView};
#[cfg(feature = "messaging")]
use wasi_messaging::{WasiMessaging, WasiMessagingCtxView, WasiMessagingView};
#[cfg(feature = "otel")]
use wasi_otel::{DefaultOtelCtx as OtelCtx, WasiOtel, WasiOtelCtxView, WasiOtelView};
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

// Helper macro to implement the `WasiView` trait for a given feature.
macro_rules! wasi_view {
    ($trait:ident, $method:ident, $ctx_view:ident, $field:ident) => {
        impl $trait for RunData {
            fn $method(&mut self) -> $ctx_view<'_> {
                $ctx_view {
                    ctx: &mut self.$field,
                    table: &mut self.table,
                }
            }
        }
    };
}

#[derive(Debug, Clone, FromEnv)]
pub struct RuntimeConfig {
    #[env(from = "ENV", default = "dev")]
    pub environment: String,
    #[env(from = "COMPONENT")]
    pub component: Option<String>,
    #[env(from = "OTEL_GRPC_URL")]
    pub otel_grpc_url: Option<String>,
}

/// Run the specified wasm guest using the specified top-level feature set.
///
/// # Errors
///
/// Returns an error if the wasm file cannot be run, or if the runtime fails to
/// instantiate the component.
pub async fn run(wasm: PathBuf) -> Result<()> {
    let mut config = RuntimeConfig::from_env().finalize()?;

    // SAFETY: Setting environment variables is safe at this point because it
    // the runtime still starting and no threads have been spawned.
    unsafe {
        let component = wasm.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
        if std::env::var("COMPONENT").is_err() {
            config.component = Some(component.to_string());
            std::env::set_var("COMPONENT", component);
        }
        #[cfg(feature = "kafka")]
        std::env::set_var("KAFKA_CLIENT_ID", format!("{component}-{}", &config.environment));
    };

    // load or compile wasm component
    let mut compiled = Runtime::new().from_file::<RunData>(&wasm)?;

    // link dependencies
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

    // prepare state
    let run_state = RunState {
        instance_pre: compiled.pre_instantiate()?,

        #[cfg(feature = "azure")]
        azure_ctx: AzureCtx::connect().await?,
        #[cfg(feature = "identity")]
        identity_ctx: IdentityCtx::connect().await?,
        #[cfg(all(feature = "kafka", not(feature = "nats")))]
        kafka_ctx: KafkaCtx::connect().await?,
        #[cfg(feature = "mongodb")]
        mongodb_ctx: MongoDbCtx::connect().await?,
        #[cfg(feature = "nats")]
        nats_ctx: NatsCtx::connect().await?,
        #[cfg(feature = "postgres")]
        postgres_ctx: PostgresCtx::connect().await?,
        #[cfg(feature = "redis")]
        redis_ctx: RedisCtx::connect().await?,
        #[cfg(feature = "otel")]
        otel_ctx: OtelCtx::connect().await?,
    };

    // start server(s)
    let futures: Vec<BoxFuture<'_, Result<()>>> = vec![
        #[cfg(feature = "http")]
        Box::pin(WasiHttp.run(&run_state)),
        #[cfg(feature = "messaging")]
        Box::pin(WasiMessaging.run(&run_state)),
        #[cfg(feature = "websockets")]
        Box::pin(WasiWebSockets.run(&run_state)),
    ];
    futures.into_iter().collect::<TryJoinAll<_>>().await?;

    Ok(())
}

#[derive(Clone)]
pub struct RunState {
    instance_pre: InstancePre<RunData>,

    #[cfg(feature = "azure")]
    azure_ctx: AzureCtx,
    #[cfg(feature = "identity")]
    identity_ctx: IdentityCtx,
    #[cfg(all(feature = "kafka", not(feature = "nats")))]
    kafka_ctx: KafkaCtx,
    #[cfg(feature = "mongodb")]
    mongodb_ctx: MongoDbCtx,
    #[cfg(feature = "nats")]
    nats_ctx: NatsCtx,
    #[cfg(feature = "postgres")]
    postgres_ctx: PostgresCtx,
    #[cfg(feature = "redis")]
    redis_ctx: RedisCtx,
    #[cfg(feature = "otel")]
    otel_ctx: OtelCtx,
}

impl State for RunState {
    type StoreData = RunData;

    fn instance_pre(&self) -> &InstancePre<Self::StoreData> {
        &self.instance_pre
    }

    fn new_store(&self) -> Self::StoreData {
        let mut ctx = WasiCtxBuilder::new();
        let wasi_ctx = ctx
            .inherit_args()
            .inherit_env()
            .inherit_stdin()
            .stdout(tokio::io::stdout())
            .stderr(tokio::io::stderr())
            .build();

        RunData {
            table: ResourceTable::new(),
            wasi_ctx,
            #[cfg(all(feature = "blobstore", feature = "nats", not(feature = "mongodb")))]
            blobstore_ctx: self.nats_ctx.clone(),
            #[cfg(all(feature = "blobstore", feature = "mongodb"))]
            blobstore_ctx: self.mongodb_ctx.clone(),
            #[cfg(feature = "http")]
            http_ctx: WasiHttpCtx,
            #[cfg(feature = "identity")]
            identity_ctx: self.identity_ctx.clone(),
            #[cfg(all(feature = "keyvalue", feature = "nats", not(feature = "redis")))]
            keyvalue_ctx: self.nats_ctx.clone(),
            #[cfg(all(feature = "keyvalue", feature = "redis"))]
            keyvalue_ctx: self.redis_ctx.clone(),
            #[cfg(all(feature = "messaging", feature = "kafka", not(feature = "nats")))]
            messaging_ctx: self.kafka_ctx.clone(),
            #[cfg(all(feature = "messaging", feature = "nats"))]
            messaging_ctx: self.nats_ctx.clone(),
            #[cfg(feature = "otel")]
            otel_ctx: self.otel_ctx.clone(),
            #[cfg(all(feature = "sql", feature = "postgres"))]
            sql_ctx: self.postgres_ctx.clone(),
            #[cfg(all(feature = "vault", feature = "azure"))]
            vault_ctx: self.azure_ctx.clone(),
            #[cfg(feature = "websockets")]
            websockets_ctx: DefaultWebSocketsCtx,
        }
    }
}

/// `RunData` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunData {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    #[cfg(all(feature = "blobstore", feature = "nats", not(feature = "mongodb")))]
    pub blobstore_ctx: NatsCtx,
    #[cfg(all(feature = "blobstore", feature = "mongodb"))]
    pub blobstore_ctx: MongoDbCtx,
    #[cfg(feature = "http")]
    pub http_ctx: WasiHttpCtx,
    #[cfg(feature = "identity")]
    pub identity_ctx: IdentityCtx,
    #[cfg(all(feature = "keyvalue", feature = "nats", not(feature = "redis")))]
    pub keyvalue_ctx: NatsCtx,
    #[cfg(all(feature = "keyvalue", feature = "redis"))]
    pub keyvalue_ctx: RedisCtx,
    #[cfg(all(feature = "messaging", feature = "kafka", not(feature = "nats")))]
    pub messaging_ctx: KafkaCtx,
    #[cfg(all(feature = "messaging", feature = "nats"))]
    pub messaging_ctx: NatsCtx,
    #[cfg(feature = "otel")]
    pub otel_ctx: OtelCtx,
    #[cfg(all(feature = "sql", feature = "postgres"))]
    pub sql_ctx: PostgresCtx,
    #[cfg(all(feature = "vault", feature = "azure"))]
    pub vault_ctx: AzureCtx,
    #[cfg(feature = "websockets")]
    pub websockets_ctx: DefaultWebSocketsCtx,
}

impl WasiView for RunData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "blobstore")]
wasi_view!(WasiBlobstoreView, blobstore, WasiBlobstoreCtxView, blobstore_ctx);

#[cfg(feature = "http")]
wasi_view!(WasiHttpView, http, WasiHttpCtxView, http_ctx);

#[cfg(feature = "identity")]
wasi_view!(WasiIdentityView, identity, WasiIdentityCtxView, identity_ctx);

#[cfg(feature = "keyvalue")]
wasi_view!(WasiKeyValueView, keyvalue, WasiKeyValueCtxView, keyvalue_ctx);

#[cfg(feature = "messaging")]
wasi_view!(WasiMessagingView, messaging, WasiMessagingCtxView, messaging_ctx);

#[cfg(feature = "otel")]
wasi_view!(WasiOtelView, otel, WasiOtelCtxView, otel_ctx);

#[cfg(feature = "sql")]
wasi_view!(WasiSqlView, sql, WasiSqlCtxView, sql_ctx);

#[cfg(feature = "vault")]
wasi_view!(WasiVaultView, vault, WasiVaultCtxView, vault_ctx);

#[cfg(feature = "websockets")]
wasi_view!(WebSocketsView, start, WasiWebSocketsCtxView, websockets_ctx);
