#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_azkeyvault::AzKeyVault;
use res_mongodb::MongoDb;
use res_nats::Nats;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_blobstore::WasiBlobstore;
use wasi_http::WasiHttp;
// use wasi_keyvalue::WasiKeyValue;
use wasi_messaging::WasiMessaging;
use wasi_otel::WasiOtel;
use wasi_vault::WasiVault;

#[tokio::main]
async fn main() -> Result<()> {
    // build_macro::buildgen! ({
    //     messaging: [Nats],
    //     keyvalue: [Nats],
    //     blobstore: [MongoDb],
    //     vault: [AzKeyVault],
    // });

    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let (mongodb, nats, az_vault) =
        tokio::try_join!(MongoDb::new(), Nats::new(), AzKeyVault::new())?;

    let messaging = WasiMessaging.resource(nats.clone()).await?;
    // let keyvalue = WasiKeyValue.resource(nats).await?;
    let blobstore = WasiBlobstore.resource(mongodb).await?;
    let vault = WasiVault.resource(az_vault).await?;

    let runtime = builder
        .register(WasiOtel)
        .register(WasiHttp)
        .register(blobstore)
        // .register(keyvalue)
        .register(messaging)
        .register(vault)
        .build();

    runtime.await
}




use tokio::io;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

/// `RunState` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunState {
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
    pub http_ctx: WasiHttpCtx,
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
            http_ctx: WasiHttpCtx::new(),
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
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
