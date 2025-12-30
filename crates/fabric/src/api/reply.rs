use bytes::Bytes;
use http::{StatusCode, header};
use std::fmt::Debug;
use std::ops::Deref;

use anyhow::Result;
use http::HeaderValue;
use http_body_util::Full;

use crate::api::{Body, Headers, NoHeaders};

/// Top-level response data structure common to all handlers.
#[derive(Debug)]
pub struct Reply<B, H = NoHeaders>
where
    H: Headers,
    B: Body,
{
    /// Reply HTTP status code.
    pub status: StatusCode,

    /// Reply HTTP headers, if any.
    pub headers: Option<H>,

    /// The endpoint-specific response.
    pub body: B,
}

impl<B: Body> Reply<B> {
    /// Create a success response
    #[must_use]
    pub const fn ok(body: B) -> Self {
        Self {
            status: StatusCode::OK,
            headers: None,
            body,
        }
    }

    /// Create a created response (201)
    #[must_use]
    pub const fn created(body: B) -> Self {
        Self {
            status: StatusCode::CREATED,
            headers: None,
            body,
        }
    }

    /// Create an accepted response (202)
    #[must_use]
    pub const fn accepted(body: B) -> Self {
        Self {
            status: StatusCode::ACCEPTED,
            headers: None,
            body,
        }
    }

    /// Check if response is successful (2xx)
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

impl<B: Body, H: Headers> Reply<B, H> {
    /// Create a success response with a specific status code.
    #[must_use]
    pub const fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add headers to the response.
    #[must_use]
    pub fn headers(mut self, headers: H) -> Self {
        self.headers = Some(headers);
        self
    }
}

impl<B: Body> From<B> for Reply<B> {
    fn from(body: B) -> Self {
        Self {
            status: StatusCode::OK,
            headers: None,
            body,
        }
    }
}

impl<B: Body> Deref for Reply<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

pub type HttpResponse<B = Full<Bytes>> = http::Response<B>;

/// Trait for converting a `Result` into an HTTP response.
pub trait IntoHttp {
    /// The body type of the response.
    type Body: http_body::Body<Data = Bytes> + Send;

    /// Convert into an HTTP response.
    fn into_http(self) -> HttpResponse<Self::Body>;
}

/// Trait for converting a `Reply` into a body + content type.
pub trait IntoBody: Body {
    /// Convert into a body + content type.
    ///
    /// # Errors
    ///
    /// Returns an error if the body cannot be encoded (for example, if JSON
    /// serialization fails).
    fn into_body(self) -> Result<(Bytes, HeaderValue)>;
}

impl<T, H> IntoHttp for Reply<T, H>
where
    T: IntoBody,
    H: Headers,
{
    type Body = Full<Bytes>;

    fn into_http(self) -> HttpResponse<Self::Body> {
        let (bytes, content_type) = match self.body.into_body() {
            Ok(v) => v,
            Err(e) => return server_error(e.to_string()),
        };

        let mut builder = http::Response::builder().status(self.status);

        if let Some(hm) = builder.headers_mut() {
            if let Some(h) = self.headers.as_ref() {
                h.apply(hm);
            }
            if !hm.contains_key(header::CONTENT_TYPE) {
                hm.insert(header::CONTENT_TYPE, content_type);
            }
        }

        match builder.body(Self::Body::from(bytes)) {
            Ok(resp) => resp,
            Err(e) => server_error(e.to_string()),
        }
    }
}

fn server_error(message: impl Into<String>) -> HttpResponse<Full<Bytes>> {
    let mut resp = http::Response::new(Full::from(Bytes::from(message.into())));
    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    resp
}


