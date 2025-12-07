#![cfg(not(target_arch = "wasm32"))]

use anyhow::Result;
use kernel::{Cli, Command, Parser};
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
use wasi_websockets::{WasiWebSockets, WebSocketsCtxImpl as WebSocketsDefault};

buildgen::runtime!({
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiWebSockets: WebSocketsDefault,
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        _ => unreachable!(),
    }
}
