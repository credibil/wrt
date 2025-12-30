use bytes::Bytes;
use http::{StatusCode, header};
use std::fmt::Debug;
use std::ops::Deref;

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
    fn into_body(self) -> (Bytes, String);
}

impl<T: IntoBody> IntoHttp for Reply<T> {
    type Body = Full<Bytes>;

    fn into_http(self) -> HttpResponse<Self::Body> {
        let (body, content_type) = self.body.into_body();
        let response = http::Response::builder()
            .status(self.status)
            .header(header::CONTENT_TYPE, content_type)
            .body(Self::Body::from(body));
        response.unwrap_or_default()
    }
}

use serde::Serialize;

impl<T: Body + Serialize> IntoBody for T {
    fn into_body(self) -> (Bytes, String) {
        let body = serde_json::to_vec(&self).unwrap_or_default();
        (Bytes::from(body), "application/json".to_string())
    }
}
