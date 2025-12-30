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
//! // Create a client (typestate builder)
//! let client = Client::new("alice").provider(provider);
//!
//! // Simple request without headers
//! let response = client.request(my_request).await?;
//!
//! // Request with headers
//! let response = client.request(my_request).headers(my_headers).await?;
//! ```

use std::error::Error;
use std::fmt::Debug;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::Arc;

use crate::api::response::Response;
use crate::api::{Body, Client, Headers, NoHeaders, Provider};

/// Request handler.
///
/// The primary role of this trait is to provide a common interface for
/// requests so they can be handled by [`handle`] method.
pub trait Handler<P: Provider> {
    /// The output type of the handler.
    type Output: Body;

    /// The error type returned by the handler.
    type Error: Error + Send;

    /// Routes the message to the concrete handler used to process the message.
    fn handle(
        self, owner: &str, provider: &P,
    ) -> impl Future<Output = Result<Response<Self::Output>, Self::Error>> + Send;

    // fn handle_with_headers<H: Headers>(
    //     self, owner: &str, provider: &P, headers: H,
    // ) -> impl Future<Output = Result<Response<Self::Output>, Self::Error>> + Send;
}

/// Request router.
///
/// The router is used to route a request to the appropriate handler with the
/// owner and headers set.
///
/// # Example Usage
///
/// ```rust,ignore
/// let router = RequestHandler::new(client, body);
/// let response = router.headers(headers).handle().await;
/// ```
#[derive(Debug)]
pub struct RequestHandler<P, H, R>
where
    P: Provider,
    H: Headers,
    R: Handler<P>,
{
    client: Client<Arc<P>>,
    request: R,
    headers: H,
}

impl<P, R> RequestHandler<P, NoHeaders, R>
where
    P: Provider,
    R: Handler<P>,
{
    /// Create a new `RequestHandler` instance.
    #[must_use]
    pub const fn new(client: Client<Arc<P>>, request: R) -> Self {
        Self {
            client,
            request,
            headers: NoHeaders,
        }
    }
}

/// [`NoHeaders`] headers set.
impl<P, R> RequestHandler<P, NoHeaders, R>
where
    P: Provider,
    R: Handler<P>,
{
    /// Set request headers.
    #[must_use]
    pub fn headers<H: Headers>(self, headers: H) -> RequestHandler<P, H, R> {
        RequestHandler {
            client: self.client,
            request: self.request,
            headers,
        }
    }
}

// Route request to it's handler.
impl<P, H, R> RequestHandler<P, H, R>
where
    P: Provider,
    H: Headers,
    R: Handler<P>,
{
    /// Handle the request by routing it to the appropriate handler.
    ///
    /// # Constraints
    ///
    /// This method requires that `Request<R, H>` implements `Handler<U, P, Error = E>`.
    /// If you see an error about missing trait implementations, ensure your request
    /// type has the appropriate handler implementation.
    ///
    /// # Errors
    ///
    /// Returns the error from the underlying handler on failure.
    #[inline]
    pub async fn handle(self) -> Result<Response<R::Output>, R::Error>
    where
        R::Output: Body,
        R::Error: Send,
    {
        self.request.handle(&self.client.owner, &self.client.provider).await
    }
}

// Implement [`IntoFuture`] so that the request can be awaited directly (without
// needing to call the `handle` method).
impl<P, H, R> IntoFuture for RequestHandler<P, H, R>
where
    P: Provider + 'static,
    H: Headers + 'static,
    R: Handler<P> + Send + 'static,
{
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<Response<R::Output>, R::Error>;

    fn into_future(self) -> Self::IntoFuture
    where
        R::Output: Body,
        R::Error: Send,
    {
        Box::pin(self.handle())
    }
}
