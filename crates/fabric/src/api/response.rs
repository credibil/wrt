use std::ops::Deref;

use bytes::Bytes;
use http::StatusCode;

use crate::api::{Body, Headers, NoHeaders};

/// Top-level response data structure common to all handlers.
#[derive(Debug)]
pub struct Response<B, H = NoHeaders>
where
    H: Headers,
    B: Body,
{
    /// Response HTTP status code.
    pub status: StatusCode,

    /// Response HTTP headers, if any.
    pub headers: Option<H>,

    /// The endpoint-specific response.
    pub body: B,
}

impl<B: Body> Response<B> {
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

impl<B: Body, H: Headers> Response<B, H> {
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

impl<B: Body> From<B> for Response<B> {
    fn from(body: B) -> Self {
        Self {
            status: StatusCode::OK,
            headers: None,
            body,
        }
    }
}

impl<B: Body> Deref for Response<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

/// Trait for converting a `Result` into an HTTP response.
pub trait IntoHttp<B>
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
{
    /// Convert into an HTTP response.
    fn into_http(self) -> http::Response<B>;
}

// impl<B, E> IntoHttp for Result<Response<B>, E>
// where
//     B: Body + Serialize,
//     E: Serialize,
// {
//     type Body = http_body_util::Full<Bytes>;

//     /// Create a new reply with the given status code and body.
//     fn into_http(self) -> http::Response<Self::Body> {
//         let result = match self {
//             Ok(r) => {
//                 let body = serde_json::to_vec(&r.body).unwrap_or_default();
//                 http::Response::builder()
//                     .status(r.status)
//                     .header(header::CONTENT_TYPE, "application/json")
//                     .body(Self::Body::from(body))
//             }
//             Err(e) => {
//                 let body = serde_json::to_vec(&e).unwrap_or_default();
//                 http::Response::builder()
//                     .status(StatusCode::BAD_REQUEST)
//                     .header(header::CONTENT_TYPE, "application/json")
//                     .body(Self::Body::from(body))
//             }
//         };
//         result.unwrap_or_default()
//     }
// }
