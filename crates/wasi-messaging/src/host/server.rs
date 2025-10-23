use anyhow::{Context, Result};
use futures::StreamExt;
use runtime::RunState;
use tracing::{Instrument, info_span};
use wasmtime::Store;
use wasmtime::component::InstancePre;

use crate::host::generated::Messaging;
use crate::host::resource::Message;
use crate::host::{CLIENT, Error};

pub async fn run(instance_pre: InstancePre<RunState>) -> Result<()> {
    // short-circuit when messaging not required
    let component_type = instance_pre.component().component_type();
    let engine = instance_pre.engine();
    if !component_type.exports(engine).any(|e| e.0.starts_with("wasi:messaging")) {
        tracing::debug!("messaging server not required");
        return Ok(());
    }

    // get guest configuration
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

    let Some(client) = CLIENT.get() else {
        return Err(anyhow::anyhow!("no messaging client registered"))?;
    };

    // process requests
    tracing::info!("starting messaging server for client: {}", client.name());

    let topics = config.topics.clone();
    let mut stream = client.subscribe(topics).await.map_err(|e| {
        tracing::error!("failed to start messaging server: {e}");
        e
    })?;

    while let Some(message) = stream.next().await {
        let instance_pre = instance_pre.clone();

        tokio::spawn(
            async move {
                if let Err(e) = client.pre_send(&message).await {
                    tracing::error!("error processing message {e}");
                    return;
                }
                if let Err(e) = call_guest(message.clone(), instance_pre).await {
                    tracing::error!("error processing message {e}");
                }
                if let Err(e) = client.post_send(&message).await {
                    tracing::error!("error processing message {e}");
                }
            }
            .instrument(info_span!("message")),
        );
    }

    Ok(())
}

// Forward message to the wasm component.
async fn call_guest(message: Message, instance_pre: InstancePre<RunState>) -> Result<(), Error> {
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
        .await
        .context("running instance")?
}
