//! # API
//!
//! The api module provides the entry point to the public API. Requests are routed
//! to the appropriate handler for processing, returning a response that can
//! be serialized to a JSON object or directly to HTTP.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use fabric::{Body, Client, Headers};
//!
//! // Create a client (typestate builder)
//! let client = Client::new("alice").provider(provider);
//!
//! // Simple request without headers
//! let response = client.request(my_request).await?;
//!
//! // Request with headers
//! let response = client.request(my_request).headers(my_headers).await?;
//! ```

mod into_http;
mod reply;
mod request;

use std::fmt::Debug;
use std::sync::Arc;

pub use self::into_http::*;
pub use self::reply::*;
pub use self::request::*;

pub trait Provider: Send + Sync {}

impl<T> Provider for T where T: Send + Sync {}

/// Typestate marker indicating a [`Client`] has not yet been configured with a provider.
///
/// Calling `.provider(...)` transitions `Client<NoProvider>` into `Client<Arc<P>>`, and
/// request methods are only available on the configured state.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProvider;

/// Build an API client to execute the request.
///
/// The client is the main entry point for making API requests. It holds
/// the provider configuration and provides methods to create the request
/// router.
#[derive(Clone, Debug)]
pub struct Client<P> {
    /// The owning tenant/namespace.
    owner: Arc<str>,

    /// The provider to use while handling of the request.
    provider: P,
}

impl Client<NoProvider> {
    /// Start building a new `Client` by setting the owner.
    #[must_use]
    pub fn new(owner: impl Into<String>) -> Self {
        Self {
            owner: Arc::<str>::from(owner.into()),
            provider: NoProvider,
        }
    }

    /// Finish building the client by providing the provider implementation.
    #[must_use]
    pub fn provider<P: Provider>(self, provider: P) -> Client<Arc<P>> {
        Client {
            owner: self.owner,
            provider: Arc::new(provider),
        }
    }
}

impl<P: Provider> Client<Arc<P>> {
    /// Create a new [`RequestHandler`] with no headers.
    #[must_use]
    pub fn request<R>(&self, request: R) -> RequestHandler<P, NoHeaders, R>
    where
        R: Handler<P>,
    {
        RequestHandler::new(self.clone(), request)
    }
}

/// The `Headers` trait is used to restrict the types able to implement
/// request headers.
///
/// It is also optionally used by [`crate::api::Reply`] to emit typed response headers
/// into a concrete `http::HeaderMap`.
pub trait Headers: Debug + Send + Sync {
    /// Apply typed headers into a concrete HTTP header map.
    ///
    /// Default implementation is a no-op so header types that are only used as typed
    /// request metadata don't need to implement it.
    fn apply(&self, _headers: &mut http::HeaderMap) {}
}

/// Implement empty headers for use by handlers that do not require headers.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHeaders;
impl Headers for NoHeaders {}

/// The `Body` trait is used to restrict the types able to implement
/// request body. It is implemented by all `xxxRequest` types.
pub trait Body: Debug + Send + Sync {}
impl<T> Body for T where T: Debug + Send + Sync {}
