use anyhow::{Result, anyhow};
use res_mongodb::MongoDb;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, Runtime};
use wasi_blobstore_mdb::Blobstore;
use wasi_http::Http;
use wasi_otel::Otel;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let mongodb = MongoDb::new().await?;
    Runtime::new(wasm).register(Otel).register(Http).register(Blobstore.resource(mongodb)?).await
}
