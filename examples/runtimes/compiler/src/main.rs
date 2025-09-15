use anyhow::{Result, anyhow};
use runtime::{Cli, Command, Parser};

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Compile { wasm, output } = Cli::parse().command else {
        return Err(anyhow!("This binary only supports the compile command"));
    };
    runtime::compile(&wasm, output)
}
