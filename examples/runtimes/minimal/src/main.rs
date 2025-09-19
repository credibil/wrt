use anyhow::{Result, anyhow};
use res_nats::Nats;
use runtime::{AddResource, Cli, Command, Parser, ResourceBuilder, Runtime};
use wasi_http::Http;
use wasi_messaging_nats::Messaging;
use wasi_otel::Otel;

#[tokio::main]
async fn main() -> Result<()> {
    let Command::Run { wasm } = Cli::parse().command else {
        return Err(anyhow!("No command provided"));
    };

    let nats = Nats::new().await?;
    Runtime::new(wasm).register(Otel).register(Http).register(Messaging.resource(nats)?).await
}
