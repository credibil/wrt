#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_mongodb::MongoDb;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_blobstore::WasiBlobstore;
use wasi_http::WasiHttp;
use wasi_otel::WasiOtel;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Server { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let mongodb = MongoDb::new().await?;
    let blobstore = WasiBlobstore.resource(mongodb).await?;

    let runtime = builder.register(WasiOtel).register(WasiHttp).register(blobstore).build();
    runtime.await
}
