#![cfg(not(target_arch = "wasm32"))]
// #![cfg(all(feature = "http", feature = "identity", feature = "otel"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use kernel::{Cli, Command, Parser};
use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxImpl as BlobstoreDefault};
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_identity::{WasiIdentity, WasiIdentityCtxImpl as IdentityDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

// Generate runtime infrastructure for the credibil feature set
buildgen::runtime!({
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiIdentity: IdentityDefault,

});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        _ => unreachable!(),
    }
}
