#![cfg(not(target_arch = "wasm32"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use runtime::{Cli, Command, Parser};
// Import backend types for the credibil feature set
// use res_azure::Client as Azure;
// use res_mongodb::Client as MongoDb;
// use res_nats::Client as Nats;
// use wasi_http::{DefaultHttp, WasiHttp};
// use wasi_identity::{DefaultIdentity, WasiIdentity};
use wasi_otel::{DefaultOtel, WasiOtel};

// Generate runtime infrastructure for the credibil feature set
buildgen::runtime!({
    // WasiHttp: DefaultHttp,
    WasiOtel: DefaultOtel,
    // WasiIdentity: DefaultIdentity,
    // "keyvalue": "default",
    // "messaging": "nats",
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime_run(wasm).await,
        Command::Env => runtime_cli::env::print(),
        #[cfg(feature = "jit")]
        Command::Compile { wasm, output } => runtime::compile(&wasm, output),
    }
}
