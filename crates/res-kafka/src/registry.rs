use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use credibil_error::Error as CredibilError;
use jsonschema::validate;
use reqwest::StatusCode;
use schema_registry_client::rest::apis::Error as SchemaRegistryError;
use schema_registry_client::rest::client_config::ClientConfig as RegistryConfig;
use schema_registry_client::rest::schema_registry_client::{Client, SchemaRegistryClient};
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::time;
use tracing::{error, instrument, warn};

use crate::RegistryOptions;

type SchemaMap = HashMap<String, Option<(i32, Value)>>;
static SERVICE: &str = "schema-registry";
/// Schema Registry client with caching
#[derive(Clone)]
pub struct Registry {
    client: Option<SchemaRegistryClient>,
    schemas: Arc<Mutex<SchemaMap>>,
}

/// Endianness byte used in schema registry payloads
const BIG_ENDIAN: u8 = 0;

impl Registry {
    /// Create a new Schema Registry client
    #[must_use]
    pub fn new(options: RegistryOptions) -> Self {
        let mut config = RegistryConfig::new(vec![options.url.clone()]);
        config.basic_auth = Some((options.api_key, Some(options.api_secret)));

        let sr_client = Self {
            client: Some(SchemaRegistryClient::new(config)),
            schemas: Arc::new(Mutex::new(HashMap::new())),
        };
        sr_client.start_cache_cleaner(options.cache_ttl_secs);

        sr_client
    }

    /// Serialize payload to JSON with optional schema registry
    #[instrument(name = "registry-validate-encode-json", skip(self, buffer))]
    pub async fn validate_and_encode_json(&self, topic: &str, buffer: Vec<u8>) -> Vec<u8> {
        // If schema registry is available, use it
        if self.client.is_some() {
            match self.get_or_fetch_schema(topic).await {
                Ok(Some((id, schema))) => {
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
                Ok(None) => buffer,
                Err(e) => {
                    trace(e, SERVICE, topic);
                    buffer
                }
            }
        } else {
            buffer
        }
    }

    /// Deserialize payload to JSON with optional schema registry
    #[allow(unused)]
    #[instrument(name = "registry-validate-decode-json", skip(self, buffer))]
    pub async fn validate_and_decode_json(&self, topic: &str, buffer: &[u8]) -> Vec<u8> {
        if self.client.is_some() {
            let (_id, schema) = match self.get_or_fetch_schema(topic).await {
                Ok(Some((id, schema))) => (id, schema),
                Ok(None) => {
                    return buffer.to_vec();
                }
                Err(e) => {
                    trace(e, SERVICE, topic);
                    return buffer.to_vec();
                }
            };

            let Some(decoded) = Payload::decode(buffer) else { return buffer.to_vec() };

            let payload: Value = match serde_json::from_slice(decoded.data) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Invalid JSON: {:?}", e);
                    return decoded.data.to_vec();
                }
            };

            if let Err(e) = self.validate_payload_with_schema(&schema, &payload) {
                tracing::error!("Schema validation failed: {}", e);
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

    async fn get_or_fetch_schema(
        &self, topic: &str,
    ) -> Result<Option<(i32, Value)>, CredibilError> {
        let sr = self.client.as_ref().ok_or_else(|| {
            CredibilError::ServerError("No schema registry client available".to_string())
        })?;

        let mut schemas = self.schemas.lock().await;
        if let Some(schema_entry) = schemas.get(topic) {
            Ok(schema_entry.clone())
        } else {
            let subject = format!("{topic}-value");
            let schema_response = sr.get_latest_version(&subject, None).await;
            let schema_response = match schema_response {
                Ok(s) => s,
                Err(e) => match e {
                    SchemaRegistryError::ResponseError(e) => {
                        if e.status == StatusCode::NOT_FOUND {
                            schemas.insert(topic.to_string(), None);
                            return Err(CredibilError::NotFound(format!(
                                "Schema not found for topic {topic}"
                            )));
                        }
                        return Err(CredibilError::BadGateway(format!(
                            "Error fetching schema for topic {topic}: {}",
                            e.content
                        )));
                    }
                    _ => {
                        return Err(CredibilError::ServerError(format!(
                            "Error fetching schema for topic {topic}: {e:?}"
                        )));
                    }
                },
            };

            let schema_str = schema_response
                .schema
                .as_ref()
                .ok_or_else(|| CredibilError::BadGateway("Schema string is missing".to_string()))?;

            let schema_json: Value = serde_json::from_str(schema_str)
                .map_err(|e| CredibilError::BadGateway(format!("Invalid schema JSON: {e:?}")))?;

            let registry_id = schema_response.id.ok_or_else(|| {
                CredibilError::BadGateway(format!("Registry ID missing for topic {topic}"))
            })?;
            schemas.insert(topic.to_string(), Some((registry_id, schema_json.clone())));
            drop(schemas);
            Ok(Some((registry_id, schema_json)))
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

/// Performs tracing and metrics.
fn trace(err: CredibilError, service: &str, topic: &str) {
    match err {
        CredibilError::ServiceUnavailable(description) => {
            error!(
                monotonic_counter.processing_errors = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        CredibilError::BadGateway(description) => {
            error!(
                monotonic_counter.external_errors = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        CredibilError::ServerError(description) => {
            error!(monotonic_counter.runtime_errors = 1,
                service = %service,
                description
            );
        }
        CredibilError::BadRequest(description) => {
            warn!(
                monotonic_counter.parsing_errors = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        CredibilError::Unauthorized(description) => {
            warn!(
                    monotonic_counter.authorization_errors = 1,
                    service = %service,
                    description);
        }
        CredibilError::NotFound(description) => {
            warn!(
                    monotonic_counter.not_found_errors = 1,
                    service = %service,
                    description);
        }
        CredibilError::Gone(description) => {
            warn!(
                monotonic_counter.deprecated_errors = 1,
                service = %service,
                description
            );
        }
        CredibilError::ImATeaPot(description) => {
            warn!(
                monotonic_counter.other_errors = 1,
                service = %service,
                description
            );
        }
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
        buf.push(BIG_ENDIAN);
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
    use super::*; // brings Payload, BIG_ENDIAN, MessagingError into scope

    #[test]
    fn encode_decode() {
        #[allow(clippy::cast_possible_wrap)]
        let registry_id: i32 = 0xAABB_CCDDu32 as i32;
        let payload = b"hello world".to_vec();

        // Encode
        let encoded = Payload::encode(registry_id, payload.clone());

        // Expected layout:
        // [ magic_byte ][ registry_id (4 bytes BE) ][ payload... ]
        assert_eq!(encoded[0], BIG_ENDIAN, "magic byte mismatch");

        let expected_id_bytes = registry_id.to_be_bytes();
        assert_eq!(&encoded[1..5], &expected_id_bytes, "registry id mismatch");
        assert_eq!(&encoded[5..], &payload, "payload mismatch");

        // Decode
        let decoded = Payload::decode(&encoded).expect("decode failed");

        assert_eq!(decoded.magic_byte, BIG_ENDIAN);
        assert_eq!(decoded.registry_id, registry_id);
        assert_eq!(decoded.data, payload.as_slice());
    }
}
