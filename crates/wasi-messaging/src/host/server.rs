use std::str::FromStr;

use anyhow::Result;
use credibil_error::Error;
use futures::StreamExt;
use kernel::State;
use tracing::{Instrument, debug_span, instrument};
use utils::messaging::log_with_metrics;
use wasmtime::Store;

use crate::host::WasiMessagingView;
use crate::host::generated::Messaging;
use crate::host::resource::{MessageProxy, Subscriptions};

#[instrument("messaging-server", skip(state))]
pub async fn run<S>(state: &S) -> Result<()>
where
    S: State,
    S::StoreCtx: WasiMessagingView,
{
    tracing::info!("starting messaging server");

    let service = std::env::var("COMPONENT").unwrap_or_else(|_| "unknown".into());

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
    S::StoreCtx: WasiMessagingView,
{
    state: S,
    service: String,
}

impl<S> Handler<S>
where
    S: State,
    S::StoreCtx: WasiMessagingView,
{
    // Get subscriptions for the topics configured in the wasm component.
    async fn subscriptions(&self) -> Result<Subscriptions> {
        let instance_pre = self.state.instance_pre();
        let store_data = self.state.store();
        let mut store = Store::new(instance_pre.engine(), store_data);

        store
            .run_concurrent(async |store| {
                let client = store.with(|mut store| store.get().messaging().ctx.connect()).await?;
                client.subscribe().await
            })
            .await?
    }

    // Forward message to the wasm guest.
    async fn send(&self, message: MessageProxy) -> Result<(), Error> {
        let mut store_data = self.state.store();
        let be_msg = store_data
            .messaging()
            .table
            .push(message.clone())
            .map_err(|e| Error::ServerError(e.to_string()))?;

        let instance_pre = self.state.instance_pre();
        let mut store = Store::new(instance_pre.engine(), store_data);
        let instance = instance_pre.instantiate_async(&mut store).await?;
        let messaging = Messaging::new(&mut store, &instance)?;

        match store
            .run_concurrent(async |store| {
                let guest = messaging.wasi_messaging_incoming_handler();
                guest.call_handle(store, be_msg).await.map(|_| ()).map_err(Error::from)
            })
            .instrument(debug_span!("messaging-handle"))
            .await
        {
            Ok(_) => {
                tracing::info!(monotonic_counter.messages_processed = 1, service = %self.service, topic = %message.topic());
                Ok(())
            }
            Err(e) => match Error::from_str(e.to_string().as_str()) {
                // Both Ok and Err arms do the same thing, but this way we ensure
                // that we only log known CredibilErrors in a structured way.
                Ok(err) | Err(err) => {
                    log_with_metrics(&err, &self.service, &message.topic());
                    Err(err)
                }
            },
        }
    }
}
