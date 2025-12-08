#![cfg(not(target_arch = "wasm32"))]

//! Realtime runtime CLI entry point.

use anyhow::Result;
use be_azure::Client as Azure;
use be_kafka::Client as Kafka;
use be_opentelemetry::Client as OpenTelemetry;
use be_redis::Client as Redis;
use kernel::{Cli, Command, Parser, tokio};
use wasi_http::{WasiHttp, WasiHttpCtxImpl};
use wasi_identity::WasiIdentity;
use wasi_keyvalue::WasiKeyValue;
use wasi_messaging::WasiMessaging;
use wasi_otel::WasiOtel;
use wasi_vault::WasiVault;

// Generate runtime
buildgen::runtime!({
    WasiHttp: WasiHttpCtxImpl,
    WasiOtel: OpenTelemetry,
    WasiIdentity: Azure,
    WasiKeyValue: Redis,
    WasiMessaging: Kafka,
    WasiVault: Azure,
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        _ => unreachable!(),
    }
}
