#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_azkeyvault_next::Client as AzKeyVaultCtx;
use res_mongodb_next::Client as MongoDbCtx;
use res_nats_next::Client as NatsCtx;
use runtime::http_ctx::HttpCtx;
use runtime::{Cli, Command, Parser, Resource, RuntimeNext, Server, State};
use tokio::io;
use wasi_blobstore_next::{WasiBlobstore, WasiBlobstoreCtxView, WasiBlobstoreView};
use wasi_http_next::{WasiHttp, WasiHttpCtxView, WasiHttpView};
use wasi_keyvalue_next::{WasiKeyValue, WasiKeyValueCtxView, WasiKeyValueView};
use wasi_otel_next::{DefaultOtelCtx, WasiOtel, WasiOtelCtxView, WasiOtelView};
use wasi_vault_next::{WasiVault, WasiVaultCtxView, WasiVaultView};
use wasmtime::component::InstancePre;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // link dependencies
    let mut rt = RuntimeNext::<RunData>::new(wasm).compile()?;
    rt.link(WasiHttp)?;
    rt.link(WasiOtel)?;
    rt.link(WasiBlobstore)?;
    rt.link(WasiKeyValue)?;
    rt.link(WasiVault)?;

    let instance_pre = rt.pre_instantiate()?;

    // prepare state
    let run_state = RunState {
        instance_pre,
        nats_client: NatsCtx::connect().await?,
        mongodb_client: MongoDbCtx::connect().await?,
        vault_client: AzKeyVaultCtx::connect().await?,
    };

    // run server(s)
    WasiHttp.run(&run_state).await?;

    Ok(())
}

#[derive(Clone)]
pub struct RunState {
    instance_pre: InstancePre<RunData>,
    nats_client: NatsCtx,
    mongodb_client: MongoDbCtx,
    vault_client: AzKeyVaultCtx,
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
            otel_ctx: DefaultOtelCtx,
            keyvalue_ctx: self.nats_client.clone(),
            blobstore_ctx: self.mongodb_client.clone(),
            vault_ctx: self.vault_client.clone(),
        }
    }
}

/// `RunData` is used to share host state between the Wasm runtime and hosts
/// each time they are instantiated.
pub struct RunData {
    pub table: ResourceTable,
    pub wasi_ctx: WasiCtx,
    pub http_ctx: HttpCtx,
    pub otel_ctx: DefaultOtelCtx,
    pub keyvalue_ctx: NatsCtx,
    pub blobstore_ctx: MongoDbCtx,
    pub vault_ctx: AzKeyVaultCtx,
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

impl WasiKeyValueView for RunData {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        WasiKeyValueCtxView {
            ctx: &mut self.keyvalue_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiBlobstoreView for RunData {
    fn blobstore(&mut self) -> WasiBlobstoreCtxView<'_> {
        WasiBlobstoreCtxView {
            ctx: &mut self.blobstore_ctx,
            table: &mut self.table,
        }
    }
}

impl WasiVaultView for RunData {
    fn vault(&mut self) -> WasiVaultCtxView<'_> {
        WasiVaultCtxView {
            ctx: &mut self.vault_ctx,
            table: &mut self.table,
        }
    }
}
