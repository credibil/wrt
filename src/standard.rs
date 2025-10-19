use anyhow::{Result, anyhow};
use res_azkeyvault::AzKeyVault;
use res_mongodb::MongoDb;
use res_nats::Nats;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_blobstore::WasiBlobstore;
use wasi_http::WasiHttp;
use wasi_keyvalue::WasiKeyValue;
use wasi_messaging::WasiMessaging;
use wasi_otel::WasiOtel;
use wasi_vault_az::Vault;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let (mongodb, nats, az_secret) =
        tokio::try_join!(MongoDb::new(), Nats::new(), AzKeyVault::new())?;

    let messaging = WasiMessaging.client(nats.clone()).await;
    let keyvalue = WasiKeyValue.client(nats).await;
    let blobstore = WasiBlobstore.client(mongodb).await;

    let runtime = builder
        .register(WasiOtel)
        .register(WasiHttp)
        .register(blobstore)
        .register(keyvalue)
        .register(messaging)
        .register(Vault.resource(az_secret)?)
        .build();

    runtime.await
}
