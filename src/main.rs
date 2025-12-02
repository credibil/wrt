#![cfg(not(target_arch = "wasm32"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use runtime::{Cli, Command, Parser};
use wasi_http::WasiHttpCtx;
// Import backend types for the credibil feature set
#[cfg(feature = "credibil")]
// use res_azure::Client as Azure;
// use res_mongodb::Client as MongoDb;
// use res_nats::Client as Nats;
use wasi_identity::DefaultIdentity;
use wasi_otel::DefaultOtelCtx;

// Generate runtime infrastructure for the credibil feature set
wrt_codegen::runtime!({
    "http": WasiHttpCtx,
    "otel": DefaultOtelCtx,
    "identity": DefaultIdentity,
    // "keyvalue": Nats,
    // "messaging": Nats,
    // "vault": Azure
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        #[cfg(feature = "credibil")]
        Command::Run { wasm } => runtime_run(wasm).await,
        #[cfg(not(feature = "credibil"))]
        Command::Run { .. } => {
            anyhow::bail!(
                "This binary was not built with the 'credibil' feature set. Please rebuild with --features credibil"
            )
        }
        Command::Env => runtime_cli::env::print(),
        #[cfg(feature = "jit")]
        Command::Compile { wasm, output } => runtime::compile(&wasm, output),
    }
}
