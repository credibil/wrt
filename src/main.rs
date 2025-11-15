#![cfg(not(target_arch = "wasm32"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use runtime::{Cli, Command, Parser};

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime_cli::run(wasm).await,
        Command::Env => runtime_cli::env::print(),
        #[cfg(feature = "jit")]
        Command::Compile { wasm, output } => runtime::compile(&wasm, output),
    }
}
