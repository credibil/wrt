use anyhow::{Result, anyhow};
use bytes::Bytes;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http::{HeaderMap, HeaderName, Method, Response};
use http_body_util::{Empty, Full};
use serde::Serialize;

use crate::guest::client::uri::UriLike;
use crate::guest::outgoing;

pub trait Safe: Send + Sync {}
impl<T: Send + Sync> Safe for T {}

#[derive(Debug)]
pub struct RequestBuilder<B, J, F> {
    method: Method,
    uri: UriLike,
    headers: HeaderMap<String>,
    query: Option<String>,
    cache: Option<String>,
    identity: Option<String>,
    body: B,
    json: J,
    form: F,
}

/// Builder has no body.
#[doc(hidden)]
pub struct NoBody;
/// Builder has a body.
#[doc(hidden)]
pub struct HasBody(Vec<u8>);

/// Builder has no json.
#[doc(hidden)]
pub struct NoJson;
/// Builder has a body.
#[doc(hidden)]
pub struct HasJson<T: Serialize>(T);

/// Builder has no json.
#[doc(hidden)]
pub struct NoForm;
/// Builder has a body.
#[doc(hidden)]
pub struct HasForm<T: Serialize>(T);

impl RequestBuilder<NoBody, NoJson, NoForm> {
    pub fn new<U: Into<UriLike>>(uri: U) -> Self {
        Self {
            method: Method::GET,
            uri: uri.into(),
            headers: HeaderMap::default(),
            query: None,
            cache: None,
            identity: None,
            body: NoBody,
            json: NoJson,
            form: NoForm,
        }
    }

    pub fn body(self, body: Vec<u8>) -> RequestBuilder<HasBody, NoJson, NoForm> {
        RequestBuilder {
            method: self.method,
            uri: self.uri,
            headers: self.headers,
            query: self.query,
            cache: self.cache,
            identity: None,
            body: HasBody(body),
            json: NoJson,
            form: NoForm,
        }
    }

    pub fn json<T: Serialize>(self, json: T) -> RequestBuilder<NoBody, HasJson<T>, NoForm> {
        RequestBuilder {
            method: self.method,
            uri: self.uri,
            headers: self.headers,
            query: self.query,
            cache: self.cache,
            identity: None,
            body: NoBody,
            json: HasJson(json),
            form: NoForm,
        }
    }

    pub fn form<T: Serialize>(self, form: T) -> RequestBuilder<NoBody, NoJson, HasForm<T>> {
        RequestBuilder {
            method: self.method,
            uri: self.uri,
            headers: self.headers,
            query: self.query,
            cache: self.cache,
            identity: None,
            body: NoBody,
            json: NoJson,
            form: HasForm(form),
        }
    }
}

impl<B, J, F> RequestBuilder<B, J, F> {
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    #[must_use]
    pub fn header(mut self, name: impl Into<HeaderName>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    #[must_use]
    pub fn headers(mut self, headers: &HeaderMap) -> Self {
        self.headers = headers
            .iter()
            .map(|(k, v)| (k.clone(), v.to_str().unwrap_or_default().to_string()))
            .collect();
        self
    }

    pub fn query(&mut self, query: impl Into<String>) -> &mut Self {
        self.query = Some(query.into());
        self
    }

    #[must_use]
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.headers.insert(AUTHORIZATION, format!("Bearer {token}"));
        self
    }

    #[must_use]
    pub fn cache(mut self, cache: impl Into<String>) -> Self {
        self.cache = Some(cache.into());
        self
    }

    /// Sets the identity to be used for client certificate authentication.
    #[must_use]
    pub fn identity(mut self, identity: impl Into<String>) -> Self {
        self.identity = Some(identity.into());
        self
    }
}

impl RequestBuilder<NoBody, NoJson, NoForm> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub async fn send(&self) -> Result<http::Response<Bytes>> {
        self.handle(None).await
    }
}

impl RequestBuilder<HasBody, NoJson, NoForm> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub async fn send(&self) -> Result<http::Response<Bytes>> {
        self.handle(Some(self.body.0.clone())).await
    }
}

impl<B: Serialize + Safe> RequestBuilder<NoBody, HasJson<B>, NoForm> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub async fn send(&mut self) -> Result<Response<Bytes>> {
        self.headers.insert(CONTENT_TYPE, "application/json".into());
        let body =
            serde_json::to_vec(&self.json.0).map_err(|e| anyhow!("issue serializing json: {e}"))?;
        self.handle(Some(body)).await
    }
}

impl<B: Serialize + Safe> RequestBuilder<NoBody, NoJson, HasForm<B>> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub async fn send(&mut self) -> Result<http::Response<Bytes>> {
        self.headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".into());
        let body = credibil_encoding::form_encode(&self.form.0)
            .map_err(|e| anyhow!("issue serializing form: {e}"))?;
        let bytes =
            serde_json::to_vec(&body).map_err(|e| anyhow!("issue serializing form: {e}"))?;
        self.handle(Some(bytes)).await
    }
}

impl<B: Safe, J: Safe, F: Safe> RequestBuilder<B, J, F> {
    async fn handle(&self, body: Option<Vec<u8>>) -> Result<http::Response<Bytes>> {
        let uri = self.uri.to_uri()?;
        let mut builder = http::Request::builder().method(Method::GET).uri(uri);

        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        if let Some(body) = body {
            let http_req = builder.body(Full::new(Bytes::from(body)))?;
            outgoing::handle(http_req).await
        } else {
            let http_req = builder.body(Empty::<Bytes>::new())?;
            outgoing::handle(http_req).await
        }
    }
}

impl From<http::Request<Bytes>> for RequestBuilder<HasBody, NoJson, NoForm> {
    fn from(req: http::Request<Bytes>) -> Self {
        let (parts, body) = req.into_parts();

        let mut builder = RequestBuilder::new(parts.uri);
        builder = builder.method(parts.method);
        for (key, value) in &parts.headers {
            builder = builder.header(key.clone(), value.to_str().unwrap_or_default().to_string());
        }

        Self {
            method: builder.method,
            uri: builder.uri,
            headers: builder.headers,
            query: builder.query,
            cache: builder.cache,
            identity: builder.identity,
            body: HasBody(body.to_vec()),
            json: NoJson,
            form: NoForm,
        }
    }
}
