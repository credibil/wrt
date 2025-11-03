//! # WASI Key-Value
//!
//! This module implements a runtime service for `wasi:keyvalue`
//! (<https://github.com/WebAssembly/wasi-keyvalue>).

#[cfg(target_arch = "wasm32")]
mod guest;
#[cfg(target_arch = "wasm32")]
pub use guest::*;

#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
pub use host::*;
use serde::{Deserialize, Serialize};

/// A type that allows for transfer of value types between guest and host where
/// the implementation may be able to manage value lifetime for an individual
/// key.
///
/// If the underlying store does not support a key-level TTL, the timestamp
/// could be used by a guest to implement expiration behavior.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TtlValue {
    /// The value bytes.
    pub value: Vec<u8>,

    /// The time-to-live in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,

    /// A timestamp of the requested storage time in seconds since the UNIX
    /// epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_seconds: Option<u64>,
}

impl TryFrom<Vec<u8>> for TtlValue {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let ttl_value = serde_json::from_slice(&value)
            .map_err(|e| anyhow::anyhow!("failed to deserialize TtlValue: {e}"))?;
        Ok(ttl_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttlvalue_tryfrom_valid() {
        let value = vec![1, 2, 3, 4];
        let ttl = Some(60u64);
        let timestamp = Some(1_700_000_000u64);
        let ttl_value = TtlValue {
            value: value.clone(),
            ttl_seconds: ttl,
            timestamp_seconds: timestamp,
        };
        let json = serde_json::to_vec(&ttl_value).unwrap();
        let parsed = TtlValue::try_from(json).unwrap();
        assert_eq!(parsed.value, value);
        assert_eq!(parsed.ttl_seconds, ttl);
        assert_eq!(parsed.timestamp_seconds, timestamp);
    }

    #[test]
    fn ttlvalue_tryfrom_missing_optional() {
        let value = vec![5, 6, 7];
        let ttl_value = TtlValue {
            value: value.clone(),
            ttl_seconds: None,
            timestamp_seconds: None,
        };
        let json = serde_json::to_vec(&ttl_value).unwrap();
        let parsed = TtlValue::try_from(json).unwrap();
        assert_eq!(parsed.value, value);
        assert_eq!(parsed.ttl_seconds, None);
        assert_eq!(parsed.timestamp_seconds, None);
    }

    #[test]
    fn ttlvalue_tryfrom_invalid_json() {
        let invalid = b"not a json".to_vec();
        let result = TtlValue::try_from(invalid);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to deserialize TtlValue"));
    }

    #[test]
    fn ttlvalue_tryfrom_wrong_type() {
        // JSON for a different type (e.g., a string)
        let json = serde_json::to_vec(&"just a string").unwrap();
        let result = TtlValue::try_from(json);
        result.unwrap_err();
    }
}
