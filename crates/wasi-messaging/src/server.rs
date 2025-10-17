use runtime::RunState;
use wasmtime::Store;
use wasmtime::component::InstancePre;

use super::generated::exports::wasi::messaging::incoming_handler::Error;
use crate::client::Client;
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
    // Client::subscribe(config.topics, instance_pre).await
    todo!()
}
