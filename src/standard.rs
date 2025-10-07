use anyhow::{Result, anyhow};
use res_azkeyvault::AzKeyVault;
use res_mongodb::MongoDb;
use res_nats::Nats;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_blobstore_mdb::Blobstore;
use wasi_http::Http;
use wasi_keyvalue_nats::KeyValue;
use wasi_messaging_nats::Messaging;
use wasi_otel::Otel;
use wasi_vault_az::Vault;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let (mongodb, az_secret, nats) =
        tokio::try_join!(MongoDb::new(), AzKeyVault::new(), Nats::new())?;

    let runtime = builder
        .register(Otel)
        .register(Http)
        .register(Blobstore.resource(mongodb)?)
        .register(KeyValue.resource(nats.clone())?)
        .register(Vault.resource(az_secret)?)
        .register(Messaging.resource(nats)?)
        .build();

    runtime.await
}
