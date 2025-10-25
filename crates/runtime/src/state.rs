//! # WebAssembly Runtime

use tokio::io;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::p3::{WasiHttpCtx, WasiHttpCtxView, WasiHttpView};

/// `RunState` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunState {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    pub http_ctx: HttpCtx,
}

impl Default for RunState {
    fn default() -> Self {
        Self::new()
    }
}

impl RunState {
    /// Create a new [`RunState`] instance.
    #[must_use]
    pub fn new() -> Self {
        let mut ctx = WasiCtxBuilder::new();
        ctx.inherit_args();
        ctx.inherit_env();
        ctx.inherit_stdin();
        ctx.stdout(io::stdout());
        ctx.stderr(io::stderr());

        Self {
            table: ResourceTable::default(),
            wasi_ctx: ctx.build(),
            http_ctx: HttpCtx {},
        }
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

impl WasiHttpView for RunState {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            table: &mut self.table,
            ctx: &mut self.http_ctx,
        }
    }
}

pub struct HttpCtx;
impl WasiHttpCtx for HttpCtx {}
