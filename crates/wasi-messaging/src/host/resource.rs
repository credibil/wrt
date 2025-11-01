use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use futures::Stream;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::host::generated::wasi::messaging::types;

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;
pub type Subscriptions = Pin<Box<dyn Stream<Item = Message> + Send>>;

#[allow(unused_variables)]
pub trait Client: Debug + Send + Sync + 'static {
    fn subscribe(&self, topics: Vec<String>) -> FutureResult<Subscriptions>;

    fn pre_send(&self, message: &Message) -> FutureResult<()> {
        Box::pin(async move { Ok(()) })
    }

    fn send(&self, topic: String, message: Message) -> FutureResult<()>;

    fn post_send(&self, message: &Message) -> FutureResult<()> {
        Box::pin(async move { Ok(()) })
    }

    fn request(
        &self, topic: String, message: Message, options: Option<RequestOptions>,
    ) -> FutureResult<Message>;
}

#[derive(Clone, Debug)]
pub struct ClientProxy(pub Arc<dyn Client>);

impl Deref for ClientProxy {
    type Target = Arc<dyn Client>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Message {
    pub topic: String,

    /// Payload of the message. Can be any arbitrary data format.
    pub payload: Vec<u8>,

    /// Optional metadata.
    pub metadata: Option<Metadata>,

    /// Optional  description.
    pub description: Option<String>,

    pub length: usize,

    /// Optional reply topic to which response can be published.
    pub reply: Option<Reply>,
}

impl Message {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn payload(mut self, payload: Vec<u8>) -> Self {
        self.length = payload.len();
        self.payload = payload;
        self
    }

    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let metadata = self.metadata.get_or_insert_with(Metadata::new);
        metadata.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    #[must_use]
    pub fn reply(mut self, reply: Reply) -> Self {
        self.reply = Some(reply);
        self
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metadata {
    pub inner: HashMap<String, String>,
}

impl Metadata {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl Deref for Metadata {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Metadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<Metadata> for types::Metadata {
    fn from(meta: Metadata) -> Self {
        let mut metadata = Self::new();
        for (k, v) in meta.inner {
            metadata.push((k, v));
        }
        metadata
    }
}

impl From<types::Metadata> for Metadata {
    fn from(meta: types::Metadata) -> Self {
        let mut map = HashMap::new();
        for (k, v) in meta {
            map.insert(k, v);
        }
        Self { inner: map }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Reply {
    pub client_name: String,
    pub topic: String,
}

#[derive(Default, Clone)]
pub struct RequestOptions {
    pub timeout: Option<std::time::Duration>,
    pub expected_replies: Option<u32>,
}
