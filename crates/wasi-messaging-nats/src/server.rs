use async_nats::Message;
use futures::stream::{self, StreamExt};
use runtime::RunState;
use tracing::{Instrument, info_span};
use wasmtime::Store;
use wasmtime::component::InstancePre;

use super::generated::exports::wasi::messaging::incoming_handler::Error;
use crate::generated::Messaging;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

pub async fn run(instance_pre: InstancePre<RunState>) -> anyhow::Result<()> {
    // short-circuit when messaging not required
    let component_type = instance_pre.component().component_type();
    let engine = instance_pre.engine();
    if !component_type.exports(engine).any(|e| e.0.starts_with("wasi:messaging")) {
        tracing::debug!("messaging server not required");
        return Ok(());
    }

    // guest configuration
    let mut store = Store::new(engine, RunState::new());
    let instance = instance_pre.instantiate_async(&mut store).await?;
    let messaging = Messaging::new(&mut store, &instance)?;

    // *** WASIP3 ***
    // use `run_concurrent` for non-blocking execution
    let config = instance
        .run_concurrent(&mut store, async |accessor| {
            messaging.wasi_messaging_incoming_handler().call_configure(accessor).await?
        })
        .await??;

    // process requests
    subscribe(config.topics, instance_pre).await
}

pub async fn subscribe(
    channels: Vec<String>, instance_pre: InstancePre<RunState>,
) -> anyhow::Result<()> {
    tracing::trace!("subscribing to messaging channels: {channels:?}");

    // channels to subscribe to
    let mut subscribers = vec![];
    let client = crate::nats()?;
    for ch in &channels {
        tracing::debug!("subscribing to {ch}");
        let subscriber = client.subscribe(ch.clone()).await?;
        subscribers.push(subscriber);
    }

    tracing::info!("subscribed to {channels:?}");

    // process messages until terminated
    let mut messages = stream::select_all(subscribers);
    while let Some(msg) = messages.next().await {
        let instance_pre = instance_pre.clone();
        tokio::spawn(
            async move {
                if let Err(e) = call_guest(msg, instance_pre).await {
                    tracing::error!("error processing message {e}");
                }
            }
            .instrument(info_span!("message")),
        );
    }

    Ok(())
}

// Forward message to the wasm component.
async fn call_guest(message: Message, instance_pre: InstancePre<RunState>) -> Result<()> {
    let mut state = RunState::new();
    let res_msg = state.table.push(message)?;

    let mut store = Store::new(instance_pre.engine(), state);
    let instance = instance_pre.instantiate_async(&mut store).await?;
    let messaging = Messaging::new(&mut store, &instance)?;

    // *** WASIP3 ***
    // use `run_concurrent` for non-blocking execution
    instance
        .run_concurrent(&mut store, async |accessor| {
            messaging.wasi_messaging_incoming_handler().call_handle(accessor, res_msg).await?
        })
        .await?
}
