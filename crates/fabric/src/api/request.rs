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
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;

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
    type Error;

    /// Routes the message to the concrete handler used to process the message.
    fn handle(
        self, owner: &str, provider: &P,
    ) -> impl Future<Output = Result<Response<Self::Output>, Self::Error>> + Send;
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
/// let response = router.owner("alice").headers(headers).handle().await;
/// ```
#[derive(Debug)]
pub struct RequestHandler<P, O, H, R, U, E>
where
    P: Provider,
    H: Headers,
    R: Handler<P>,
    U: Body,
{
    client: Client<P>,
    owner: O,
    request: R,
    headers: H,
    _phantom: PhantomData<fn() -> (U, E)>,
}

/// The router has no owner set.
#[doc(hidden)]
pub struct NoOwner;
/// The router has an owner set.
#[doc(hidden)]
pub struct OwnerSet(String);

impl<P, R, U, E> RequestHandler<P, NoOwner, NoHeaders, R, U, E>
where
    P: Provider,
    R: Handler<P>,
    U: Body,
{
    /// Create a new `RequestHandler` instance.
    #[must_use]
    pub fn new(client: Client<P>, request: R) -> Self {
        Self {
            client,
            owner: NoOwner,
            request,
            headers: NoHeaders,
            _phantom: PhantomData,
        }
    }
}

// No owner set.
impl<P, H, R, U, E> RequestHandler<P, NoOwner, H, R, U, E>
where
    P: Provider,
    H: Headers,
    R: Handler<P>,
    U: Body,
{
    /// Set the owner (tenant).
    #[must_use]
    pub fn owner(self, owner: impl Into<String>) -> RequestHandler<P, OwnerSet, H, R, U, E> {
        RequestHandler {
            client: self.client,
            owner: OwnerSet(owner.into()),
            request: self.request,
            headers: self.headers,
            _phantom: PhantomData,
        }
    }
}

/// [`NoHeaders`] headers set.
impl<P, O, R, U, E> RequestHandler<P, O, NoHeaders, R, U, E>
where
    P: Provider,
    R: Handler<P>,
    U: Body,
{
    /// Set request headers.
    #[must_use]
    pub fn headers<H: Headers>(self, headers: H) -> RequestHandler<P, O, H, R, U, E> {
        RequestHandler {
            client: self.client,
            owner: self.owner,
            request: self.request,
            headers,
            _phantom: PhantomData,
        }
    }
}

// Owner set, maybe headers set: request can be routed to it's handler.
impl<P, H, R, U, E> RequestHandler<P, OwnerSet, H, R, U, E>
where
    P: Provider,
    H: Headers,
    R: Handler<P, Output = U, Error = E>,
    U: Body,
    E: Send,
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
    pub async fn handle(self) -> Result<Response<U>, E> {
        self.request.handle(&self.owner.0, &self.client.provider).await
    }
}

// Implement [`IntoFuture`] so that the request can be awaited directly (without
// needing to call the `handle` method).
impl<P, H, R, U, E> IntoFuture for RequestHandler<P, OwnerSet, H, R, U, E>
where
    P: Provider + 'static,
    H: Headers + 'static,
    R: Handler<P, Output = U, Error = E> + Send + 'static,
    U: Body + 'static,
    E: Send + 'static,
{
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<Response<U>, E>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.handle())
    }
}
