#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_kafka::{Client as KafkaCtx, ConnectOptions as KafkaConfig};
use res_postgres::{Client as PostgresCtx, ConnectOptions as PostgresConfig};
use res_redis::{Client as RedisCtx, ConnectOptions as RedisConfig};
use runtime::{Cli, Command, FromEnv, Parser, Resource, Runtime, Server, State};
use tokio::{io, try_join};
use wasi_http::{WasiHttp, WasiHttpCtx, WasiHttpCtxView, WasiHttpView};
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxView, WasiKeyValueView};
use wasi_messaging::{WasiMessaging, WasiMessagingCtxView, WasiMessagingView};
use wasi_otel::{DefaultOtelCtx, WasiOtel, WasiOtelCtxView, WasiOtelView};
use wasi_sql::{WasiSql, WasiSqlCtxView, WasiSqlView};
use wasmtime::component::InstancePre;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // environment variables
    let kafka_options = <KafkaConfig as FromEnv>::from_env()?;
    let redis_options = <RedisConfig as FromEnv>::from_env()?;
    let postgres_options = <PostgresConfig as FromEnv>::from_env()?;

    // link dependencies
    let mut rt = Runtime::<RunData>::new(wasm).compile()?;
    rt.link(WasiHttp)?;
    rt.link(WasiOtel)?;
    rt.link(WasiMessaging)?;
    rt.link(WasiKeyValue)?;
    rt.link(WasiSql)?;

    let instance_pre = rt.pre_instantiate()?;

    // prepare state
    let run_state = RunState {
        instance_pre,
        kafka_client: KafkaCtx::connect_with(kafka_options).await?,
        redis_client: RedisCtx::connect_with(redis_options).await?,
        postgres_client: PostgresCtx::connect_with(postgres_options).await?,
    };

    // run server(s)
    try_join!(WasiHttp.run(&run_state), WasiMessaging.run(&run_state))?;

    Ok(())
}

#[derive(Clone)]
pub struct RunState {
    instance_pre: InstancePre<RunData>,
    kafka_client: KafkaCtx,
    redis_client: RedisCtx,
    postgres_client: PostgresCtx,
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
            .stdout(io::stdout())
            .stderr(io::stderr())
            .build();

        RunData {
            table: ResourceTable::new(),
            wasi_ctx,
            http_ctx: WasiHttpCtx,
            otel_ctx: DefaultOtelCtx,
            messaging_ctx: self.kafka_client.clone(),
            keyvalue_ctx: self.redis_client.clone(),
            sql_ctx: self.postgres_client.clone(),
        }
    }
}

/// `RunData` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunData {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
    pub otel_ctx: DefaultOtelCtx,
    pub messaging_ctx: KafkaCtx,
    pub keyvalue_ctx: RedisCtx,
    pub sql_ctx: PostgresCtx,
}

impl WasiView for RunData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for RunData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiOtelView for RunData {
    fn otel(&mut self) -> WasiOtelCtxView<'_> {
        WasiOtelCtxView {
            ctx: &mut self.otel_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiMessagingView for RunData {
    fn messaging(&mut self) -> WasiMessagingCtxView<'_> {
        WasiMessagingCtxView {
            ctx: &mut self.messaging_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiKeyValueView for RunData {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        WasiKeyValueCtxView {
            ctx: &mut self.keyvalue_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiSqlView for RunData {
    fn sql(&mut self) -> WasiSqlCtxView<'_> {
        WasiSqlCtxView {
            ctx: &mut self.sql_ctx,
            table: &mut self.table,
        }
    }
}
