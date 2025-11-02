use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use jsonschema::validate;
use schema_registry_client::rest::client_config::{BasicAuth, ClientConfig as SchemaClientConfig};
use schema_registry_client::rest::schema_registry_client::{Client, SchemaRegistryClient};
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::time;

#[derive(Debug, Clone)]
pub struct SchemaConfig {
    pub url: String,
    api_key: Option<String>,
    api_secret: Option<String>,
    cache_ttl_secs: Option<u64>,
}

impl SchemaConfig {
    pub fn from_env() -> Option<Self> {
        let url = env::var("SCHEMA_REGISTRY_URL").ok()?;
        let api_key = env::var("SCHEMA_REGISTRY_API_KEY").ok();
        let api_secret = env::var("SCHEMA_REGISTRY_API_SECRET").ok();
        let cache_ttl_secs = env::var("SCHEMA_CACHE_TTL_SECS").ok().and_then(|s| s.parse().ok());

        Some(Self {
            url,
            api_key,
            api_secret,
            cache_ttl_secs,
        })
    }
}

/// Schema Registry client with caching
#[derive(Clone)]
pub struct Registry {
    client: Option<SchemaRegistryClient>,
    schemas: Arc<Mutex<HashMap<String, (i32, Value)>>>,
}

/// Constants for encoding/decoding
const MAGIC_BYTE: u8 = 0; // single byte

impl Registry {
    /// Create a new Schema Registry client
    #[must_use]
    pub fn new(schema_cfg: &SchemaConfig) -> Self {
        // Build optional basic auth
        let auth: Option<BasicAuth> = schema_cfg.api_key.as_ref().map(|key| {
            (key.clone(), schema_cfg.api_secret.clone()) // BasicAuth = (String, Option<String>)
        });

        // Create SchemaRegistry client config with just URLs
        let mut client_config = SchemaClientConfig::new(vec![schema_cfg.url.clone()]);

        // Set basic auth if present
        if let Some((username, password)) = auth {
            client_config.basic_auth = Some((username, password));
        }

        // Create the schema registry client
        let client = Some(SchemaRegistryClient::new(client_config));

        let schemas = Arc::new(Mutex::new(HashMap::new()));

        let sr_client = Self {
            client,
            schemas: Arc::clone(&schemas),
        };

        // Start background cache cleaner only if TTL is provided
        if let Some(ttl_secs) = schema_cfg.cache_ttl_secs {
            sr_client.start_cache_cleaner(ttl_secs);
        }

        sr_client
    }

    /// Serialize payload to JSON with optional schema registry
    pub async fn validate_and_encode_json(&self, topic: &str, buffer: Vec<u8>) -> Vec<u8> {
        // If schema registry is available, use it
        if self.client.is_some() {
            match self.get_or_fetch_schema(topic).await {
                Ok((id, schema)) => {
                    let payload: Value = match serde_json::from_slice(&buffer) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!("Invalid JSON: {:?}", e);
                            return buffer;
                        }
                    };

                    if let Err(e) = self.validate_payload_with_schema(&schema, &payload) {
                        tracing::error!("JSON validation failed: {}", e);
                        return buffer;
                    }

                    Payload::encode(id, buffer)
                }
                Err(e) => {
                    tracing::error!("Failed to fetch schema for topic {}: {:?}", topic, e);
                    buffer
                }
            }
        } else {
            buffer
        }
    }

    /// Deserialize payload to JSON with optional schema registry
    #[allow(unused)]
    pub async fn validate_and_decode_json(&self, topic: &str, buffer: &[u8]) -> Vec<u8> {
        if let Some(_sr_client) = &self.client {
            let Some(decoded) = Payload::decode(buffer) else { return buffer.to_vec() };

            let (_id, schema) = match self.get_or_fetch_schema(topic).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to fetch schema: {:?}", e);
                    return decoded.data.to_vec();
                }
            };

            let payload: Value = match serde_json::from_slice(decoded.data) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Invalid JSON: {:?}", e);
                    return decoded.data.to_vec();
                }
            };

            if let Err(e) = self.validate_payload_with_schema(&schema, &payload) {
                tracing::error!("JSON validation failed: {}", e);
            }

            decoded.data.to_vec()
        } else {
            buffer.to_vec()
        }
    }

    /// # Errors`RegisteredSchema`
    ///
    /// Validate a JSON payload against a provided `RegisteredSchema`
    #[allow(clippy::unused_self)]
    pub fn validate_payload_with_schema(
        &self, schema: &Value, payload: &Value,
    ) -> Result<(), String> {
        validate(schema, payload).map_err(|e| format!("Validation error: {e}"))?;
        Ok(())
    }

    async fn get_or_fetch_schema(&self, topic: &str) -> Result<(i32, Value), String> {
        let sr = self
            .client
            .as_ref()
            .ok_or_else(|| "No schema registry client available".to_string())?;

        let mut schemas = self.schemas.lock().await;
        if let Some((id, value)) = schemas.get(topic) {
            Ok((*id, value.clone()))
        } else {
            let subject = format!("{topic}-value");
            let schema_response = sr
                .get_latest_version(&subject, None)
                .await
                .map_err(|e| format!("Failed to fetch schema for {subject}: {e:?}"))?;

            let schema_str = schema_response
                .schema
                .as_ref()
                .ok_or_else(|| "Schema string is missing".to_string())?;

            let schema_json: Value = serde_json::from_str(schema_str)
                .map_err(|e| format!("Invalid schema JSON: {e:?}"))?;

            let registry_id = schema_response
                .id
                .ok_or_else(|| format!("Registry ID missing for topic {topic}"))?;

            schemas.insert(topic.to_string(), (registry_id, schema_json.clone()));
            drop(schemas);
            Ok((registry_id, schema_json))
        }
    }

    /// Private method to spawn the cache cleaner task every hour
    fn start_cache_cleaner(&self, cache_ttl_secs: u64) {
        let schemas_clone = Arc::clone(&self.schemas);
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(cache_ttl_secs));
            loop {
                interval.tick().await;
                schemas_clone.lock().await.clear();
                tracing::info!("Schema cache cleared");
            }
        });
    }
}

#[allow(unused)]
pub struct Payload<'a> {
    magic_byte: u8,
    registry_id: i32,
    data: &'a [u8],
}

impl Payload<'_> {
    /// Encode payload with schema registry ID repeats JS code
    #[must_use]
    pub fn encode(registry_id: i32, payload: Vec<u8>) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 4 + payload.len());
        buf.push(MAGIC_BYTE);
        buf.extend(&registry_id.to_be_bytes());
        buf.extend(payload);
        buf
    }

    /// Decode payload
    pub fn decode(buffer: &[u8]) -> Option<Payload<'_>> {
        if buffer.len() < 5 {
            tracing::error!("Buffer too short to decode");
            return None;
        }

        let magic_byte = buffer[0];
        let registry_id = i32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
        let data = &buffer[5..];

        Some(Payload {
            magic_byte,
            registry_id,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*; // brings Payload, MAGIC_BYTE, MessagingError into scope

    #[test]
    fn encode_then_decode_roundtrip() {
        #[allow(clippy::cast_possible_wrap)]
        let registry_id: i32 = 0xAABB_CCDDu32 as i32;
        let payload = b"hello world".to_vec();

        // Encode
        let encoded = Payload::encode(registry_id, payload.clone());

        // Expected layout:
        // [ magic_byte ][ registry_id (4 bytes BE) ][ payload... ]
        assert_eq!(encoded[0], MAGIC_BYTE, "magic byte mismatch");

        let expected_id_bytes = registry_id.to_be_bytes();
        assert_eq!(&encoded[1..5], &expected_id_bytes, "registry id mismatch");
        assert_eq!(&encoded[5..], &payload, "payload mismatch");

        // Decode
        let decoded = Payload::decode(&encoded).expect("decode failed");

        assert_eq!(decoded.magic_byte, MAGIC_BYTE);
        assert_eq!(decoded.registry_id, registry_id);
        assert_eq!(decoded.data, payload.as_slice());
    }
}
