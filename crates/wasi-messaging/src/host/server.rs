use anyhow::Result;
use futures::StreamExt;
use runtime::State;
use runtime_error::Error as RuntimeError;
use tracing::{Instrument, debug_span, instrument};
use wasmtime::Store;

use crate::host::WasiMessagingView;
use crate::host::generated::Messaging;
use crate::host::resource::{MessageProxy, Subscriptions};

#[instrument("messaging-server", skip(state))]
pub async fn run<S>(state: &S) -> Result<()>
where
    S: State,
    <S as State>::StoreCtx: WasiMessagingView,
{
    tracing::info!("starting messaging server");

    let service = std::env::var("COMPONENT").unwrap_or_else(|_| "unknown".to_string());

    let handler = Handler {
        state: state.clone(),
        service,
    };
    let mut stream = handler.subscriptions().await?;

    while let Some(message) = stream.next().await {
        let handler = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = handler.send(message.clone()).await {
                tracing::error!("error processing message {e}");
            }
        });
    }

    Ok(())
}

#[derive(Clone)]
struct Handler<S>
where
    S: State,
    <S as State>::StoreCtx: WasiMessagingView,
{
    state: S,
    service: String,
}

impl<S> Handler<S>
where
    S: State,
    <S as State>::StoreCtx: WasiMessagingView,
{
    // Get subscriptions for the topics configured in the wasm component.
    async fn subscriptions(&self) -> Result<Subscriptions> {
        let instance_pre = self.state.instance_pre();
        let store_data = self.state.new_store();
        let mut store = Store::new(instance_pre.engine(), store_data);

        store
            .run_concurrent(async |store| {
                let client = store.with(|mut store| store.get().messaging().ctx.connect()).await?;
                client.subscribe().await
            })
            .await?
    }

    // Forward message to the wasm guest.
    async fn send(&self, message: MessageProxy) -> Result<(), RuntimeError> {
        let mut store_data = self.state.new_store();
        let res_msg = store_data
            .messaging()
            .table
            .push(message.clone())
            .map_err(|e| RuntimeError::ServerError(e.to_string()))?;

        let instance_pre = self.state.instance_pre();
        let mut store = Store::new(instance_pre.engine(), store_data);
        let instance = instance_pre.instantiate_async(&mut store).await?;
        let messaging = Messaging::new(&mut store, &instance)?;

        store
            .run_concurrent(async |store| {
                let guest = messaging.wasi_messaging_incoming_handler();
                Ok(guest.call_handle(store, res_msg).await??)
            })
            .instrument(debug_span!("messaging-handle"))
            .await
    }
}
