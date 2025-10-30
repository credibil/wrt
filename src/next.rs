#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_nats_next::NatsClient;
// use wasmtime_wasi_http::p3::DefaultWasiHttpCtx;
use runtime::Runner;
use runtime::http_ctx::HttpCtx;
use runtime::{Cli, Command, Parser};
use tokio::io;
use wasi_http_next::{WasiHttpCtxView, WasiHttpView};
use wasi_keyvalue_next::{WasiKeyValueCtxView, WasiKeyValueView};
use wasmtime::component::InstancePre;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // link all dependencies
    let mut linker = runtime::RuntimeNext::new(wasm).init::<RunData>()?;
    wasi_keyvalue_next::add_to_linker::<RunData>(&mut linker)?;
    wasi_http_next::add_to_linker::<RunData>(&mut linker)?;

    // start servers
    // get instance pre
    // let instance_pre = linker.instantiate_pre_async().await?;
    // wasi_http_next::WasiHttp::serve();

    Ok(())
}

#[derive(Clone)]
pub struct Initializer {
    pre: InstancePre<RunData>,
    nats_client: NatsClient,
}

impl Runner for Initializer {
    type StoreData = RunData;

    fn instance_pre(&self) -> InstancePre<Self::StoreData> {
        self.pre.clone()
    }

    fn new_data(&self) -> Self::StoreData {
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
            http_ctx: HttpCtx,
            keyvalue_ctx: self.nats_client.clone(),
        }
    }
}

/// `RunData` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunData {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    pub http_ctx: HttpCtx,
    pub keyvalue_ctx: NatsClient,
}

impl RunData {
    /// Create a new [`RunData`] instance.
    #[must_use]
    pub async fn new() -> Result<Self> {
        let mut ctx = WasiCtxBuilder::new();
        let wasi_ctx = ctx
            .inherit_args()
            .inherit_env()
            .inherit_stdin()
            .stdout(io::stdout())
            .stderr(io::stderr())
            .build();

        // TODO: pass already connected NATS client
        Ok(Self {
            table: ResourceTable::new(),
            wasi_ctx,
            http_ctx: HttpCtx,
            keyvalue_ctx: NatsClient::connect().await?,
        })
    }
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

impl WasiKeyValueView for RunData {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        WasiKeyValueCtxView {
            ctx: &mut self.keyvalue_ctx,
            table: &mut self.table,
        }
    }
}
