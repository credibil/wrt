use anyhow::{Result, anyhow};
use res_azkeyvault::AzKeyVault;
use res_mongodb::MongoDb;
// use messaging_nats::Nats;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_blobstore_mdb::Blobstore;
use wasi_http::Http;
use wasi_keyvalue::WasiKeyValue;
use wasi_messaging::WasiMessaging;
use wasi_otel::Otel;
use wasi_vault_az::Vault;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let (mongodb, az_secret) = tokio::try_join!(MongoDb::new(), AzKeyVault::new())?;

    let nats_client = messaging_nats::Nats::new().connect().await?;
    let messaging = WasiMessaging;
    let messaging = messaging.client(nats_client).await;

    let nats_client = messaging_nats::Nats::new().connect().await?;
    let keyvalue = WasiKeyValue;
    // let keyvalue = keyvalue.client(nats_client).await;

    let runtime = builder
        .register(Otel)
        .register(Http)
        .register(Blobstore.resource(mongodb)?)
        // .register(keyvalue)
        .register(Vault.resource(az_secret)?)
        .register(messaging)
        .build();

    runtime.await
}
