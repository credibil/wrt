#![cfg(not(target_arch = "wasm32"))]

//! WRT runtime CLI entry point.

use anyhow::{Result, anyhow};
use runtime::{Cli, Command, Parser};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("only run command is supported"));
    };

    // if let Command::Env = Cli::parse().command {
    runtime_cli::config::print();
    // }

    runtime_cli::run(wasm).await
}
