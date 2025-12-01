mod producer_impl;
mod request_reply_impl;
mod resource;
mod server;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use wasi::messaging::types::Error;

    pub use crate::host::resource::{ClientProxy, MessageProxy, RequestOptions};

    wasmtime::component::bindgen!({
        world: "messaging",
        path: "wit",
        imports: {
            default: async | store | tracing | trappable,
        },
        exports: {
            default: async | store | tracing | trappable,
        },
        with: {
            "wasi:messaging/request-reply.request-options": RequestOptions,
            "wasi:messaging/types.client": ClientProxy,
            "wasi:messaging/types.message": MessageProxy,
        },
        trappable_error_type: {
            "wasi:messaging/types.error" => Error,
        },
    });
}

use std::fmt::Debug;
use std::sync::Arc;

pub use resource::*;
use runtime::{Host, Server, State};
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::{ResourceTable, ResourceTableError};

pub use self::generated::Messaging;
pub use self::generated::wasi::messaging::types::Error;
use self::generated::wasi::messaging::{producer, request_reply, types};

pub type Result<T, E = Error> = anyhow::Result<T, E>;

impl<T> Host<T> for WasiMessaging
where
    T: WasiMessagingView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        producer::add_to_linker::<_, Self>(linker, T::messaging)?;
        request_reply::add_to_linker::<_, Self>(linker, T::messaging)?;
        types::add_to_linker::<_, Self>(linker, T::messaging)
    }
}

impl<S> Server<S> for WasiMessaging
where
    S: State,
    <S as State>::StoreCtx: WasiMessagingView,
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

/// A trait which provides internal WASI Messaging context.
///
/// This is implemented by the resource-specific provider of messaging
/// functionality. For example, a NATS, or a Kafka broker.
#[allow(unused)]
pub trait WasiMessagingCtx: Debug + Send + Sync + 'static {
    /// Connect to the messaging system and return a client proxy.
    fn connect(&self) -> FutureResult<Arc<dyn Client>>;

    /// Create a new message with the given payload.
    fn new_message(&self, data: Vec<u8>) -> FutureResult<Arc<dyn Message>>;

    /// Set the content-type on a message.
    fn set_content_type(
        &self, message: Arc<dyn Message>, content_type: String,
    ) -> FutureResult<Arc<dyn Message>>;

    /// Set the payload on a message.
    fn set_payload(
        &self, message: Arc<dyn Message>, data: Vec<u8>,
    ) -> FutureResult<Arc<dyn Message>>;

    /// Append a key-value pair to the metadata of a message.
    fn add_metadata(
        &self, message: Arc<dyn Message>, key: String, value: String,
    ) -> FutureResult<Arc<dyn Message>>;

    /// Set all the metadata on a message.
    fn set_metadata(
        &self, message: Arc<dyn Message>, metadata: Metadata,
    ) -> FutureResult<Arc<dyn Message>>;

    /// Remove a key-value pair from the metadata of a message.
    fn remove_metadata(
        &self, message: Arc<dyn Message>, key: String,
    ) -> FutureResult<Arc<dyn Message>>;
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
