use anyhow::{Result, anyhow};
use runtime::{Cli, Command, Parser, Runtime};
use wasi_http::Http;
use wasi_otel::Otel;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("No command provided"));
    };
    Runtime::new(wasm).register(Otel).register(Http).await
}
