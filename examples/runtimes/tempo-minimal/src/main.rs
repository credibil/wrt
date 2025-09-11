use anyhow::Result;
use runtime::{Cli, Command, Parser, Runtime};
use wasi_http::Http;
use wasi_otel::Otel;

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => Runtime::new(wasm).register(Otel).register(Http).await,
        #[cfg(feature = "compile")]
        Command::Compile { wasm, output } => runtime::compile(&wasm, output),
    }
}
