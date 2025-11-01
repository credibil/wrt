use std::time::{SystemTime, UNIX_EPOCH};

use crate::host::Result;
pub use crate::host::generated::wasi::messaging::types::{
    Error, Host, HostClient, HostClientWithStore, HostMessage, HostMessageWithStore, Metadata,
    Topic,
};
use crate::host::resource::KafkaProducer;
use crate::host::server::rebuild_message;
use crate::host::{WasiMessaging, WasiMessagingCtxView};
use crate::partitioner::Partitioner;
use crate::schema_registry::RegistryClient;
use crate::{KafkaClient, ProduceCallbackLogger};
use anyhow::anyhow;
use rdkafka::message::OwnedMessage;
use rdkafka::message::{Header, Headers as _, OwnedHeaders};
use rdkafka::{ClientConfig, Message as _, Timestamp};
use runtime::Resource as _;
use wasmtime::component::{Accessor, Resource};

impl HostClientWithStore for WasiMessaging {
    async fn connect<T>(
        accessor: &Accessor<T, Self>, _name: String,
    ) -> Result<Resource<KafkaProducer>> {
        tracing::debug!("HostClient::connect Kafka");

        let kafka_config = KafkaClient::connect().await?;

        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", kafka_config.brokers.clone());

        // Optional SASL authentication
        if let (Some(user), Some(pass)) =
            (kafka_config.username.clone(), kafka_config.password.clone())
        {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanisms", "PLAIN");
            config.set("sasl.username", &user);
            config.set("sasl.password", &pass);
        }

        // Optional: Initialize custom partitioner if js_partitioner is true
        let partitioner = if kafka_config.js_partitioner.unwrap_or(false) {
            kafka_config.partition_count.map(Partitioner::new)
        } else {
            None
        };

        // Initialize schema registry client if config is provided
        let sr_client = kafka_config.schema.as_ref().map_or_else(
            || None,
            |cfg| if cfg.url.is_empty() { None } else { Some(RegistryClient::new(&cfg.clone())) },
        );

        let producer = config
            .create_with_context(ProduceCallbackLogger {})
            .map_err(|e| anyhow!("invalid producer config: {e}"))?;

        let client = KafkaProducer {
            producer,
            partitioner,
            sr_client,
        };

        Ok(accessor.with(|mut store| store.get().table.push(client))?)
    }

    async fn disconnect<T>(_: &Accessor<T, Self>, _rep: Resource<KafkaProducer>) -> Result<()> {
        tracing::debug!("HostClient::disconnect (noop for Kafka producer)");
        Ok(())
    }

    async fn drop<T>(
        accessor: &Accessor<T, Self>, rep: Resource<KafkaProducer>,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostClient::drop");
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl HostMessageWithStore for WasiMessaging {
    /// Create a new message with the given payload.
    async fn new<T>(
        accessor: &Accessor<T, Self>, data: Vec<u8>,
    ) -> anyhow::Result<Resource<OwnedMessage>> {
        tracing::debug!("HostMessage::new with {} bytes", data.len());
        let now = i64::try_from(
            SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis(),
        ); // rdkafka expects i64
        let msg = OwnedMessage::new(
            data.into(),                 //payload
            None,                        //key
            String::new(),               //topic
            Timestamp::CreateTime(now?), //timestamp
            -1,                          //partition
            -1,                          //offset
            None,                        //headers
        );
        Ok(accessor.with(|mut store| store.get().table.push(msg))?)
    }

    /// The topic/subject/channel this message was received on, if any.
    async fn topic<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>,
    ) -> anyhow::Result<Option<Topic>> {
        tracing::debug!("HostMessage::topic");
        let msg = get_message(accessor, &self_)?;
        let topic = msg.topic();
        if topic.is_empty() { Ok(None) } else { Ok(Some(topic.to_string())) }
    }

    /// An optional content-type describing the format of the data in the
    /// message. This is sometimes described as the "format" type".
    async fn content_type<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>,
    ) -> anyhow::Result<Option<String>> {
        tracing::debug!("HostMessage::content_type");
        let msg = get_message(accessor, &self_)?;

        // Access headers from the message
        if let Some(headers) = msg.headers() {
            for i in 0..headers.count() {
                let header = headers.get(i); // Header<'_, &[u8]>
                if header.key.eq_ignore_ascii_case("content-type")
                    && let Some(value) = header.value
                {
                    // Convert &[u8] to String
                    return Ok(Some(String::from_utf8_lossy(value).into_owned()));
                }
            }
        }
        Ok(None)
    }

    /// Set the content-type describing the format of the data in the message.
    /// This is sometimes described as the "format" type.
    async fn set_content_type<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>, content_type: String,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::set_content_type {}", content_type);
        accessor.with(|mut store| {
            let msg = store.get().table.get_mut(&self_)?;
            let new_headers =
                update_headers(msg.headers(), "content-type", Some(content_type.as_bytes()));
            *msg = rebuild_message(msg, None, Some(new_headers));
            Ok(())
        })
    }

    /// An opaque blob of data.
    async fn data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>,
    ) -> anyhow::Result<Vec<u8>> {
        tracing::debug!("HostMessage::data");
        let msg = get_message(accessor, &self_)?;
        let data: Vec<u8> = msg
            .payload()
            .map(<[u8]>::to_vec) // convert &[u8] to Vec<u8>
            .unwrap_or_default(); // default to empty Vec if None

        Ok(data)
    }

    /// Set the opaque blob of data for this message, discarding the old value".
    async fn set_data<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>, data: Vec<u8>,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::set_data");

        accessor.with(|mut store| {
            let msg = store.get().table.get_mut(&self_)?;
            *msg = rebuild_message(msg, Some(data), None);
            Ok(())
        })
    }

    async fn metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>,
    ) -> anyhow::Result<Option<Metadata>> {
        tracing::debug!("HostMessage::metadata");
        let msg = get_message(accessor, &self_)?;
        let headers = msg.headers().map(|owned| {
            owned
                .iter()
                .map(|h| {
                    (
                        h.key.to_string(),
                        h.value.map(|v| String::from_utf8_lossy(v).to_string()).unwrap_or_default(),
                    )
                })
                .collect()
        });
        Ok(headers)
    }

    async fn add_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>, key: String, value: String,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::add_metadata {key}={value}");

        accessor.with(|mut store| {
            let msg = store.get().table.get_mut(&self_)?;
            let new_headers = update_headers(msg.headers(), &key, Some(value.as_bytes()));
            *msg = rebuild_message(msg, None, Some(new_headers));
            Ok(())
        })
    }

    //Replace all headers with the provided metadata
    async fn set_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>, meta: Metadata,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::set_metadata");

        accessor.with(|mut store| {
            let msg = store.get().table.get_mut(&self_)?;
            let mut new_headers = OwnedHeaders::new();

            // Insert all metadata key/value pairs into headers
            for (key, value) in meta {
                new_headers = new_headers.insert(Header {
                    key: &key,
                    value: Some(value.as_bytes()),
                });
            }
            *msg = rebuild_message(msg, None, Some(new_headers));
            Ok(())
        })
    }

    async fn remove_metadata<T>(
        accessor: &Accessor<T, Self>, self_: Resource<OwnedMessage>, key: String,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::remove_metadata {key}");

        accessor.with(|mut store| {
            let msg = store.get().table.get_mut(&self_)?;
            let new_headers = update_headers(msg.headers(), &key, None);
            *msg = rebuild_message(msg, None, Some(new_headers));
            Ok(())
        })
    }

    async fn drop<T>(
        accessor: &Accessor<T, Self>, rep: Resource<OwnedMessage>,
    ) -> anyhow::Result<()> {
        tracing::debug!("HostMessage::drop");
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

/// Helper to clone headers and optionally update a single key/value
pub fn update_headers(
    existing: Option<&OwnedHeaders>, key: &str, value: Option<&[u8]>,
) -> OwnedHeaders {
    let mut new_headers = OwnedHeaders::new();
    if let Some(hdrs) = existing {
        for h in hdrs.iter() {
            if !h.key.eq_ignore_ascii_case(key) {
                new_headers = new_headers.insert(h);
            }
        }
    }
    if let Some(val) = value {
        new_headers = new_headers.insert(Header {
            key,
            value: Some(val),
        });
    }
    new_headers
}

impl Host for WasiMessagingCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> anyhow::Result<Error> {
        Ok(err)
    }
}
impl HostClient for WasiMessagingCtxView<'_> {}
impl HostMessage for WasiMessagingCtxView<'_> {}
use std::sync::Arc;
pub fn get_client<T>(
    accessor: &Accessor<T, WasiMessaging>, self_: Resource<KafkaProducer>,
) -> Result<KafkaProducer> {
    accessor.with(|mut store| {
        let client = store.get().table.get(self_)?;
        Ok::<_, Error>(Arc::new(client.clone()))
    })
}

pub fn get_message<T>(
    accessor: &Accessor<T, WasiMessaging>, self_: &Resource<OwnedMessage>,
) -> Result<OwnedMessage> {
    accessor.with(|mut store| {
        let message = store.get().table.get(self_)?;
        Ok::<_, Error>(message.clone())
    })
}
