use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use futures::FutureExt;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::{Bytes, Message};

use crate::host::generated::wasi::websockets::types::{Error, Peer};
use crate::host::server::{get_peer_map, service_client};
use crate::host::store_impl::FutureResult;
use crate::host::types::PublishMessage;

#[derive(Clone, Debug)]
pub struct WebSocketProxy(pub Arc<dyn WebSocketServer>);

impl Deref for WebSocketProxy {
    type Target = Arc<dyn WebSocketServer>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
            .filter(|(_, peer)| !peer.is_service)
            .map(|(key, peer)| Peer {
                address: key.to_string(),
                query: peer.query.clone(),
            })
            .collect()
    }

    /// Send a message to the specified peers.
    fn send_peers(&self, message: String, peers: Vec<String>) -> FutureResult<()> {
        tracing::debug!("WebSocket write: {message} for peers: {:?}", peers);
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
        tracing::debug!("WebSocket write: {}", message);
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
