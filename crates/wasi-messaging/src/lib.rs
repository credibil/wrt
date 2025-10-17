//! # WASI Messaging NATS
//!
//! This module implements a runtime service for `wasi:messaging`
//! (<https://github.com/WebAssembly/wasi-messaging>).

pub mod client;
mod host;
mod server;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    // pub use super::ClientName;
    pub use wasi::messaging::types::Error;

    pub use super::client::Message;
    pub use super::{ClientProxy, RequestOptions};

    wasmtime::component::bindgen!({
        world: "messaging",
        path: "wit",
        imports: {
            default: async | tracing | trappable,
        },
        exports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:messaging/request-reply/request-options": RequestOptions,
            "wasi:messaging/types/client": ClientProxy,
            "wasi:messaging/types/message": Message,
        },
        trappable_error_type: {
            "wasi:messaging/types/error" => Error,
        },
    });
}

use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{Result, anyhow};
use futures::future::{BoxFuture, FutureExt};
use runtime::{AddResource, RunState, Service};
use wasmtime::component::{InstancePre, Linker};

pub use self::generated::Messaging as Messaging2;
pub use self::generated::wasi::messaging::types::Error;
use self::generated::wasi::messaging::{producer, request_reply, types};
use crate::host::{Host, HostData};

#[derive(Debug, Default)]
pub struct Messaging {
    // A registry of messaging clients
    // clients: HashMap<String, Box<dyn Client>>,
}

impl Messaging {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // #[must_use]
    // fn client(mut self, client: impl Client + 'static) -> Self {
    //     self.clients.insert(client.name().into(), Box::new(client));
    //     self
    // }
}

impl Service for Messaging {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> anyhow::Result<()> {
        producer::add_to_linker::<_, HostData>(linker, Host::new)?;
        request_reply::add_to_linker::<_, HostData>(linker, Host::new)?;
        types::add_to_linker::<_, HostData>(linker, Host::new)?;
        Ok(())
    }

    fn start(&self, pre: InstancePre<RunState>) -> BoxFuture<'static, Result<()>> {
        server::run(pre).boxed()
    }
}

impl AddResource<async_nats::Client> for Messaging {
    fn resource(self, resource: async_nats::Client) -> anyhow::Result<Self> {
        Ok(self)
    }
}

// AddServer
use std::sync::Arc;

use futures::lock::Mutex;

use crate::client::Client;

static CLIENTS: LazyLock<Mutex<HashMap<String, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn client(client: impl Client + 'static) -> Result<()> {
    let name = client.name();
    let client = Arc::new(client);

    CLIENTS.lock().await.insert(name.to_string(), client);
    Ok(())
}

pub type ClientName = String;

#[derive(Clone, Debug)]
pub struct ClientProxy(Arc<dyn Client>);

impl ClientProxy {
    pub async fn try_from(name: String) -> Result<Self> {
        let clients = CLIENTS.lock().await;
        let Some(client) = clients.get(&name) else {
            return Err(anyhow!("messaging client '{name}' not configured in host"))?;
        };
        Ok(Self(Arc::clone(client)))
    }
}

impl Client for ClientProxy {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn subscribe(
        &self, topics: Vec<String>, instance_pre: InstancePre<RunState>,
    ) -> BoxFuture<'static, Result<()>> {
        self.0.subscribe(topics, instance_pre)
    }

    fn send(
        &self, topic: String, message: crate::client::Message,
    ) -> BoxFuture<'static, Result<()>> {
        self.0.send(topic, message)
    }

    fn request(
        &self, topic: String, message: crate::client::Message,
    ) -> BoxFuture<'static, Result<crate::client::Message>> {
        self.0.request(topic, message)
    }
}

#[derive(Default)]
pub struct RequestOptions {
    pub timeout: Option<std::time::Duration>,
}
