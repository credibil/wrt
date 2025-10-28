mod producer_impl;
mod resource;
mod server;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use rdkafka::message::OwnedMessage;
    pub use wasi::messaging::types::Error;

    pub use crate::host::resource::KafkaClient;

    wasmtime::component::bindgen!({
        world: "messaging",
        path: "../wasi-messaging/wit",
        imports: {
            default: async | tracing | trappable,
        },
        exports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:messaging/types/client": KafkaClient,
            "wasi:messaging/types/message": OwnedMessage,
        },
        trappable_error_type: {
            "wasi:messaging/types/error" => Error,
        },
    });
}

use std::sync::OnceLock;

use anyhow::anyhow;
use futures::future::{BoxFuture, FutureExt};
pub use resource::*;
use runtime::{AddResource, RunState, Service};
use wasmtime::component::{HasData, InstancePre, Linker};
use wasmtime_wasi::{ResourceTable, ResourceTableError};

pub use self::generated::Messaging;
use self::generated::wasi::messaging;
pub use self::generated::wasi::messaging::types::Error;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

static CLIENT: OnceLock<KafkaClient> = OnceLock::new();

/// Access the registered Kafka client
///
/// # Errors
/// * no Kafka client registered
pub fn kafka() -> Result<&'static KafkaClient> {
    CLIENT.get().ok_or_else(|| anyhow!("no Kafka client registered").into())
}

#[derive(Debug)]
pub struct WasiMessaging;

impl AddResource<KafkaClient> for WasiMessaging {
    async fn resource(self, resource: KafkaClient) -> anyhow::Result<Self> {
        if CLIENT.set(resource).is_err() {
            return Err(anyhow!("messaging client already registered"));
        }
        Ok(self)
    }
}

impl Service for WasiMessaging {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> anyhow::Result<()> {
        messaging::producer::add_to_linker::<_, HostData>(linker, Host::new)?;
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

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
