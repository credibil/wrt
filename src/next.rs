#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_nats_next::Client as Nats;
use runtime::http_ctx::HttpCtx;
use runtime::{Cli, Command, Host, Parser, Resource, Server, State};
use tokio::io;
use wasi_http_next::{WasiHttp, WasiHttpCtxView, WasiHttpView};
use wasi_keyvalue_next::{WasiKeyValue, WasiKeyValueCtxView, WasiKeyValueView};
use wasmtime::component::InstancePre;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Server { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // link all dependencies
    let (mut linker, component) = runtime::RuntimeNext::new(wasm).init::<RunData>()?;
    WasiKeyValue::add_to_linker(&mut linker)?;
    WasiHttp::add_to_linker(&mut linker)?;

    // instantiate state
    let run_state = RunState {
        instance_pre: linker.instantiate_pre(&component)?,
        nats_client: Nats::connect().await?,
    };

    // run server(s)
    WasiHttp.run(&run_state).await?;

    Ok(())
}

#[derive(Clone)]
pub struct RunState {
    instance_pre: InstancePre<RunData>,
    nats_client: Nats,
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
    pub keyvalue_ctx: Nats,
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
