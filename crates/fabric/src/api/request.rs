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

use std::fmt::Debug;
use std::future::Future;

use crate::api::{Provider, Response};

/// A request to process.
#[derive(Clone, Debug)]
pub struct Request<B, H = NoHeaders>
where
    H: Headers,
    B: Body,
{
    /// Headers associated with this request.
    pub headers: H,

    /// The request to process.
    pub body: B,
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
impl<T> Body for T where T: Clone + Debug + Send + Sync {}

impl<B: Body> From<B> for Request<B> {
    fn from(body: B) -> Self {
        Self {
            body,
            headers: NoHeaders,
        }
    }
}

/// Request handler.
///
/// The primary role of this trait is to provide a common interface for
/// requests so they can be handled by [`handle`] method.
pub trait Handler<B, P>
where
    P: Provider,
    B: Body,
{
    /// The error type returned by the handler.
    type Error;

    /// Routes the message to the concrete handler used to process the message.
    fn handle(
        self, owner: &str, provider: &P,
    ) -> impl Future<Output = Result<Response<B>, Self::Error>> + Send;
}
