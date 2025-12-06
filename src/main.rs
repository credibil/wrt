#![cfg(not(target_arch = "wasm32"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use kernel::{Cli, Command, Parser};
// Import backend types for the credibil feature set
// use be_azure::Client as Azure;
// use be_mongodb::Client as MongoDb;
// use be_nats::Client as Nats;
use wasi_http::{WasiHttp, WasiHttpCtxImpl};
use wasi_identity::{WasiIdentity, WasiIdentityCtxImpl};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl};

// Generate runtime infrastructure for the credibil feature set
buildgen::runtime!({
    WasiHttp: WasiHttpCtxImpl,
    WasiOtel: WasiOtelCtxImpl,
    WasiIdentity: WasiIdentityCtxImpl,
    
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        #[cfg(feature = "jit")]
        Command::Compile { wasm, output } => kernel::compile(&wasm, output),
    }
}
