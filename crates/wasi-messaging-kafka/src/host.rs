mod producer_impl;
mod resource;
mod server;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use rdkafka::message::OwnedMessage;
    pub use wasi::messaging::types::Error;

    pub use crate::host::resource::KafkaProducer;

    wasmtime::component::bindgen!({
        world: "messaging",
        path: "../wasi-messaging/wit",
        imports: {
            default: async | store | tracing | trappable,
        },
        exports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:messaging/types/client": KafkaProducer,
            "wasi:messaging/types/message": OwnedMessage,
        },
        trappable_error_type: {
            "wasi:messaging/types/error" => Error,
        },
    });
}

use std::fmt::Debug;

pub use resource::*;
use runtime::{Host, Server, State};
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::{ResourceTable, ResourceTableError};

pub use self::generated::Messaging;
pub use self::generated::wasi::messaging::types::Error;
use self::generated::wasi::messaging::{producer, types};

pub type Result<T, E = Error> = anyhow::Result<T, E>;

impl<T> Host<T> for WasiMessaging
where
    T: WasiMessagingView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        producer::add_to_linker::<_, Self>(linker, T::messaging)?;
        types::add_to_linker::<_, Self>(linker, T::messaging)
    }
}

impl<S> Server<S> for WasiMessaging
where
    S: State,
    <S as State>::StoreData: WasiMessagingView,
{
    async fn run(&self, state: &S) -> anyhow::Result<()> {
        server::run(state).await
    }
}

#[derive(Debug)]
pub struct WasiMessaging;
impl HasData for WasiMessaging {
    type Data<'a> = WasiMessagingCtxView<'a>;
}

/// A trait which provides internal WASI Key-Value context.
///
/// This is implemented by the resource-specific provider of Key-Value
/// functionality. For example, an in-memory store, or a Redis-backed store.
#[allow(unused)]
pub trait WasiMessagingCtx: Debug + Send + Sync + 'static {
    fn connect(&self) -> KafkaClient;
}

/// View into [`WasiMessagingCtx`] implementation and [`ResourceTable`].
pub struct WasiMessagingCtxView<'a> {
    /// Mutable reference to the WASI Key-Value context.
    pub ctx: &'a mut dyn WasiMessagingCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI Key-Value state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiMessagingView: Send {
    /// Return a [`WasiMessagingCtxView`] from mutable reference to self.
    fn messaging(&mut self) -> WasiMessagingCtxView<'_>;
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
