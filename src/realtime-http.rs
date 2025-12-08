#![cfg(not(target_arch = "wasm32"))]

//! Realtime runtime CLI entry point.

use anyhow::Result;
use be_opentelemetry::Client as OpenTelemetry;
use be_redis::Client as Redis;
use kernel::{Cli, Command, Parser, tokio};
use wasi_http::{WasiHttp, WasiHttpCtxImpl};
use wasi_keyvalue::WasiKeyValue;
use wasi_otel::WasiOtel;

// Generate runtime
buildgen::runtime!({
    WasiHttp: WasiHttpCtxImpl,
    WasiOtel: OpenTelemetry,
    WasiKeyValue: Redis,
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        _ => unreachable!(),
    }
}
