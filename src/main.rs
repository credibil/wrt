#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Result, anyhow};
use runtime::{Cli, Command, Parser};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };
    runtime_cli::run(wasm).await
}
