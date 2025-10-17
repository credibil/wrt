use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use anyhow::Result;
use futures::future::BoxFuture;
use runtime::RunState;
use serde::{Deserialize, Serialize};
use wasmtime::component::InstancePre;

pub trait Client: Debug + Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn subscribe(
        &self, topics: Vec<String>, instance_pre: InstancePre<RunState>,
    ) -> BoxFuture<'static, Result<()>>;

    fn send(&self, topic: String, message: Message) -> BoxFuture<'static, Result<()>>;

    fn request(&self, topic: String, message: Message) -> BoxFuture<'static, Result<Message>>;
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

    /// Optional timeout for request-response pattern.
    pub timeout: Option<Option<Duration>>,
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
    pub const fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = Some(timeout);
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

    // pub fn insert(&mut self, key: String, value: String) {
    //     self.inner.entry(key).or_default().push(value);
    // }

    // pub fn get(&self, key: &str) -> Option<&Vec<String>> {
    //     self.inner.get(key)
    // }
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Reply {
    pub client_name: String,
    pub topic: String,
}

pub struct Request {
    pub payload: Option<Vec<u8>>,
    pub metadata: Option<Metadata>,
    pub timeout: Option<Option<Duration>>,
    pub inbox: Option<String>,
}
