mod request;

use anyhow::Result;
use bytes::Bytes;
use http::{Method, Response};
use serde::de::DeserializeOwned;

use crate::guest::client::request::{NoBody, NoForm, NoJson, RequestBuilder};
use crate::guest::uri::UriLike;

pub trait Safe: Send + Sync {}
impl<T: Send + Sync> Safe for T {}

#[derive(Default)]
pub struct Client;

impl Client {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub fn get<U: Into<UriLike>>(&self, uri: U) -> RequestBuilder<NoBody, NoJson, NoForm> {
        RequestBuilder::new(uri)
    }

    pub fn post<U: Into<UriLike>>(&self, uri: U) -> RequestBuilder<NoBody, NoJson, NoForm> {
        RequestBuilder::new(uri).method(Method::POST)
    }
}

pub trait IntoJson {
    /// Decode the response body into JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the response body is not valid JSON.
    fn json<T: DeserializeOwned>(self) -> Result<T>;
}

impl IntoJson for Response<Bytes> {
    fn json<T: DeserializeOwned>(self) -> Result<T> {
        let body = self.into_body();
        let data = serde_json::from_slice::<T>(&body)?;
        Ok(data)
    }
}
