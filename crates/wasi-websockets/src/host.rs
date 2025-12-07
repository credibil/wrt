//! # WASI Websockets Service
//!
//! This module implements a runtime server for websockets

pub mod default_impl;
mod resource;
mod server;
mod store_impl;
mod types;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use super::resource::WebSocketProxy;

    wasmtime::component::bindgen!({
        world: "websockets",
        path: "wit",
        imports: {
            default: async | store | tracing,
        },
        with: {
            "wasi:websockets/store.server": WebSocketProxy,
        },
    });
}

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use kernel::{Host, Server, State};
use resource::WebSocketServer;
use server::run_server;
use store_impl::FutureResult;
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

pub use self::default_impl::WebSocketsCtxImpl;
use self::generated::wasi::websockets::store;

#[derive(Clone, Debug)]
pub struct WasiWebSockets;

impl<T> Host<T> for WasiWebSockets
where
    T: WebSocketsView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> Result<()> {
        store::add_to_linker::<_, Self>(linker, T::websockets)
    }
}

impl HasData for WasiWebSockets {
    type Data<'a> = WasiWebSocketsCtxView<'a>;
}

impl<S> Server<S> for WasiWebSockets
where
    S: State,
    S::StoreCtx: WebSocketsView,
{
    /// Provide http proxy service the specified wasm component.
    /// ``state`` will be used at a later time to provide resource access to guest handlers
    async fn run(&self, state: &S) -> Result<()> {
        run_server(state).await
    }
}

pub trait WebSocketsView: Send {
    fn websockets(&mut self) -> WasiWebSocketsCtxView<'_>;
}

/// View into [`WebSocketsCtxView`] and [`ResourceTable`].
pub struct WasiWebSocketsCtxView<'a> {
    /// Mutable reference to the WASI WebSockets context.
    pub ctx: &'a dyn WebSocketsCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

pub trait WebSocketsCtx: Debug + Send + Sync + 'static {
    fn serve(&self) -> FutureResult<Arc<dyn WebSocketServer>>;
}

#[macro_export]
macro_rules! wasi_view {
    ($store_ctx:ty, $field_name:ident) => {
        impl wasi_websockets::WebSocketsView for $store_ctx {
            fn websockets(&mut self) -> wasi_websockets::WasiWebSocketsCtxView<'_> {
                wasi_websockets::WasiWebSocketsCtxView {
                    ctx: &self.$field_name,
                    table: &mut self.table,
                }
            }
        }
    };
}
