#![feature(prelude_import)]
//! WRT runtime CLI entry point.
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use anyhow::Result;
use runtime::{Cli, Command, Parser};
use wasi_http::DefaultHttp;
use wasi_http::WasiHttp;
use wasi_identity::{DefaultIdentity, WasiIdentity};
use wasi_otel::{DefaultOtel, WasiOtel};
use anyhow::Context as _;
use futures::future::{BoxFuture, try_join_all};
use runtime::{Resource, Server};
use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
use wasmtime::component::InstancePre;
/// Run the specified wasm guest using the configured runtime.
pub async fn runtime_run(wasm: std::path::PathBuf) -> anyhow::Result<()> {
    runtime_cli::RuntimeConfig::from_wasm(&wasm)?;
    let mut compiled = runtime::Runtime::new()
        .build(&wasm)
        .with_context(|| ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("compiling {0}", wasm.display()))
        }))?;
    let run_state = Context::new(&mut compiled)
        .await
        .context("preparing runtime state")?;
    run_state.start().await.context("starting runtime services")
}
/// Runtime state holding pre-instantiated components and backend connections.
struct Context {
    instance_pre: InstancePre<StoreCtx>,
    defaultotel: DefaultOtel,
    defaultidentity: DefaultIdentity,
    defaulthttp: DefaultHttp,
}
#[automatically_derived]
impl ::core::clone::Clone for Context {
    #[inline]
    fn clone(&self) -> Context {
        Context {
            instance_pre: ::core::clone::Clone::clone(&self.instance_pre),
            defaultotel: ::core::clone::Clone::clone(&self.defaultotel),
            defaultidentity: ::core::clone::Clone::clone(&self.defaultidentity),
            defaulthttp: ::core::clone::Clone::clone(&self.defaulthttp),
        }
    }
}
impl Context {
    /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
    async fn new(compiled: &mut runtime::Compiled<StoreCtx>) -> anyhow::Result<Self> {
        compiled.link(WasiHttp)?;
        compiled.link(WasiOtel)?;
        compiled.link(WasiIdentity)?;
        Ok(Self {
            instance_pre: compiled.pre_instantiate()?,
            defaultotel: DefaultOtel::connect().await?,
            defaultidentity: DefaultIdentity::connect().await?,
            defaulthttp: DefaultHttp::connect().await?,
        })
    }
    /// Starts all enabled server interfaces (HTTP, messaging, websockets).
    async fn start(&self) -> anyhow::Result<()> {
        {
            ::std::io::_print(format_args!("Starting server interfaces...\n"));
        };
        let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = <[_]>::into_vec(
            ::alloc::boxed::box_new([Box::pin(wasi_http::WasiHttp.run(self))]),
        );
        try_join_all(futures).await?;
        Ok(())
    }
}
impl runtime::State for Context {
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
            WasiHttp: self.defaulthttp.clone(),
            WasiOtel: self.defaultotel.clone(),
            WasiIdentity: self.defaultidentity.clone(),
        }
    }
}
/// Per-instance data shared between the WebAssembly runtime and host functions.
pub struct StoreCtx {
    pub table: wasmtime_wasi::ResourceTable,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub WasiHttp: DefaultHttp,
    pub WasiOtel: DefaultOtel,
    pub WasiIdentity: DefaultIdentity,
}
impl wasmtime_wasi::WasiView for StoreCtx {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
impl wasi_http::WasiHttpView for StoreCtx {
    fn http(&mut self) -> wasi_http::WasiHttpCtxView<'_> {
        wasi_http::WasiHttpCtxView {
            ctx: &mut self.WasiHttp,
            table: &mut self.table,
        }
    }
}
impl wasi_otel::WasiOtelView for StoreCtx {
    fn otel(&mut self) -> wasi_otel::WasiOtelCtxView<'_> {
        wasi_otel::WasiOtelCtxView {
            ctx: &mut self.WasiOtel,
            table: &mut self.table,
        }
    }
}
impl wasi_identity::WasiIdentityView for StoreCtx {
    fn identity(&mut self) -> wasi_identity::WasiIdentityCtxView<'_> {
        wasi_identity::WasiIdentityCtxView {
            ctx: &mut self.WasiIdentity,
            table: &mut self.table,
        }
    }
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
