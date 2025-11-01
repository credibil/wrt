///Custom parttioner for Kafka producer
pub mod partitioner;
///Schema registry module for Kafka
pub mod schema_registry;

use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::pin::Pin;

use anyhow::Result;
use rdkafka::producer::{ProducerContext, ThreadedProducer};
use rdkafka::{ClientContext, Message as _};
use runtime::ResourceBuilder;
use tracing::instrument;

use crate::partitioner::Partitioner;
use crate::schema_registry::RegistryClient;

const DEF_KAFKA_BROKERS: &str = "localhost:9094";

/// Kafka producer client
pub struct KafkaProducer {
    pub producer: ThreadedProducer<ProduceCallbackLogger>,
    pub partitioner: Option<Partitioner>,
    pub sr_client: Option<RegistryClient>,
}

/// Kafka resource builder
pub struct Kafka {
    attributes: HashMap<String, String>,
}

/// Kafka configuration
#[derive(Debug, Clone)]
pub struct KafkaClient {
    /// Comma-separated list of Kafka brokers
    pub brokers: String,
    /// Optional username for SASL authentication
    pub username: Option<String>,
    /// Optional password for SASL authentication
    pub password: Option<String>,
    /// Consumer group ID
    pub group_id: Option<String>, // only used for consumer
    /// Enable custom JS partitioner for producer
    pub js_partitioner: Option<bool>,
    /// Number of partitions for custom partitioner
    pub partition_count: Option<i32>, // only used for producer
    /// Optional schema registry configuration
    pub schema: Option<SchemaConfig>,
}

impl KafkaClient {
    /// Get the name of the client
    #[must_use]
    pub fn name(&self) -> String {
        "kafka".to_string()
    }
}

/// Schema registry configuration
#[derive(Debug, Clone)]
pub struct SchemaConfig {
    /// Schema registry URL
    pub url: String,
    /// Optional API key for schema registry
    pub api_key: Option<String>,
    /// Optional API secret for schema registry
    pub api_secret: Option<String>,
    /// Optional cache TTL in seconds for schema registry
    pub cache_ttl_secs: Option<u64>,
}

impl ResourceBuilder<KafkaClient> for Kafka {
    fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    #[instrument(name = "Kafka::connect", skip(self))]
    async fn connect(self) -> Result<KafkaClient> {
        let brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| DEF_KAFKA_BROKERS.into());
        let username = env::var("KAFKA_USERNAME").ok();
        let password = env::var("KAFKA_PASSWORD").ok();
        let consumer_group = env::var("KAFKA_CONSUMER_GROUP").ok();
        let js_partitioner =
            env::var("KAFKA_JS_PARTITIONER").ok().and_then(|s| s.parse::<bool>().ok());
        let partition_count =
            env::var("KAFKA_PARTITION_COUNT").ok().and_then(|s| s.parse::<i32>().ok());

        let schema_config = env::var("SCHEMA_REGISTRY_URL").map_or(None, |url| {
            let api_key = env::var("SCHEMA_REGISTRY_API_KEY").ok();
            let api_secret = env::var("SCHEMA_REGISTRY_API_SECRET").ok();
            let cache_ttl_secs =
                env::var("SCHEMA_CACHE_TTL_SECS").ok().and_then(|v| v.parse::<u64>().ok());

            Some(SchemaConfig {
                url,
                api_key,
                api_secret,
                cache_ttl_secs,
            })
        });

        let config = KafkaClient {
            brokers: brokers.clone(),
            username,
            password,
            group_id: consumer_group,
            js_partitioner,
            partition_count,
            schema: schema_config,
        };

        tracing::info!("Kafka configuration built for brokers: {brokers}");
        Ok(config)
    }
}

/// Logger for Kafka produce callbacks
pub struct ProduceCallbackLogger;

impl ClientContext for ProduceCallbackLogger {}

impl ProducerContext for ProduceCallbackLogger {
    type DeliveryOpaque = ();

    fn delivery(
        &self, delivery_result: &rdkafka::producer::DeliveryResult<'_>,
        _delivery_opaque: Self::DeliveryOpaque,
    ) {
        let dr = delivery_result.as_ref();
        //let msg = dr.unwrap();

        match dr {
            Ok(msg) => {
                let key: &str = msg.key_view().unwrap().unwrap();
                tracing::debug!(
                    "produced message with key {} in offset {} of partition {}",
                    key,
                    msg.offset(),
                    msg.partition()
                );
            }
            Err((producer_err, message)) => {
                let key: &str = message.key_view().unwrap().unwrap();

                // Log or forward the structured error
                tracing::error!("Failed to produce message with key '{}': {}", key, producer_err);
            }
        }
    }
}

impl IntoFuture for Kafka {
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<KafkaClient>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}
