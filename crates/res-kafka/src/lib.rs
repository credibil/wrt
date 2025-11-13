#![cfg(not(target_arch = "wasm32"))]

//! Kafka Client.

mod messaging;
mod partitioner;
mod registry;

use std::fmt::{self, Debug};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use fromenv::{FromEnv, ParseResult};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{DeliveryResult, ProducerContext, ThreadedProducer};
use rdkafka::{ClientConfig, ClientContext, Message as _};
use runtime::Resource;
use tracing::instrument;

use crate::partitioner::Partitioner;
use crate::registry::Registry;

#[derive(Clone)]
pub struct Client {
    producer: ThreadedProducer<Tracer>,
    partitioner: Option<Partitioner>,
    registry: Option<Registry>,
    consumer: Option<Arc<StreamConsumer>>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaClient").finish()
    }
}

impl Resource for Client {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let config = ClientConfig::from(&options);

        // producer
        let producer = config
            .create_with_context(Tracer {})
            .map_err(|e| anyhow!("issue creating producer: {e}"))?;

        // maybe custom partitioner and schema registry
        let partitioner = options.partition_count.map(Partitioner::new);
        let registry = options.registry.map(Registry::new);

        // maybe consumer
        let consumer = if let Some(topics) = options.topics {
            let consumer: StreamConsumer =
                config.create().map_err(|e| anyhow!("issue creating consumer: {e}"))?;
            let topics = topics.iter().map(String::as_str).collect::<Vec<_>>();
            consumer.subscribe(&topics)?;
            Some(Arc::new(consumer))
        } else {
            None
        };

        Ok(Self {
            producer,
            partitioner,
            registry,
            consumer,
        })
    }
}

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "KAFKA_BROKERS")]
    pub brokers: String,
    #[env(from = "KAFKA_USERNAME")]
    pub username: Option<String>,
    #[env(from = "KAFKA_PASSWORD")]
    pub password: Option<String>,
    #[env(from = "KAFKA_TOPICS", with = split)]
    pub topics: Option<Vec<String>>,
    #[env(from = "KAFKA_CONSUMER_GROUP")]
    pub group_id: Option<String>,
    #[env(from = "KAFKA_PARTITION_COUNT")]
    pub partition_count: Option<i32>,
    #[env(from = "COMPONENT")]
    pub component: Option<String>,
    #[env(from = "ENVIRONMENT", default = "dev")]
    pub env: String,
    #[env(nested)]
    pub registry: Option<RegistryOptions>,
}

#[derive(Debug, Clone, FromEnv)]
pub struct RegistryOptions {
    #[env(from = "KAFKA_REGISTRY_URL")]
    pub url: String,
    #[env(from = "KAFKA_REGISTRY_API_KEY")]
    api_key: String,
    #[env(from = "KAFKA_REGISTRY_API_SECRET")]
    api_secret: String,
    #[env(from = "KAFKA_REGISTRY_CACHE_TTL", default = "3600")]
    cache_ttl_secs: u64,
}

#[allow(clippy::unnecessary_wraps)]
fn split(s: &str) -> ParseResult<Vec<String>> {
    Ok(s.split(',').map(ToOwned::to_owned).collect())
}

impl From<&ConnectOptions> for ClientConfig {
    fn from(kafka: &ConnectOptions) -> Self {
        let mut config = Self::new();
        config.set("bootstrap.servers", kafka.brokers.clone());

        // SASL authentication
        if let Some(user) = &kafka.username
            && let Some(pass) = &kafka.password
        {
            config.set("security.protocol", "SASL_SSL");
            config.set("sasl.mechanisms", "PLAIN");
            config.set("sasl.username", user);
            config.set("sasl.password", pass);
        }

        if let Some(group_id) = kafka.group_id.clone() {
            config.set("group.id", &group_id);
        }

        if let Some(component) = &kafka.component {
            config.set(
                "client.id",
                format!("{}-{component}-{}", &kafka.env, rand::random_range(1000..9999)),
            );
        }

        config
    }
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
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
