#![cfg(not(target_arch = "wasm32"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use kernel::{Cli, Command, Parser};
// Import backend types for the credibil feature set
// use res_azure::Client as Azure;
// use res_mongodb::Client as MongoDb;
// use res_nats::Client as Nats;
// use wasi_http::{DefaultHttp as WasiHttpCtxImpl, WasiHttp};
// use wasi_identity::{DefaultIdentity as WasiIdentityCtxImpl, WasiIdentity};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl};

// Generate runtime infrastructure for the credibil feature set
buildgen::runtime!({
    // WasiHttp: WasiHttpCtxImpl,
    WasiOtel: WasiOtelCtxImpl,
    // WasiIdentity: WasiIdentityCtxImpl,
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        #[cfg(feature = "jit")]
        Command::Compile { wasm, output } => kernel::compile(&wasm, output),
    }
}
