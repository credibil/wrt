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
use wasi_vault::WasiVault;

#[tokio::main]
async fn main() -> Result<()> {
    // let Command::Run { wasm } = Cli::parse().command else {
    //     return Err(anyhow!("only run command is supported"));
    // };
    // let builder = RuntimeBuilder::new(wasm, true);
    // tracing::info!("Tracing initialised, logging available");

    // let (mongodb, nats, az_vault) =
    //     tokio::try_join!(MongoDb::new(), Nats::new(), AzKeyVault::new())?;

    // let messaging = WasiMessaging.resource(nats.clone()).await?;
    // let keyvalue = WasiKeyValue.resource(nats).await?;
    // let blobstore = WasiBlobstore.resource(mongodb).await?;
    // let vault = WasiVault.resource(az_vault).await?;

    // let runtime = builder
    //     .register(WasiOtel)
    //     .register(WasiHttp)
    //     .register(blobstore)
    //     .register(keyvalue)
    //     .register(messaging)
    //     .register(vault)
    //     .build();

    // runtime.await;

    build_macro::buildgen! ({
        messaging: [Nats],
        keyvalue: [Nats],
        blobstore: [MongoDb],
        vault: [AzKeyVault],
    });
}
