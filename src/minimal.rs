use anyhow::{Result, anyhow};
use res_mongodb::MongoDb;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_blobstore_mdb::Blobstore;
use wasi_http::Http;
use wasi_otel::Otel;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let mongodb = MongoDb::new().await?;
    let runtime =
        builder.register(Otel).register(Http).register(Blobstore.resource(mongodb)?).build();
    runtime.await
}
