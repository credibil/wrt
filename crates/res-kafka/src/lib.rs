#![cfg(not(target_arch = "wasm32"))]

//! Kafka Client.

mod messaging;
mod partitioner;
mod registry;

use std::env;
use std::fmt::{self, Debug};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use rdkafka::consumer::StreamConsumer;
use rdkafka::producer::{DeliveryResult, ProducerContext, ThreadedProducer};
use rdkafka::{ClientConfig, ClientContext, Message as _};
use runtime::Resource;
use tracing::instrument;

use crate::partitioner::Partitioner;
use crate::registry::{Registry, SchemaConfig};

const KAFKA_BROKERS: &str = "localhost:9094";

#[derive(Clone)]
pub struct Client {
    producer: ThreadedProducer<Tracer>,
    consumer: Arc<StreamConsumer>,
    partitioner: Option<Partitioner>,
    registry: Option<Registry>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaClient").finish()
    }
}

impl Resource for Client {
    type ConnectOptions = ();

    #[instrument]
    async fn connect() -> Result<Self> {
        let kafka_config = KafkaConfig::from_env();
        let client_config = ClientConfig::from(&kafka_config);

        // maybe custom partitioner
        let partitioner = if kafka_config.js_partitioner.unwrap_or_default() {
            kafka_config.partition_count.map(Partitioner::new)
        } else {
            None
        };

        // maybe schema registry
        let registry = if let Some(cfg) = kafka_config.schema.as_ref()
            && !cfg.url.is_empty()
        {
            Some(Registry::new(cfg))
        } else {
            None
        };

        // producer and consumer
        let producer = client_config
            .create_with_context(Tracer {})
            .map_err(|e| anyhow!("issue creating producer: {e}"))?;
        let consumer =
            client_config.create().map_err(|e| anyhow!("issue creating consumer: {e}"))?;

        Ok(Self {
            producer,
            consumer: Arc::new(consumer),
            partitioner,
            registry,
        })
    }
}

#[derive(Debug, Clone)]
struct KafkaConfig {
    brokers: String,
    username: Option<String>,
    password: Option<String>,
    js_partitioner: Option<bool>,
    partition_count: Option<i32>, // producer
    schema: Option<SchemaConfig>,
    #[allow(unused)]
    group_id: Option<String>, // consumer
}

impl KafkaConfig {
    fn from_env() -> Self {
        let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| KAFKA_BROKERS.into());
        let username = env::var("KAFKA_USERNAME").ok();
        let password = env::var("KAFKA_PASSWORD").ok();
        let consumer_group = env::var("KAFKA_CONSUMER_GROUP").ok();
        let js_partitioner = env::var("KAFKA_JS_PARTITIONER").ok().and_then(|s| s.parse().ok());
        let partition_count = env::var("KAFKA_PARTITION_COUNT").ok().and_then(|s| s.parse().ok());
        let schema = SchemaConfig::from_env();

        tracing::info!("Kafka configuration built for brokers: {brokers}");

        Self {
            brokers,
            username,
            password,
            group_id: consumer_group,
            js_partitioner,
            partition_count,
            schema,
        }
    }
}

impl From<&KafkaConfig> for ClientConfig {
    fn from(kafka: &KafkaConfig) -> Self {
        let mut config = Self::new();
        config.set("bootstrap.servers", kafka.brokers.clone());

        // SASL authentication
        if let Some(user) = kafka.username.as_ref()
            && let Some(pass) = kafka.password.as_ref()
        {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanisms", "PLAIN");
            config.set("sasl.username", user);
            config.set("sasl.password", pass);
        }

        if let Some(group_id) = kafka.group_id.clone() {
            config.set("group.id", &group_id);
        }

        config
    }
}

pub struct Tracer;
impl ClientContext for Tracer {}
impl ProducerContext for Tracer {
    type DeliveryOpaque = ();

    fn delivery(&self, delivery_result: &DeliveryResult<'_>, (): Self::DeliveryOpaque) {
        match delivery_result {
            Ok(msg) => {
                let key: &str = msg.key_view().unwrap().unwrap();
                tracing::debug!(
                    "sent message {key} in offset {offset} of partition {partition}",
                    offset = msg.offset(),
                    partition = msg.partition()
                );
            }
            Err((err, message)) => {
                let key: &str = message.key_view().unwrap().unwrap();
                tracing::error!("Failed to send message {key}: {err}");
            }
        }
    }
}
