use std::any::Any;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http::uri::Authority;
use http::{HeaderMap, HeaderName, HeaderValue, Response, request};
use http_body_util::BodyExt;
use http_body_util::combinators::BoxBody;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use wasip3::http::handler;
use wasip3::http::types::Method;
use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

// use wasi::http::outgoing_handler;
// use wasi::http::types::{
//     FutureIncomingResponse, Headers, Method, OutgoingBody, OutgoingRequest, Scheme,
// };

// use wasmtime_wasi_http::types::{
//     FutureIncomingResponse, Headers, Method, OutgoingBody, OutgoingRequest, Scheme,
// };
// use wasmtime_wasi_http::types::OutgoingRequestConfig;
use crate::guest::cache::{CACHE_BUCKET, Cache};
// use crate::guest::handler;
use crate::guest::uri::UriLike;

/// Send an HTTP request using the WASI HTTP proxy handler.
///
/// # Errors
///
/// Returns an error if the request could not be sent.
pub async fn handle<T>(request: http::Request<T>) -> Result<http::Response<String>>
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let wasi_req =
        http_into_wasi_request(request).map_err(|e| anyhow!("Issue converting request: {e}"))?;

    println!("wasi request: {:?}", wasi_req.get_headers());

    let wasi_resp =
        handler::handle(wasi_req).await.map_err(|e| anyhow!("Issue calling proxy: {e}"))?;

    println!("wasi response: {:?}", wasi_resp);

    let response = http_from_wasi_response(wasi_resp)
        .map_err(|e| anyhow!("Issue converting response: {e}"))?;

    println!("response: {:?}", response.headers());
    let body = response.into_body();
    let bytes = body.collect().await?;
    println!("body bytes: {}", bytes.to_bytes().len());

    Ok(http::Response::new("Hello, WASI!".to_string()))
}

pub struct OutgoingBody(BoxBody<Bytes, anyhow::Error>);

#[derive(Default)]
pub struct Client {
    /// The cache bucket to use for caching responses.
    cache: Option<String>,
}

impl Client {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cache to use when caching responses.
    #[must_use]
    pub fn cache(mut self, cache: impl Into<String>) -> Self {
        self.cache = Some(cache.into());
        self
    }

    pub fn get<U: Into<UriLike>>(&self, uri: U) -> RequestBuilder<NoBody, NoJson, NoForm> {
        let Some(cache) = &self.cache else {
            return RequestBuilder::new(uri);
        };
        RequestBuilder::new(uri).cache(cache.clone())
    }

    pub fn post<U: Into<UriLike>>(&self, uri: U) -> RequestBuilder<NoBody, NoJson, NoForm> {
        let Some(cache) = &self.cache else {
            return RequestBuilder::new(uri).method(Method::Post);
        };
        RequestBuilder::new(uri).method(Method::Post).cache(cache.clone())
    }
}

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
    fn new<U: Into<UriLike>>(uri: U) -> Self {
        Self {
            method: Method::Get,
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
    pub fn send(&self) -> Result<Response<Bytes>> {
        self.send_bytes(None)
    }
}

impl RequestBuilder<HasBody, NoJson, NoForm> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub fn send(&self) -> Result<Response<Bytes>> {
        self.send_bytes(Some(&self.body.0))
    }
}

impl<B: Serialize> RequestBuilder<NoBody, HasJson<B>, NoForm> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub fn send(&mut self) -> Result<Response<Bytes>> {
        self.headers.insert(CONTENT_TYPE, "application/json".into());
        let body =
            serde_json::to_vec(&self.json.0).map_err(|e| anyhow!("issue serializing json: {e}"))?;
        self.send_bytes(Some(&body))
    }
}

impl<B: Serialize> RequestBuilder<NoBody, NoJson, HasForm<B>> {
    /// Send the request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails to send.
    pub fn send(&mut self) -> Result<Response<Bytes>> {
        self.headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".into());
        let body = credibil_encoding::form_encode(&self.form.0)
            .map_err(|e| anyhow!("issue serializing form: {e}"))?;
        let bytes =
            serde_json::to_vec(&body).map_err(|e| anyhow!("issue serializing form: {e}"))?;
        self.send_bytes(Some(&bytes))
    }
}

impl<B, J, F> RequestBuilder<B, J, F> {
    fn send_bytes(&self, body: Option<&[u8]>) -> Result<Response<Bytes>> {
        todo!()
        // let request = self.prepare_request(body)?;

        // tracing::trace!(
        //     "sending request: {:?}://{}{}",
        //     request.scheme().unwrap_or(Scheme::Http),
        //     request.authority().unwrap_or_default(),
        //     request.path_with_query().unwrap_or_default()
        // );
        // for (name, value) in request.headers().entries().as_slice() {
        //     tracing::trace!("request header: {name}: {:?}", String::from_utf8_lossy(value));
        // }

        // // caching
        // let bucket = self.cache.as_deref().unwrap_or(CACHE_BUCKET);
        // let mut cache = Cache::new(bucket);

        // match cache.headers(&request.headers()) {
        //     Ok(()) => Ok(()),
        //     Err(e) => {
        //         let err = format!("issue setting cache headers: {e}");
        //         tracing::error!(err);
        //         Err(anyhow!(err))
        //     }
        // }?;

        // let response = if cache.should_use_cache() {
        //     tracing::debug!("cache-first enabled, checking cache");

        //     let fut_resp = match cache.get() {
        //         Ok(Some(resp)) => {
        //             tracing::debug!("response found in cache");
        //             return Ok(resp);
        //         }
        //         Ok(None) => {
        //             tracing::debug!("no cached response found, fetching from origin");
        //             outgoing_handler::handle(request, None)
        //                 .map_err(|e| anyhow!("making request: {e}"))?

        //             // handler::handle(request).await.context("making request")?
        //         }
        //         Err(e) => {
        //             tracing::error!("retrieving cached response: {e}, fetching from origin");
        //             outgoing_handler::handle(request, None)
        //                 .map_err(|e| anyhow!("making request: {e}"))?

        //             // handler::handle(request).await.context("making request")?
        //         }
        //     };

        //     Self::process_response(&fut_resp)
        // } else {
        //     tracing::debug!("resource-first enabled, fetching from origin");

        //     let fut_resp = outgoing_handler::handle(request, None)
        //         .map_err(|e| anyhow!("making request: {e}"))?;
        //     Self::process_response(&fut_resp)
        // }?;

        // // TODO: spawn task for storing cache and return response immediately
        // if cache.should_store() {
        //     tracing::debug!("storing response in cache");
        //     if let Err(e) = cache.put(&response) {
        //         tracing::error!("storing response in cache failed: {e}");
        //     }
        // }
        // Ok(response)
    }

    // fn prepare_request(&self, body: Option<&[u8]>) -> Result<OutgoingRequest> {
    //     // headers
    //     let headers = Headers::new();
    //     for (key, value) in &self.headers {
    //         headers
    //             .append(key.as_str(), value.as_bytes())
    //             .map_err(|e| anyhow!("setting header: {e}"))?;
    //     }
    //     let request = OutgoingRequest::new(headers);

    //     // method
    //     request.set_method(&self.method).map_err(|()| anyhow!("setting method"))?;

    //     // url
    //     let uri = self.uri.into_uri()?;

    //     // scheme
    //     let Some(scheme) = uri.scheme() else {
    //         return Err(anyhow!("missing scheme"));
    //     };
    //     let scheme = match scheme.as_str() {
    //         "http" => Scheme::Http,
    //         "https" => Scheme::Https,
    //         _ => return Err(anyhow!("unsupported scheme: {}", scheme.as_str())),
    //     };
    //     request.set_scheme(Some(&scheme)).map_err(|()| anyhow!("setting scheme"))?;

    //     // authority
    //     request
    //         .set_authority(uri.authority().map(Authority::as_str))
    //         .map_err(|()| anyhow!("setting authority"))?;

    //     // path + query
    //     let mut path_with_query = uri.path().to_string();
    //     if let Some(query) = uri.query() {
    //         path_with_query = format!("{path_with_query}?{query}");
    //     }
    //     request
    //         .set_path_with_query(Some(&path_with_query))
    //         .map_err(|()| anyhow!("setting path_with_query"))?;

    //     tracing::trace!("encoded path_with_query: {path_with_query}");

    //     // body
    //     let out_body = request.body().map_err(|()| anyhow!("getting outgoing body"))?;
    //     if let Some(mut buf) = body {
    //         let out_stream = out_body.write().map_err(|()| anyhow!("getting output stream"))?;

    //         let pollable = out_stream.subscribe();
    //         while !buf.is_empty() {
    //             pollable.block();
    //             let Ok(permit) = out_stream.check_write() else {
    //                 return Err(anyhow!("output stream is not writable"));
    //             };

    //             #[expect(clippy::cast_possible_truncation)]
    //             let len = buf.len().min(permit as usize);

    //             let (chunk, rest) = buf.split_at(len);
    //             if out_stream.write(chunk).is_err() {
    //                 return Err(anyhow!("writing to output stream"));
    //             }
    //             buf = rest;
    //         }

    //         if out_stream.flush().is_err() {
    //             return Err(anyhow!("flushing output stream"));
    //         }

    //         pollable.block();
    //         if out_stream.check_write().is_err() {
    //             return Err(anyhow!("output stream error"));
    //         }
    //     }

    //     OutgoingBody::finish(out_body, None)?;
    //     Ok(request)
    // }

    // fn process_response(fut_resp: &FutureIncomingResponse) -> Result<Response<Bytes>> {
    //     fut_resp.subscribe().block();
    //     let Some(result) = fut_resp.get() else {
    //         return Err(anyhow!("missing response"));
    //     };
    //     let response = result
    //         .map_err(|()| anyhow!("issue getting response"))?
    //         .map_err(|e| anyhow!("response error: {e}"))?;

    //     // process body
    //     let body = response.consume().map_err(|()| anyhow!("issue getting body"))?;
    //     let stream = body.stream().map_err(|()| anyhow!("issue getting body's stream"))?;

    //     let mut body = vec![];
    //     while let Ok(chunk) = stream.blocking_read(1024 * 1024) {
    //         body.extend_from_slice(&chunk);
    //     }

    //     // transform unsuccessful requests into an error
    //     let status = response.status();
    //     if !(200..300).contains(&status) {
    //         if body.is_empty() {
    //             return Err(anyhow!("request unsuccessful {status}"));
    //         }

    //         // extract error description from body
    //         let msg = if let Ok(msg) = serde_json::from_slice::<Value>(&body) {
    //             serde_json::to_string(&msg)?
    //         } else {
    //             String::from_utf8_lossy(&body).to_string()
    //         };
    //         return Err(anyhow!("request unsuccessful {status}, {msg}"));
    //     }

    //     // convert response
    //     let mut resp = Response::new(Bytes::from(body));
    //     for (name, value) in response.headers().entries() {
    //         let name = HeaderName::from_str(&name)
    //             .with_context(|| format!("Failed to parse header: {name}"))?;
    //         let value = HeaderValue::from_bytes(&value)
    //             .with_context(|| format!("Failed to parse header value for {name}"))?;
    //         resp.headers_mut().insert(name, value);
    //     }

    //     drop(stream);
    //     drop(response);

    //     Ok(resp)
    // }
}

// pub trait Decode {
//     /// Decode the response body as JSON.
//     ///
//     /// # Errors
//     ///
//     /// Returns an error if the response body is not valid JSON.
//     fn json<T: DeserializeOwned>(self) -> Result<T>;
// }

// impl Decode for Response<Bytes> {
//     fn json<T: DeserializeOwned>(self) -> Result<T> {
//         let body = self.into_body();
//         let data = serde_json::from_slice::<T>(&body)?;
//         Ok(data)
//     }
// }
