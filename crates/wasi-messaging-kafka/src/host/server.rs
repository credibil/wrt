use anyhow::Result;
use futures::StreamExt;
use rdkafka::consumer::{CommitMode, Consumer as _, StreamConsumer};
use rdkafka::message::{Headers as _, OwnedHeaders};
use rdkafka::{ClientConfig, Message as _};
use runtime::{Resource, State};
use tracing::{Instrument, info_span};
use wasmtime::Store;

use crate::host::generated::Messaging;
use crate::host::generated::wasi::messaging::types::Message;
use crate::host::resource::KafkaClient;
use crate::host::{Error, WasiMessagingView};
use crate::schema_registry::RegistryClient;

pub async fn run<S>(state: &S) -> Result<()>
where
    S: State,
    <S as State>::StoreData: WasiMessagingView,
{
    let instance_pre = state.instance_pre();
    let store_data = state.new_store();
    let mut store = Store::new(instance_pre.engine(), store_data);
    let instance = instance_pre.instantiate_async(&mut store).await?;
    let messaging = Messaging::new(&mut store, &instance)?;

    let config = instance
        .run_concurrent(&mut store, async |accessor| {
            messaging.wasi_messaging_incoming_handler().call_configure(accessor).await?
        })
        .await??;

    // process requests
    tracing::info!("starting messaging server");

    // process requests
    subscribe(config.topics, state).await
}

async fn subscribe<S>(topics: Vec<String>, state: &S) -> anyhow::Result<()>
where
    S: State,
    <S as State>::StoreData: WasiMessagingView,
{
    tracing::debug!("subscribing to kafka topics: {topics:?}");

    let kafka_config = KafkaClient::connect().await?;

    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", kafka_config.brokers.clone());

    // Optional SASL authentication
    if let (Some(user), Some(pass)) = (kafka_config.username.clone(), kafka_config.password.clone())
    {
        config.set("security.protocol", "SASL_SSL");
        config.set("sasl.mechanisms", "PLAIN");
        config.set("sasl.username", &user);
        config.set("sasl.password", &pass);
    }

    if let Some(group_id) = kafka_config.group_id.clone() {
        config.set("group.id", &group_id);
    }

    // Initialize schema registry client if config is provided
    let sr_client = kafka_config.schema.as_ref().map_or_else(
        || None,
        |cfg| if cfg.url.is_empty() { None } else { Some(RegistryClient::new(&cfg.clone())) },
    );

    let consumer: StreamConsumer = config.create().unwrap();
    consumer.subscribe(&topics.iter().map(|s| &**s).collect::<Vec<&str>>())?;
    tracing::debug!("subscribed to topics: {topics:?}");

    let mut stream = consumer.stream();

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(msg) => {
                let state = state.clone();
                let mut owned_msg = msg.detach();

                //validate payload if schema registry provided
                let payload_bytes = owned_msg.payload().map_or_else(Vec::new, <[u8]>::to_vec);

                if let Some(sr) = &sr_client {
                    // schema_registry exists → serialize
                    let payload =
                        sr.validate_and_encode_json(owned_msg.topic(), payload_bytes).await;
                    owned_msg = rebuild_message(&owned_msg, Some(payload), None);
                }

                tokio::spawn(
                    async move {
                        // Process the message.
                        if let Err(e) = call_guest(owned_msg, &state).await {
                            tracing::error!("error processing message {e}");
                        }
                    }
                    .instrument(info_span!("message")),
                );
                //Do we need batch commit to improve performance?
                if let Err(e) = consumer.commit_message(&msg, CommitMode::Async) {
                    tracing::error!("failed to commit message: {e}");
                }
            }
            Err(e) => tracing::error!("kafka error: {e}"),
        }
    }

    Ok(())
}

// Forward message to the wasm component.
async fn call_guest<S>(message: Message, state: &S) -> Result<(), Error>
where
    S: State,
    <S as State>::StoreData: WasiMessagingView,
{
    let instance_pre = state.instance_pre();
    let mut store_data = state.new_store();

    let res_msg = store_data.messaging().table.push(message.clone())?;

    let mut store = Store::new(instance_pre.engine(), store_data);
    let instance = instance_pre.instantiate_async(&mut store).await?;
    let messaging = Messaging::new(&mut store, &instance)?;

    instance
        .run_concurrent(&mut store, async |accessor| {
            messaging.wasi_messaging_incoming_handler().call_handle(accessor, res_msg).await?
        })
        .await?
}

/// Helper: build a new message based on an existing one, overriding only some fields.
pub fn rebuild_message(
    base: &Message, payload: Option<Vec<u8>>, headers: Option<OwnedHeaders>,
) -> Message {
    let new_headers = headers.or_else(|| base.headers().cloned());
    let new_key = new_headers
        .as_ref()
        .and_then(|hs| hs.iter().find(|h| h.key == "key"))
        .and_then(|h| h.value)
        .map(<[u8]>::to_vec);
    Message::new(
        payload.or_else(|| base.payload().map(<[u8]>::to_vec)),
        new_key,
        base.topic().to_string(),
        base.timestamp(),
        base.partition(),
        base.offset(),
        new_headers,
    )
}
