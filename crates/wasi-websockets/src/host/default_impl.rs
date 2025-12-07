//! Default no-op implementation for wasi-websockets
//!
//! This is a lightweight implementation for development use only.
//! It provides a basic WebSockets server implementation without persistent connections.
//! For production use, use a backend with proper WebSockets connection management.

#![allow(clippy::used_underscore_binding)]

use std::sync::Arc;

use anyhow::Result;
use futures::FutureExt;
use kernel::{Backend, FutureResult};
use tracing::instrument;

use crate::host::WebSocketsCtx;
use crate::host::resource::{DefaultWebSocketServer, WebSocketServer};

#[derive(Debug, Clone, Default)]
pub struct ConnectOptions;

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Ok(Self)
    }
}

#[derive(Debug, Clone)]
pub struct WasiWebSocketsCtxImpl;

impl Backend for WasiWebSocketsCtxImpl {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(_options: Self::ConnectOptions) -> Result<Self> {
        tracing::debug!("initializing default WebSocket implementation");
        tracing::warn!("Using default WebSocket implementation - suitable for development only");
        Ok(Self)
    }
}

impl WebSocketsCtx for WasiWebSocketsCtxImpl {
    /// Provide a default WebSockets server.
    ///
    /// This is a basic implementation for development use only.
    fn serve(&self) -> FutureResult<Arc<dyn WebSocketServer>> {
        async move {
            tracing::debug!("creating default WebSockets server");
            Ok(Arc::new(DefaultWebSocketServer) as Arc<dyn WebSocketServer>)
        }
        .boxed()
    }
}
