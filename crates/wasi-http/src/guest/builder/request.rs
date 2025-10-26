use std::convert::TryFrom;
use std::fmt;

use crate::guest::builder::body::Body;
use crate::guest::builder::client::Client;
use crate::guest::builder::response::Response;
use anyhow::{Result, anyhow};
use bytes::Bytes;
use http::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue};
use http::{HeaderMap, Uri};
use http::{Method, Request as HttpRequest, request::Parts};
use serde::Serialize;

/// A request which can be executed with `Client::execute()`.
#[derive(Debug)]
pub struct Request {
    method: Method,
    url: Uri,
    headers: HeaderMap,
    body: Option<Body>,
}

/// A builder to construct the properties of a `Request`.
#[derive(Debug)]
pub struct RequestBuilder {
    client: Client,
    request: Result<Request>,
}

impl Request {
    /// Constructs a new request.
    #[inline]
    pub fn new(method: Method, url: impl Into<Uri>) -> Self {
        Request {
            method,
            url: url.into(),
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// Get the method.
    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get a mutable reference to the method.
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    /// Get the url.
    #[inline]
    pub fn url(&self) -> &Uri {
        &self.url
    }

    /// Get a mutable reference to the url.
    #[inline]
    pub fn url_mut(&mut self) -> &mut Uri {
        &mut self.url
    }

    /// Get the headers.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get a mutable reference to the headers.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Get the body.
    #[inline]
    pub fn body(&self) -> Option<&Body> {
        self.body.as_ref()
    }

    /// Get a mutable reference to the body.
    #[inline]
    pub fn body_mut(&mut self) -> &mut Option<Body> {
        &mut self.body
    }

    /// Attempts to clone the `Request`.
    ///
    /// None is returned if a body is which can not be cloned.
    pub fn try_clone(&self) -> Option<Request> {
        let body = match self.body.as_ref() {
            Some(body) => Some(body.try_clone()?),
            None => None,
        };

        Some(Self {
            method: self.method.clone(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            body,
        })
    }
}

impl RequestBuilder {
    pub(crate) fn new(client: Client, request: Result<Request>) -> RequestBuilder {
        RequestBuilder { client, request }
    }

    /// Assemble a builder starting from an existing `Client` and a `Request`.
    pub fn from_parts(client: Client, request: Request) -> RequestBuilder {
        RequestBuilder {
            client,
            request: Ok(request),
        }
    }

    /// Modify the query string of the URL.
    ///
    /// Modifies the URL of this request, adding the parameters provided.
    /// This method appends and does not overwrite. This means that it can
    /// be called multiple times and that existing query parameters are not
    /// overwritten if the same key is used. The key will simply show up
    /// twice in the query string.
    /// Calling `.query([("foo", "a"), ("foo", "b")])` gives `"foo=a&foo=b"`.
    ///
    /// # Note
    /// This method does not support serializing a single key-value
    /// pair. Instead of using `.query(("key", "val"))`, use a sequence, such
    /// as `.query(&[("key", "val")])`. It's also possible to serialize structs
    /// and maps into a key-value pair.
    ///
    /// # Errors
    /// This method will fail if the object you provide cannot be serialized
    /// into a query string.
    pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> RequestBuilder {
        let mut error = None;

        if let Ok(ref mut req) = self.request {
            let url = req.url_mut();
            let mut pairs = url.query();

            // let serializer = serde_urlencoded::Serializer::new(&mut pairs);
            // if let Err(err) = query.serialize(serializer) {
            //     error = Some(anyhow!(err));
            // }
        }

        if let Ok(ref mut req) = self.request {
            if let Some("") = req.url().query() {
                *req.url_mut() = Uri::from_static("");
            }
        }

        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Send a form body.
    ///
    /// Sets the body to the url encoded serialization of the passed value,
    /// and also sets the `Content-Type: application/x-www-form-urlencoded`
    /// header.
    ///
    /// # Errors
    ///
    /// This method fails if the passed value cannot be serialized into
    /// url encoded format
    pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> RequestBuilder {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match serde_urlencoded::to_string(form) {
                Ok(body) => {
                    req.headers_mut().insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    *req.body_mut() = Some(body.into());
                }
                Err(err) => error = Some(anyhow!(err)),
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Set the request json
    pub fn json<T: Serialize + ?Sized>(mut self, json: &T) -> RequestBuilder {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match serde_json::to_vec(json) {
                Ok(body) => {
                    req.headers_mut()
                        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                    *req.body_mut() = Some(body.into());
                }
                Err(err) => error = Some(anyhow!(err)),
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Enable HTTP bearer authentication.
    pub fn bearer_auth<T>(self, token: T) -> RequestBuilder
    where
        T: fmt::Display,
    {
        let header_value = format!("Bearer {token}");
        self.header(AUTHORIZATION, header_value)
    }

    /// Set the request body.
    pub fn body<T: Into<Body>>(mut self, body: T) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            req.body = Some(body.into());
        }
        self
    }

    /// Add a `Header` to this Request.
    pub fn header<K, V>(mut self, key: K, value: V) -> RequestBuilder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match <HeaderName as TryFrom<K>>::try_from(key) {
                Ok(key) => match <HeaderValue as TryFrom<V>>::try_from(value) {
                    Ok(value) => {
                        req.headers_mut().append(key, value);
                    }
                    Err(e) => error = Some(anyhow!(e.into())),
                },
                Err(e) => error = Some(anyhow!(e.into())),
            };
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Add a set of Headers to the existing ones on this Request.
    ///
    /// The headers will be merged in to any already set.
    pub fn headers(mut self, headers: HeaderMap) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            for (key, value) in headers.iter() {
                req.headers_mut().insert(key, value.clone());
            }
        }
        self
    }

    /// Build a `Request`, which can be inspected, modified and executed with
    /// `Client::execute()`.
    pub fn build(self) -> Result<Request> {
        self.request
    }

    /// Build a `Request`, which can be inspected, modified and executed with
    /// `Client::execute()`.
    ///
    /// This is similar to [`RequestBuilder::build()`], but also returns the
    /// embedded `Client`.
    pub fn build_split(self) -> (Client, Result<Request>) {
        (self.client, self.request)
    }

    /// Constructs the Request and sends it to the target URL, returning a
    /// future Response.
    ///
    /// # Errors
    ///
    /// This method fails if there was an error while sending request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use wasi_http::Error;
    /// #
    /// # async fn run() -> Result<(), Error> {
    /// let response = wasi_http::Client::new()
    ///     .get("https://hyper.rs")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(self) -> Result<Response> {
        let req = self.request?;
        self.client.execute_request(req).await
    }

    /// Attempt to clone the RequestBuilder.
    ///
    /// `None` is returned if the RequestBuilder can not be cloned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wasi_http::Error;
    /// #
    /// # fn run() -> Result<(), Error> {
    /// let client = wasi_http::Client::new();
    /// let builder = client.post("http://httpbin.org/post")
    ///     .body("from a &str!");
    /// let clone = builder.try_clone();
    /// assert!(clone.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_clone(&self) -> Option<RequestBuilder> {
        self.request.as_ref().ok().and_then(|req| req.try_clone()).map(|req| RequestBuilder {
            client: self.client.clone(),
            request: Ok(req),
        })
    }
}

impl<T> TryFrom<HttpRequest<T>> for Request
where
    T: Into<Body>,
{
    type Error = anyhow::Error;

    fn try_from(req: HttpRequest<T>) -> Result<Self> {
        let (parts, body) = req.into_parts();
        let Parts {
            method, uri, headers, ..
        } = parts;

        Ok(Request {
            method,
            url: uri,
            headers,
            body: Some(body.into()),
        })
    }
}

impl TryFrom<Request> for HttpRequest<Body> {
    type Error = anyhow::Error;

    fn try_from(req: Request) -> Result<Self> {
        let Request {
            method,
            url,
            headers,
            body,
            ..
        } = req;

        let mut req = HttpRequest::builder()
            .method(method)
            .uri(url)
            .body(body.unwrap_or_else(|| Body::from(Bytes::default())))?;

        *req.headers_mut() = headers;
        Ok(req)
    }
}
