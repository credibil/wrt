//! # API
//!
//! The api module provides the entry point to the public API. Requests are routed
//! to the appropriate handler for processing, returning a response that can
//! be serialized to a JSON object or directly to HTTP.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use common::api::{Client, Body, Headers};
//!
//! // Create a client
//! let client = Client::new(provider);
//!
//! // Simple request without headers
//! let response = client.request(my_request).owner("alice").await?;
//!
//! // Request with headers
//! let response = client.request(my_request).owner("alice").headers(my_headers).await?;
//! ```

mod request;
mod response;

use std::fmt::Debug;
use std::sync::Arc;

pub use self::request::*;
pub use self::response::*;

pub trait Provider: Send + Sync {}

impl<T> Provider for T where T: Send + Sync {}

/// Build an API client to execute the request.
///
/// The client is the main entry point for making API requests. It holds
/// the provider configuration and provides methods to create the request
/// router.
#[derive(Debug)]
pub struct Client<P: Provider> {
    /// The provider to use while handling of the request.
    provider: Arc<P>,
}

impl<P: Provider> Clone for Client<P> {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
        }
    }
}

impl<P: Provider> Client<P> {
    /// Create a new `Client`.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }
}

impl<P: Provider> Client<P> {
    /// Create a new [`RequestHandler`] with no headers.
    #[must_use]
    pub fn request<B: Body + Handler<P, Output = U, Error = E>, U: Body, E>(
        &self, body: B,
    ) -> RequestHandler<P, NoOwner, NoHeaders, B> {
        RequestHandler::new(self.clone(), body)
    }
}

/// The `Headers` trait is used to restrict the types able to implement
/// request headers.
pub trait Headers: Clone + Debug + Send + Sync {}

/// Implement empty headers for use by handlers that do not require headers.
#[derive(Clone, Debug)]
pub struct NoHeaders;
impl Headers for NoHeaders {}

/// The `Body` trait is used to restrict the types able to implement
/// request body. It is implemented by all `xxxRequest` types.
pub trait Body: Clone + Debug + Send + Sync {}
impl<T> Body for T
where
    T: Clone + Debug + Send + Sync,
{
    // fn from_bytes(bytes: &[u8]) -> Result<Self> {
    //     serde_json::from_slice(bytes)
    // }

    // fn to_bytes(self) -> Vec<u8> {
    //     serde_json::to_vec(&self).unwrap()
    // }
}

// impl<B: Body> From<B> for Request<B> {
//     fn from(body: B) -> Self {
//         Self {
//             body,
//             headers: NoHeaders,
//         }
//     }
// }
