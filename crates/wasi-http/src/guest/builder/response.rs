use std::fmt;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use http::{HeaderMap, StatusCode, Version};

use http::Uri;
use serde::de::DeserializeOwned;

/// A Response to a submitted `Request`.
pub struct Response {
    http: http::Response<Bytes>,
    url: Box<Uri>,
}

impl Response {
    pub(crate) fn new(http: http::Response<Bytes>, url: Uri) -> Self {
        Self {
            http,
            url: Box::new(url),
        }
    }

    /// Get the `StatusCode` of this `Response`.
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.http.status()
    }

    /// Get the `Headers` of this `Response`.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.http.headers()
    }

    /// Get a mutable reference to the `Headers` of this `Response`.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.http.headers_mut()
    }

    /// Get the content-length of this response, if known.
    ///
    /// Reasons it may not be known:
    ///
    /// - The server didn't send a `content-length` header.
    /// - The response is compressed and automatically decoded (thus changing
    ///   the actual decoded length).
    pub fn content_length(&self) -> Option<u64> {
        self.headers().get(http::header::CONTENT_LENGTH)?.to_str().ok()?.parse().ok()
    }

    /// Get the final `Uri` of this `Response`.
    #[inline]
    pub fn url(&self) -> &Uri {
        &self.url
    }

    /// Get the HTTP `Version` of this `Response`.
    #[inline]
    pub fn version(&self) -> Version {
        self.http.version()
    }

    /// Try to deserialize the response body as JSON.
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json<T: DeserializeOwned>(self) -> Result<T> {
        serde_json::from_slice(&self.bytes()).map_err(|e| anyhow!("deserializing JSON: {e}"))
    }

    /// Get the response as text
    pub fn text(self) -> String {
        String::from_utf8_lossy(&self.bytes()).to_string()
    }

    /// Get the response as bytes
    pub fn bytes(self) -> Bytes {
        let http_resp = self.http;
        http_resp.into_body()
    }

    /// Convert the response into a [`wasi::http::types::IncomingBody`] resource which can
    /// then be used to stream the body.
    #[cfg(feature = "stream")]
    pub fn bytes_stream(
        &mut self,
    ) -> Result<(wasi::io::streams::InputStream, wasi::http::types::IncomingBody)> {
        let incoming_body =
            self.http.body().consume().map_err(|_| anyhow!("failed to consume response body"))?;
        let input_stream =
            incoming_body.stream().map_err(|_| anyhow!("failed to stream response body"))?;
        Ok((input_stream, incoming_body))
    }

    /// Turn a response into an error if the server returned an error.
    pub fn error_for_status(self) -> Result<Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            Err(anyhow!("HTTP error {status} for URL: {}", *self.url))
        } else {
            Ok(self)
        }
    }

    /// Turn a reference to a response into an error if the server returned an error.
    pub fn error_for_status_ref(&self) -> Result<&Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            Err(anyhow!("HTTP error {status} for URL: {}", *self.url))
        } else {
            Ok(self)
        }
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("url", self.url())
            .field("status", &self.status())
            .field("headers", self.headers())
            .finish()
    }
}

// /// Implements `std::io::Read` for a `wasi::io::streams::InputStream`.
// pub struct InputStreamReader<'a> {
//     stream: &'a mut wasi::io::streams::InputStream,
// }

// impl<'a> From<&'a mut wasi::io::streams::InputStream> for InputStreamReader<'a> {
//     fn from(stream: &'a mut wasi::io::streams::InputStream) -> Self {
//         Self { stream }
//     }
// }

// impl std::io::Read for InputStreamReader<'_> {
//     fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
//         use std::io;
//         use wasi::io::streams::StreamError;

//         let n = buf
//             .len()
//             .try_into()
//             .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
//         match self.stream.blocking_read(n) {
//             Ok(chunk) => {
//                 let n = chunk.len();
//                 if n > buf.len() {
//                     return Err(io::Error::new(
//                         io::ErrorKind::Other,
//                         "more bytes read than requested",
//                     ));
//                 }
//                 buf[..n].copy_from_slice(&chunk);
//                 Ok(n)
//             }
//             Err(StreamError::Closed) => Ok(0),
//             Err(StreamError::LastOperationFailed(e)) => {
//                 Err(io::Error::new(io::ErrorKind::Other, e.to_debug_string()))
//             }
//         }
//     }
// }
