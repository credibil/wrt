#![feature(prelude_import)]
//! WRT runtime CLI entry point.
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;

use anyhow::Result;
use runtime::{Cli, Command, Parser, Resource, Server};
use wasi_http::WasiHttpCtx;
use wasi_identity::DefaultIdentity;
use wasi_otel::DefaultOtel;
/// Runtime state holding pre-instantiated components and backend connections.
struct RuntimeContext {
    instance_pre: wasmtime::component::InstancePre<RuntimeStoreCtx>,
    defaultotelctx: DefaultOtel,
    defaultidentity: DefaultIdentity,
}
#[automatically_derived]
impl ::core::clone::Clone for RuntimeContext {
    #[inline]
    fn clone(&self) -> RuntimeContext {
        RuntimeContext {
            instance_pre: ::core::clone::Clone::clone(&self.instance_pre),
            defaultotelctx: ::core::clone::Clone::clone(&self.defaultotelctx),
            defaultidentity: ::core::clone::Clone::clone(&self.defaultidentity),
        }
    }
}
impl RuntimeContext {
    /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
    async fn new(compiled: &mut runtime::Compiled<RuntimeStoreCtx>) -> anyhow::Result<Self> {
        compiled.link(wasi_identity::WasiIdentity)?;
        compiled.link(wasi_http::WasiHttp)?;
        compiled.link(wasi_otel::WasiOtel)?;
        Ok(Self {
            instance_pre: compiled.pre_instantiate()?,
            defaultotelctx: DefaultOtel::connect().await?,
            defaultidentity: DefaultIdentity::connect().await?,
        })
    }

    /// Starts all enabled server interfaces (HTTP, messaging, websockets).
    async fn start(&self) -> anyhow::Result<()> {
        use futures::future::{BoxFuture, try_join_all};
        let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> =
            <[_]>::into_vec(::alloc::boxed::box_new([Box::pin(wasi_http::WasiHttp.run(self))]));
        try_join_all(futures).await?;
        Ok(())
    }
}
impl runtime::State for RuntimeContext {
    type StoreCtx = RuntimeStoreCtx;

    fn instance_pre(&self) -> &wasmtime::component::InstancePre<Self::StoreCtx> {
        &self.instance_pre
    }

    fn new_store(&self) -> Self::StoreCtx {
        use wasmtime_wasi::{ResourceTable, WasiCtxBuilder};
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_args()
            .inherit_env()
            .inherit_stdin()
            .stdout(tokio::io::stdout())
            .stderr(tokio::io::stderr())
            .build();
        RuntimeStoreCtx {
            table: ResourceTable::new(),
            wasi: wasi_ctx,
            identity: self.defaultidentity.clone(),
            http: wasi_http::WasiHttpCtx,
            otel: self.defaultotelctx.clone(),
        }
    }
}
/// Per-instance data shared between the WebAssembly runtime and host functions.
pub struct RuntimeStoreCtx {
    pub table: wasmtime_wasi::ResourceTable,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub identity: DefaultIdentity,
    pub http: wasi_http::WasiHttpCtx,
    pub otel: DefaultOtel,
}
impl wasmtime_wasi::WasiView for RuntimeStoreCtx {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
impl wasi_identity::WasiIdentityView for RuntimeStoreCtx {
    fn identity(&mut self) -> wasi_identity::WasiIdentityCtxView<'_> {
        wasi_identity::WasiIdentityCtxView {
            ctx: &mut self.identity,
            table: &mut self.table,
        }
    }
}
impl wasi_http::WasiHttpView for RuntimeStoreCtx {
    fn http(&mut self) -> wasi_http::WasiHttpCtxView<'_> {
        wasi_http::WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
        }
    }
}
impl wasi_otel::WasiOtelView for RuntimeStoreCtx {
    fn otel(&mut self) -> wasi_otel::WasiOtelCtxView<'_> {
        wasi_otel::WasiOtelCtxView {
            ctx: &mut self.otel,
            table: &mut self.table,
        }
    }
}
/// Run the specified wasm guest using the configured runtime.
pub async fn runtime_run(wasm: std::path::PathBuf) -> anyhow::Result<()> {
    use anyhow::Context as _;
    runtime_cli::RuntimeConfig::from_wasm(&wasm)?;
    let mut compiled = runtime::Runtime::new().build(&wasm).with_context(|| {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("compiling {0}", wasm.display()))
        })
    })?;
    let run_state = RuntimeContext::new(&mut compiled).await.context("preparing runtime state")?;
    run_state.start().await.context("starting runtime services")
}
fn main() -> Result<()> {
    let body = async {
        match Cli::parse().command {
            Command::Run { wasm } => runtime_run(wasm).await,
            Command::Env => runtime_cli::env::print(),
            Command::Compile { wasm, output } => runtime::compile(&wasm, output),
        }
    };
    #[allow(
        clippy::expect_used,
        clippy::diverging_sub_expression,
        clippy::needless_return,
        clippy::unwrap_in_result
    )]
    {
        return tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body);
    }
}
