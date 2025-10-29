#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_nats_next::NatsClient;
use runtime::{Cli, Command, Parser};
use tokio::io;

use wasi_keyvalue_next::{WasiKeyValueCtxView, WasiKeyValueView};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // let keyvalue = WasiKeyValue.resource(Nats::new().await?).await?;
    // RuntimeBuilder::new(wasm, true).register(WasiHttp).register(keyvalue).build().await

    let runstate = RunState::new();

    todo!()
}

/// `RunState` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunState {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    pub keyvalue_ctx: NatsClient,
}

impl RunState {
    /// Create a new [`RunState`] instance.
    #[must_use]
    pub async fn new() -> Result<Self> {
        let mut ctx = WasiCtxBuilder::new();
        ctx.inherit_args();
        ctx.inherit_env();
        ctx.inherit_stdin();
        ctx.stdout(io::stdout());
        ctx.stderr(io::stderr());

        Ok(Self {
            table: ResourceTable::default(),
            wasi_ctx: ctx.build(),
            keyvalue_ctx: NatsClient::connect().await?,
        })
    }
}

impl WasiView for RunState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiKeyValueView for RunState {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        WasiKeyValueCtxView {
            ctx: &mut self.keyvalue_ctx,
            table: &mut self.table,
        }
    }
}
