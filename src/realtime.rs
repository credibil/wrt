#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use res_redis::Redis;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, RuntimeBuilder};
use wasi_http::WasiHttp;
use wasi_keyvalue::WasiKeyValue;
use wasi_otel::WasiOtel;

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

    let redis = Redis::new().await?;
    let keyvalue = WasiKeyValue.resource(redis).await?;

    let runtime = builder
        .register(WasiHttp)
        .register(WasiOtel)
        .register(keyvalue)
        .build();

    runtime.await
}
