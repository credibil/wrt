#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use runtime::{Cli, Command, Parser, RuntimeBuilder};
use wasi_http::WasiHttp;
use wasi_websockets::WebSockets;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    let builder = RuntimeBuilder::new(wasm, true);
    tracing::info!("Tracing initialised, logging available");

    let runtime = builder.register(WasiHttp).register(WebSockets).build();

    runtime.await
}
