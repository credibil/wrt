mod impls;
mod resource;
mod server;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use wasi::messaging::types::Error;

    pub use crate::host::resource::{ClientProxy, Message, RequestOptions};

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

// re-exports
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use futures::future::{BoxFuture, FutureExt};
use futures::lock::Mutex;
pub use resource::*;
use runtime::{RunState, Service};
use wasmtime::component::{HasData, InstancePre, Linker};
use wasmtime_wasi::{ResourceTable, ResourceTableError};

pub use self::generated::Messaging;
use self::generated::wasi::messaging;
pub use self::generated::wasi::messaging::types::Error;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct WasiMessaging;

impl WasiMessaging {
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
        messaging::producer::add_to_linker::<_, HostData>(linker, Host::new)?;
        messaging::request_reply::add_to_linker::<_, HostData>(linker, Host::new)?;
        messaging::types::add_to_linker::<_, HostData>(linker, Host::new)?;
        Ok(())
    }

    fn start(&self, pre: InstancePre<RunState>) -> BoxFuture<'static, anyhow::Result<()>> {
        server::run(pre).boxed()
    }
}

pub struct Host<'a> {
    table: &'a mut ResourceTable,
}

impl Host<'_> {
    pub const fn new(c: &mut RunState) -> Host<'_> {
        Host { table: &mut c.table }
    }
}

pub struct HostData;
impl HasData for HostData {
    type Data<'a> = Host<'a>;
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}
