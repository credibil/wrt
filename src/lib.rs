#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use anyhow::Result;
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
    feature = "postgres"
))]
use runtime::Resource;
#[cfg(any(feature = "http", feature = "messaging", feature = "websockets"))]
use runtime::Server;
use runtime::{Runtime, State};
#[cfg(feature = "blobstore")]
use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxView, WasiBlobstoreView};
#[cfg(feature = "http")]
use wasi_http::{WasiHttp, WasiHttpCtx, WasiHttpCtxView, WasiHttpView};
#[cfg(feature = "keyvalue")]
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxView, WasiKeyValueView};
#[cfg(feature = "messaging")]
use wasi_messaging::{WasiMessaging, WasiMessagingCtxView, WasiMessagingView};
#[cfg(feature = "otel")]
use wasi_otel::{DefaultOtelCtx, WasiOtel, WasiOtelCtxView, WasiOtelView};
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
    // link dependencies
    let mut compiled = Runtime::<RunData>::new(wasm).compile()?;
    #[cfg(feature = "blobstore")]
    compiled.link(WasiBlobstore)?;
    #[cfg(feature = "http")]
    compiled.link(WasiHttp)?;
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
    };

    // single server
    #[cfg(all(feature = "http", not(all(feature = "messaging", feature = "websockets"))))]
    tokio::try_join!(WasiHttp.run(&run_state))?;

    #[cfg(all(feature = "messaging", not(all(feature = "http", feature = "websockets"))))]
    tokio::try_join!(WasiMessaging.run(&run_state))?;

    #[cfg(all(feature = "websockets", not(all(feature = "http", feature = "messaging"))))]
    tokio::try_join!(WasiWebSockets.run(&run_state))?;

    // two servers
    #[cfg(all(feature = "http", feature = "messaging", not(feature = "websockets")))]
    tokio::try_join!(WasiHttp.run(&run_state), WasiMessaging.run(&run_state))?;

    #[cfg(all(feature = "http", feature = "websockets", not(feature = "messaging")))]
    tokio::try_join!(WasiHttp.run(&run_state), WasiWebSockets.run(&run_state))?;

    #[cfg(all(feature = "messaging", feature = "websockets", not(feature = "http")))]
    tokio::try_join!(WasiMessaging.run(&run_state), WasiWebSockets.run(&run_state))?;

    // three servers
    #[cfg(all(feature = "http", feature = "messaging", feature = "websockets"))]
    tokio::try_join!(
        WasiHttp.run(&run_state),
        WasiMessaging.run(&run_state),
        WasiWebSockets.run(&run_state)
    )?;

    Ok(())
}

#[derive(Clone)]
pub struct RunState {
    instance_pre: InstancePre<RunData>,

    #[cfg(feature = "azure")]
    azure_ctx: AzureCtx,
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
            #[cfg(all(feature = "blobstore", all(feature = "nats", not(feature = "mongodb"))))]
            blobstore_ctx: self.nats_ctx.clone(),
            #[cfg(all(feature = "blobstore", feature = "mongodb"))]
            blobstore_ctx: self.mongodb_ctx.clone(),
            #[cfg(feature = "http")]
            http_ctx: WasiHttpCtx,
            #[cfg(all(feature = "keyvalue", all(feature = "nats", not(feature = "redis"))))]
            keyvalue_ctx: self.nats_ctx.clone(),
            #[cfg(all(feature = "keyvalue", feature = "redis"))]
            keyvalue_ctx: self.redis_ctx.clone(),
            #[cfg(all(feature = "messaging", all(feature = "kafka", not(feature = "nats"))))]
            messaging_ctx: self.kafka_ctx.clone(),
            #[cfg(all(feature = "messaging", feature = "nats"))]
            messaging_ctx: self.nats_ctx.clone(),
            #[cfg(feature = "otel")]
            otel_ctx: DefaultOtelCtx,
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
    #[cfg(all(feature = "blobstore", all(feature = "nats", not(feature = "mongodb"))))]
    pub blobstore_ctx: NatsCtx,
    #[cfg(all(feature = "blobstore", feature = "mongodb"))]
    pub blobstore_ctx: MongoDbCtx,
    #[cfg(feature = "http")]
    pub http_ctx: WasiHttpCtx,
    #[cfg(all(feature = "keyvalue", all(feature = "nats", not(feature = "redis"))))]
    pub keyvalue_ctx: NatsCtx,
    #[cfg(all(feature = "keyvalue", feature = "redis"))]
    pub keyvalue_ctx: RedisCtx,
    #[cfg(all(feature = "messaging", all(feature = "kafka", not(feature = "nats"))))]
    pub messaging_ctx: KafkaCtx,
    #[cfg(all(feature = "messaging", feature = "nats"))]
    pub messaging_ctx: NatsCtx,
    #[cfg(feature = "otel")]
    pub otel_ctx: DefaultOtelCtx,
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
impl WasiBlobstoreView for RunData {
    fn blobstore(&mut self) -> WasiBlobstoreCtxView<'_> {
        WasiBlobstoreCtxView {
            ctx: &mut self.blobstore_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "http")]
impl WasiHttpView for RunData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "keyvalue")]
impl WasiKeyValueView for RunData {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        WasiKeyValueCtxView {
            ctx: &mut self.keyvalue_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "messaging")]
impl WasiMessagingView for RunData {
    fn messaging(&mut self) -> WasiMessagingCtxView<'_> {
        WasiMessagingCtxView {
            ctx: &mut self.messaging_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "otel")]
impl WasiOtelView for RunData {
    fn otel(&mut self) -> WasiOtelCtxView<'_> {
        WasiOtelCtxView {
            ctx: &mut self.otel_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "sql")]
impl WasiSqlView for RunData {
    fn sql(&mut self) -> WasiSqlCtxView<'_> {
        WasiSqlCtxView {
            ctx: &mut self.sql_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "vault")]
impl WasiVaultView for RunData {
    fn vault(&mut self) -> WasiVaultCtxView<'_> {
        WasiVaultCtxView {
            ctx: &mut self.vault_ctx,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "websockets")]
impl WebSocketsView for RunData {
    fn start(&mut self) -> WasiWebSocketsCtxView<'_> {
        WasiWebSocketsCtxView {
            ctx: &mut self.websockets_ctx,
            table: &mut self.table,
        }
    }
}
