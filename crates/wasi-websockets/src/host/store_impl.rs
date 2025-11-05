use std::fmt::Debug;

use anyhow::anyhow;
use futures::FutureExt;
use futures::future::BoxFuture;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::{Bytes, Message};
use wasmtime::component::{Accessor, Resource};
use wasmtime_wasi::ResourceTable;

use crate::WebSocketsCtxView;
use crate::host::generated::wasi::websockets::store::{
    Host, HostServer, HostServerWithStore, HostWithStore,
};
use crate::host::generated::wasi::websockets::types::{Error, Peer};
use crate::host::resource::{PublishMessage, WebSocketProxy};
use crate::host::server::{get_peer_map, service_client};
use crate::host::{Result, WasiWebSockets};

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

impl HostWithStore for WasiWebSockets {
    async fn get_server<T>(
        accessor: &Accessor<T, Self>,
    ) -> Result<Resource<WebSocketProxy>, Error> {
        let server = accessor.with(|mut store| store.get().ctx.serve()).await?;
        let proxy = WebSocketProxy(server);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }
}

impl HostServerWithStore for WasiWebSockets {
    async fn get_peers<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>,
    ) -> Result<Vec<Peer>, Error> {
        let ws_server = use_server(accessor, &self_)?;
        Ok(ws_server.get_peers())
    }

    async fn send_peers<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>, message: String,
        peers: Vec<String>,
    ) -> Result<(), Error> {
        let ws_server = use_server(accessor, &self_)?;
        let result = ws_server.send_peers(message, peers).await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn send_all<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>, message: String,
    ) -> Result<(), Error> {
        let ws_server = use_server(accessor, &self_)?;
        let result = ws_server.send_all(message).await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn health_check<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>,
    ) -> Result<String, Error> {
        let ws_server = use_server(accessor, &self_)?;
        let result = ws_server.health_check().await;
        match result {
            Ok(status) => Ok(status),
            Err(e) => Err(e.into()),
        }
    }

    async fn drop<T>(_a: &Accessor<T, Self>, _r: Resource<WebSocketProxy>) -> Result<()>
    where
        Self: Sized,
    {
        let peers = get_peer_map().map_err(|e| Error {
            message: format!("Failed to get peer map: {e}"),
        })?;
        peers.lock().unwrap().clear();
        Ok(())
    }
}

/// View into [`WebSocketsCtxView`] and [`ResourceTable`].
pub struct WasiWebSocketsView<'a> {
    /// View to the Web sockets context.
    pub ctx: &'a dyn WebSocketsCtxView,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

impl Host for WasiWebSocketsView<'_> {}

impl HostServer for WasiWebSocketsView<'_> {}

pub fn use_server<T>(
    accessor: &Accessor<T, WasiWebSockets>, self_: &Resource<WebSocketProxy>,
) -> Result<WebSocketProxy> {
    accessor.with(|mut store| {
        let server =
            store.get().table.get(self_).map_err(|_e| anyhow!("Failed to get WebSocket server"))?;
        Ok::<_, anyhow::Error>(server.clone())
    })
}

impl From<wasmtime_wasi::ResourceTableError> for Error {
    fn from(err: wasmtime_wasi::ResourceTableError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

/// Providers implement the [`WebSocketServer`] trait to allow the host to
/// interact with backend resources.
pub trait WebSocketServer: Debug + Send + Sync + 'static {
    /// Get the peers connected to the server.
    fn get_peers(&self) -> Vec<Peer> {
        let peer_map = get_peer_map().unwrap();
        peer_map
            .lock()
            .unwrap()
            .iter()
            .map(|(key, peer)| Peer {
                address: key.to_string(),
                query: peer.query.clone(),
            })
            .collect()
    }

    /// Send a message to the specified peers.
    fn send_peers(&self, message: String, peers: Vec<String>) -> FutureResult<()> {
        tracing::info!("WebSocket write: {message} for peers: {:?}", peers);
        async move {
            let ws_client = service_client().await;
            let msg = serde_json::to_string(&PublishMessage {
                peers: peers.join(","),
                content: message,
            })
            .map_err(|e| Error {
                message: format!("Failed to serialize PublishMessage: {e}"),
            })?;
            let send_result = ws_client.lock().await.send(Message::Text(msg.into())).await;
            if let Err(e) = send_result {
                tracing::error!("Failed to send message to peers: {e}");
            }
            Ok(())
        }
        .boxed()
    }

    /// Send a message to all connected peers.
    fn send_all(&self, message: String) -> FutureResult<()> {
        tracing::info!("WebSocket write: {}", message);
        async move {
            let ws_client = service_client().await;
            let msg = serde_json::to_string(&PublishMessage {
                peers: "all".into(),
                content: message,
            })
            .unwrap();
            let _ = ws_client.lock().await.send(Message::Text(msg.into())).await;
            Ok(())
        }
        .boxed()
    }

    /// Perform a health check on the server.
    fn health_check(&self) -> FutureResult<String> {
        async move {
            let ws_client = service_client().await;
            ws_client.lock().await.send(Message::Ping(Bytes::new())).await.map_err(|e| Error {
                message: format!("Websocket service is unhealthy: {e}"),
            })?;
            Ok("websockets service is healthy".into())
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct DefaultWebSocketServer;
impl WebSocketServer for DefaultWebSocketServer {}
