use anyhow::{Context, Result};
use futures::StreamExt;
use runtime::State;
use tracing::{Instrument, debug_span};
use wasmtime::Store;

use crate::host::WasiMessagingView;
use crate::host::generated::Messaging;
use crate::host::resource::{Message, Subscriptions};

pub async fn run<S>(state: &S) -> Result<()>
where
    S: State,
    <S as State>::StoreData: WasiMessagingView,
{
    tracing::info!("starting messaging server");

    let handler = Handler { state: state.clone() };
    let mut stream = handler.subscribe().await?;

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
    <S as State>::StoreData: WasiMessagingView,
{
    state: S,
}

impl<S> Handler<S>
where
    S: State,
    <S as State>::StoreData: WasiMessagingView,
{
    // Get subscriptions for the topics configured in the wasm component.
    async fn subscribe(&self) -> Result<Subscriptions> {
        let instance_pre = self.state.instance_pre();
        let store_data = self.state.new_store();
        let mut store = Store::new(instance_pre.engine(), store_data);
        let instance = instance_pre.instantiate_async(&mut store).await?;
        let messaging = Messaging::new(&mut store, &instance)?;

        instance
            .run_concurrent(&mut store, async |accessor| {
                let guest = messaging.wasi_messaging_incoming_handler();
                let config = guest.call_configure(accessor).await??;
                let client =
                    accessor.with(|mut store| store.get().messaging().ctx.connect()).await?;
                client.subscribe(config.topics.clone()).await
            })
            .await?
    }

    // Forward message to the wasm component.
    async fn send(&self, message: Message) -> Result<()> {
        let mut store_data = self.state.new_store();
        let res_msg = store_data.messaging().table.push(message.clone())?;

        let instance_pre = self.state.instance_pre();
        let mut store = Store::new(instance_pre.engine(), store_data);
        let instance = instance_pre.instantiate_async(&mut store).await?;
        let messaging = Messaging::new(&mut store, &instance)?;

        instance
            .run_concurrent(&mut store, async |accessor| {
                let client =
                    accessor.with(|mut store| store.get().messaging().ctx.connect()).await?;

                client.pre_send(&message).await?;
                let guest = messaging.wasi_messaging_incoming_handler();
                guest.call_handle(accessor, res_msg).await??;
                client.post_send(&message).await?;

                Ok::<(), anyhow::Error>(())
            })
            .instrument(debug_span!("messaging-handle"))
            .await
            .context("running instance")?
    }
}
