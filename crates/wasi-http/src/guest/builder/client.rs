use crate::guest::builder::request::Request;
use crate::guest::builder::request::RequestBuilder;
use crate::guest::builder::response::Response;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use http::Uri;
use http::header::{Entry, USER_AGENT};
use http::{HeaderMap, HeaderValue, Method};
use http_body_util::BodyExt;
use http_body_util::{Empty, Full};
use std::convert::TryInto;
use std::sync::Arc;
use wasip3::http::handler;
use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

/// A client for making HTTP requests.
#[derive(Default, Debug, Clone)]
pub struct Client {
    config: Arc<Config>,
}

/// A builder to configure a [`Client`].
#[derive(Default, Debug)]
pub struct ClientBuilder {
    config: Config,
}

impl Client {
    /// Constructs a new [`Client`].
    pub fn new() -> Self {
        Client::builder().build().expect("Client::new()")
    }

    /// Constructs a new [`ClientBuilder`].
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Convenience method to make a `GET` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn get(&self, url: impl Into<Uri>) -> RequestBuilder {
        self.request(Method::GET, url.into())
    }

    /// Convenience method to make a `POST` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn post(&self, url: impl Into<Uri>) -> RequestBuilder {
        self.request(Method::POST, url.into())
    }

    /// Convenience method to make a `PUT` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn put(&self, url: impl Into<Uri>) -> RequestBuilder {
        self.request(Method::PUT, url.into())
    }

    /// Convenience method to make a `PATCH` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn patch(&self, url: impl Into<Uri>) -> RequestBuilder {
        self.request(Method::PATCH, url.into())
    }

    /// Convenience method to make a `DELETE` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn delete(&self, url: impl Into<Uri>) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Convenience method to make a `HEAD` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn head(&self, url: impl Into<Uri>) -> RequestBuilder {
        self.request(Method::HEAD, url.into())
    }

    /// Start building a `Request` with the `Method` and `Url`.
    ///
    /// Returns a `RequestBuilder`, which will allow setting headers and
    /// request body before sending.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn request(&self, method: Method, url: impl Into<Uri>) -> RequestBuilder {
        let req = Request::new(method, url.into());
        RequestBuilder::new(self.clone(), Ok(req))
    }

    /// Executes a `Request`.
    ///
    /// A `Request` can be built manually with `Request::new()` or obtained
    /// from a RequestBuilder with `RequestBuilder::build()`.
    ///
    /// You should prefer to use the `RequestBuilder` and
    /// `RequestBuilder::send()`.
    ///
    /// # Errors
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    pub async fn execute(&self, request: Request) -> Result<Response> {
        self.execute_request(request).await
    }

    /// Merge [`Request`] headers with default headers set in [`Config`]
    fn merge_default_headers(&self, req: &mut Request) {
        let headers: &mut HeaderMap = req.headers_mut();
        // Insert without overwriting existing headers
        for (key, value) in self.config.headers.iter() {
            if let Entry::Vacant(entry) = headers.entry(key) {
                entry.insert(value.clone());
            }
        }
    }

    pub(super) async fn execute_request(&self, mut req: Request) -> Result<Response> {
        self.merge_default_headers(&mut req);
        fetch(req).await
    }
}

async fn fetch(req: Request) -> Result<Response> {
    // build convertable http::RRequest
    let mut builder =
        http::Request::builder().method(req.method().clone()).uri(req.url().to_string());

    for (name, value) in req.headers() {
        builder = builder.header(name, value.clone());
    }

    // convert to wasi request
    let wasi_req = if let Some(body) = req.body() {
        let bytes = body.as_bytes().unwrap().to_vec();
        let http_req = builder.body(Full::new(Bytes::from(bytes)))?;
        http_into_wasi_request(http_req)?
    } else {
        let http_req = builder.body(Empty::<Bytes>::new())?;
        http_into_wasi_request(http_req)?
    };

    // call wasip3 handler
    let wasi_resp = handler::handle(wasi_req).await?;

    // convert back to http response
    let http_resp = http_from_wasi_response(wasi_resp)?;

    // collect body and convert back to http::Response
    let (parts, body) = http_resp.into_parts();
    let collected = body.collect().await?;
    let bytes = collected.to_bytes();
    let response = http::Response::from_parts(parts, bytes);

    Ok(Response::new(response, req.url().clone()))
}

impl ClientBuilder {
    /// Return a new `ClientBuilder`.
    pub fn new() -> Self {
        ClientBuilder {
            config: Config::default(),
        }
    }

    /// Returns a 'Client' that uses this ClientBuilder configuration
    pub fn build(mut self) -> Result<Client> {
        if let Some(err) = self.config.error {
            return Err(err);
        }

        let config = std::mem::take(&mut self.config);
        Ok(Client {
            config: Arc::new(config),
        })
    }

    /// Sets the `User-Agent` header to be used by this client.
    pub fn user_agent<V>(mut self, value: V) -> ClientBuilder
    where
        V: TryInto<HeaderValue>,
        V::Error: Into<http::Error>,
    {
        match value.try_into() {
            Ok(value) => {
                self.config.headers.insert(USER_AGENT, value);
            }
            Err(e) => {
                self.config.error = Some(anyhow!(e.into()));
            }
        }
        self
    }

    /// Sets the default headers for every request
    pub fn default_headers(mut self, headers: HeaderMap) -> ClientBuilder {
        for (key, value) in headers.iter() {
            self.config.headers.insert(key, value.clone());
        }
        self
    }
}

#[derive(Default, Debug)]
struct Config {
    headers: HeaderMap,
    error: Option<anyhow::Error>,
}
