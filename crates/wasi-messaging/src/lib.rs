//! # WASI Messaging NATS
//!
//! This module implements a runtime service for `wasi:messaging`
//! (<https://github.com/WebAssembly/wasi-messaging>).

mod host;
pub mod resource;
mod server;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use wasi::messaging::types::Error;

    pub use crate::resource::{ClientProxy, Message, RequestOptions};

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
use std::sync::{Arc, LazyLock};

use anyhow::{Result};
use futures::future::{BoxFuture, FutureExt};
use futures::lock::Mutex;
use runtime::{ RunState, Service};
use wasmtime::component::{InstancePre, Linker};

pub use self::generated::Messaging;
pub use self::generated::wasi::messaging::types::Error;
use self::generated::wasi::messaging::{producer, request_reply, types};
use crate::host::{Host, HostData};
use crate::resource::Client;

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct WasiMessaging;

impl WasiMessaging {
    // #[must_use]
    /// Register a messaging client with the host
    ///
    /// # Errors
    ///
    /// If the client could not be registered
    pub async fn client(self, client: impl Client + 'static) -> Self {
        CLIENTS.lock().await.insert(client.name(), Arc::new(client));
        self
    }
}

impl Service for WasiMessaging {
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

// impl AddResource<async_nats::Client> for WasiMessaging {
//     fn resource(self, resource: async_nats::Client) -> anyhow::Result<Self> {
//         Ok(self)
//     }
// }
