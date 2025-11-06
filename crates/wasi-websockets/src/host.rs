//! # WASI Websockets Service
//!
//! This module implements a runtime server for websockets

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
            "wasi:websockets/store/server": WebSocketProxy,
        },
    });
}

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use futures::FutureExt;
use resource::{DefaultWebSocketServer, WebSocketServer};
use runtime::{Host, Server, State};
use server::run_server;
use store_impl::FutureResult;
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::websockets::store;
use self::generated::wasi::websockets::store::{Host as WsHost, HostServer};

const DEF_WEBSOCKETS_ADDR: &str = "0.0.0.0:80";

impl<T> Host<T> for WasiWebSockets
where
    T: WebSocketsView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> Result<()> {
        store::add_to_linker::<_, Self>(linker, T::start)
    }
}

impl<S> Server<S> for WasiWebSockets
where
    S: State,
    <S as State>::StoreData: WebSocketsView,
{
    /// Provide http proxy service the specified wasm component.
    /// ``state`` will be used at a later time to provide resource access to guest handlers
    async fn run(&self, state: &S) -> Result<()> {
        run_server(state).await
    }
}

/// View into [`WebSocketsCtxView`] and [`ResourceTable`].
pub struct WasiWebSocketsCtxView<'a> {
    /// View to the Web sockets context.
    pub ctx: &'a dyn WebSocketsCtxView,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

impl WsHost for WasiWebSocketsCtxView<'_> {}

impl HostServer for WasiWebSocketsCtxView<'_> {}

pub trait WebSocketsView: Send {
    fn start(&mut self) -> WasiWebSocketsCtxView<'_>;
}

#[derive(Clone, Debug)]
pub struct WasiWebSockets;
impl HasData for WasiWebSockets {
    type Data<'a> = WasiWebSocketsCtxView<'a>;
}

pub trait WebSocketsCtxView: Debug + Send + Sync + 'static {
    fn serve(&self) -> FutureResult<Arc<dyn WebSocketServer>> {
        async move { Ok(Arc::new(DefaultWebSocketServer) as Arc<dyn WebSocketServer>) }.boxed()
    }
}

#[derive(Clone, Debug)]
pub struct DefaultWebSocketsCtx;
impl WebSocketsCtxView for DefaultWebSocketsCtx {}
