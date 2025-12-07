#![cfg(not(target_arch = "wasm32"))]

use anyhow::Result;
use kernel::{Cli, Command, Parser};
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxImpl as KeyValueDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

buildgen::runtime!({
    WasiHttp: HttpDefault,
    WasiKeyValue: KeyValueDefault,
    WasiOtel: OtelDefault,
});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        _ => unreachable!(),
    }
}
