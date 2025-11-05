#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_kafka::Client as KafkaCtx;
use runtime::{Cli, Command, Parser, Resource, Runtime, Server, State};
use tokio::{io, try_join};
use wasi_http::{DefaultWasiHttpCtx, WasiHttp, WasiHttpCtxView, WasiHttpView};
use wasi_messaging::{WasiMessaging, WasiMessagingCtxView, WasiMessagingView};
use wasi_otel::{DefaultOtelCtx, WasiOtel, WasiOtelCtxView, WasiOtelView};
use wasi_websockets::{DefaultWebSocketsCtx, WasiWebSockets, WasiWebSocketsView, WebSocketsView};
use wasmtime::component::InstancePre;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // compile and link dependencies
    let mut compiled = Runtime::<RunData>::new(wasm).compile()?;
    compiled.link(WasiHttp)?;
    compiled.link(WasiOtel)?;
    compiled.link(WasiMessaging)?;
    compiled.link(WasiWebSockets)?;

    // prepare state
    let run_state = RunState {
        instance_pre: compiled.pre_instantiate()?,
        kafka_client: KafkaCtx::connect().await?,
    };

    // run server(s)
    try_join!(WasiHttp.run(&run_state), WasiWebSockets.run(&run_state))?;

    Ok(())
}

#[derive(Clone)]
pub struct RunState {
    instance_pre: InstancePre<RunData>,
    kafka_client: KafkaCtx,
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
            http_ctx: DefaultWasiHttpCtx,
            otel_ctx: DefaultOtelCtx,
            messaging_ctx: self.kafka_client.clone(),
            websockets_ctx: DefaultWebSocketsCtx,
        }
    }
}

/// `RunData` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunData {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    pub http_ctx: DefaultWasiHttpCtx,
    pub otel_ctx: DefaultOtelCtx,
    pub messaging_ctx: KafkaCtx,
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

impl WasiHttpView for RunData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http_ctx,
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

impl WasiOtelView for RunData {
    fn otel(&mut self) -> WasiOtelCtxView<'_> {
        WasiOtelCtxView {
            ctx: &mut self.otel_ctx,
            table: &mut self.table,
        }
    }
}

impl WebSocketsView for RunData {
    fn start(&mut self) -> WasiWebSocketsView<'_> {
        WasiWebSocketsView {
            ctx: &mut self.websockets_ctx,
            table: &mut self.table,
        }
    }
}
