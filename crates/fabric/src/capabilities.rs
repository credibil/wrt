//! # Provider
//!
//! Provider defines external data interfaces for the crate.

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;

/// The `HttpRequest` trait defines the behavior for fetching data from a source.
pub trait HttpRequest: Send + Sync {
    /// Make outbound HTTP request.
    fn fetch<T>(&self, request: Request<T>) -> impl Future<Output = Result<Response<Bytes>>> + Send
    where
        T: Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>;
}

/// The `Config` trait is used by implementers to provide configuration from
/// WASI-guest to dependent crates.
pub trait Config: Send + Sync {
    /// Request configuration setting.
    fn get(&self, key: &str) -> impl Future<Output = Result<String>> + Send;
}

/// Message represents a message to be published.
#[derive(Clone, Debug)]
pub struct Message {
    pub payload: Vec<u8>,
    pub headers: HashMap<String, String>,
}

impl Message {
    #[must_use]
    pub fn new(payload: &[u8]) -> Self {
        Self {
            payload: payload.to_vec(),
            headers: HashMap::new(),
        }
    }
}

/// The `Publisher` trait defines the message publishing behavior.
pub trait Publisher: Send + Sync {
    /// Make outbound HTTP request.
    fn send(&self, topic: &str, message: &Message) -> impl Future<Output = Result<()>> + Send;
}

/// The `StateStore` trait defines the behavior storing and retrieving train state.
pub trait StateStore: Send + Sync {
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

    fn set(
        &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

    fn delete(&self, key: &str) -> impl Future<Output = Result<()>> + Send;
}

pub trait Identity: Send + Sync {
    /// Get the unique identifier for the entity.
    fn access_token(&self) -> impl Future<Output = Result<String>> + Send;
}
