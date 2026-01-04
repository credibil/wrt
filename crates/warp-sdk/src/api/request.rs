//! Request routing and handler traits.
//!
//! This module contains:
//! - [`Handler`]: implemented by request types to produce a [`Reply`]
//! - [`RequestHandler`]: a small request builder / router (supports `.headers(...)`)
//!
//! The main entry point is usually [`crate::Client`], re-exported from the
//! top-level `api` module.

use std::error::Error;
use std::fmt::Debug;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::Arc;

use http::HeaderMap;

use crate::api::reply::Reply;
use crate::api::{Body, Client, Provider};

/// Request-scoped context passed to [`Handler::handle`].
///
/// Bundles common request inputs (owner, provider, headers) into a single
/// parameter, making handler signatures more ergonomic and easier to extend.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a, P: Provider> {
    /// The owning tenant / namespace for the request.
    pub owner: &'a str,

    /// The provider implementation used to fulfill the request.
    pub provider: &'a P,

    /// Request headers (typed).
    pub headers: &'a HeaderMap<String>,
}

/// Request handler.
///
/// The primary role of this trait is to provide a common interface for
/// requests so they can be handled by [`handle`] method.
pub trait Handler<P: Provider> {
    /// The output type of the handler.
    type Output: Body;

    /// The error type returned by the handler.
    type Error: Error + Send + Sync + 'static;

    /// Routes the message to the concrete handler used to process the message.
    fn handle(
        self, ctx: Context<P>,
    ) -> impl Future<Output = Result<Reply<Self::Output>, Self::Error>> + Send;
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
#[must_use = "requests do nothing unless you `.await` them (or call `.handle().await`)"]
#[derive(Debug)]
pub struct RequestHandler<P, R>
where
    P: Provider,
    R: Handler<P>,
{
    request: R,
    headers: HeaderMap<String>,

    /// The owning tenant/namespace.
    owner: Arc<str>,

    /// The provider to use while handling of the request.
    provider: Arc<P>,
}

impl<P, R> RequestHandler<P, R>
where
    P: Provider,
    R: Handler<P>,
{
    /// Create a new `RequestHandler` instance.
    pub fn from_client(client: &Client<P>, request: R) -> Self {
        Self {
            request,
            headers: HeaderMap::default(),
            owner: Arc::clone(&client.owner),
            provider: Arc::clone(&client.provider),
        }
    }

    /// Set request headers.
    pub fn headers(self, headers: HeaderMap<String>) -> Self {
        Self {
            request: self.request,
            headers,
            owner: self.owner,
            provider: self.provider,
        }
    }

    /// Handle the request by routing it to the appropriate handler.
    ///
    /// # Constraints
    ///
    /// This method requires that `R` implements [`Handler<P>`].
    /// If you see an error about missing trait implementations, ensure your request type
    /// has the appropriate handler implementation.
    ///
    /// # Errors
    ///
    /// Returns the error from the underlying handler on failure.
    #[inline]
    pub async fn handle(self) -> Result<Reply<R::Output>, R::Error> {
        let ctx = Context {
            owner: &self.owner,
            provider: &*self.provider,
            headers: &self.headers,
        };
        self.request.handle(ctx).await
    }
}

// Implement [`IntoFuture`] so that the request can be awaited directly (without
// needing to call the `handle` method).
impl<P, R> IntoFuture for RequestHandler<P, R>
where
    P: Provider + 'static,
    R: Handler<P> + Send + 'static,
{
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'static>>;
    type Output = Result<Reply<R::Output>, R::Error>;

    fn into_future(self) -> Self::IntoFuture
    where
        R::Output: Body,
        R::Error: Send,
    {
        Box::pin(self.handle())
    }
}
