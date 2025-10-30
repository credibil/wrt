//! # WASI Websockets Service
//!
//! This module implements a runtime server for websockets
#![allow(missing_docs)]
mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]
    wasmtime::component::bindgen!({
        world: "websockets",
        path: "wit",
        imports: {
            default: async | tracing,
        },
    });
}

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use futures_channel::mpsc::{UnboundedSender, unbounded};
use futures_util::stream::TryStreamExt;
use futures_util::{SinkExt, StreamExt, future, pin_mut};
use runtime::RunState;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, OnceCell};
use tokio_tungstenite::tungstenite::{Bytes, Message, Utf8Bytes};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, accept_async, connect_async};
use wasmtime::component::{HasData, InstancePre, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::websockets::handler;
use self::generated::wasi::websockets::types::{Error, Peer};

const DEF_WEBSOCKETS_ADDR: &str = "0.0.0.0:80";

/// Websockets service
#[derive(Debug)]
pub struct WebSockets;

impl runtime::Service for WebSockets {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> Result<()> {
        handler::add_to_linker::<_, SocketMessaging>(linker, Host::new)?;
        Ok(())
    }

    /// Provide http proxy service the specified wasm component.
    fn start(&self, _: InstancePre<RunState>) -> BoxFuture<'static, Result<()>> {
        Self::run().boxed()
    }
}

struct SocketMessaging;
impl HasData for SocketMessaging {
    type Data<'a> = Host<'a>;
}

/// Host for WASI websockets
pub struct Host<'a> {
    _table: &'a mut ResourceTable,
}

impl Host<'_> {
    const fn new(c: &mut RunState) -> Host<'_> {
        Host { _table: &mut c.table }
    }
}

impl handler::Host for Host<'_> {
    async fn get_peers(&mut self) -> Result<Vec<Peer>, Error> {
        let peer_map = PEER_MAP.get().ok_or_else(|| Error {
            message: "Peer map not initialized".into(),
        })?;
        let peers = peer_map
            .lock()
            .unwrap()
            .iter()
            .map(|(key, peer)| Peer {
                address: key.to_string(),
                query: peer.query.clone(),
            })
            .collect();
        Ok(peers)
    }

    async fn send_peers(&mut self, message: String, peers: Vec<String>) -> Result<(), Error> {
        tracing::info!("WebSocket write: {}", message);
        let ws_client = service_client().await;
        let msg = serde_json::to_string(&PublishMessage {
            peers: peers.join(","),
            content: message,
        })
        .unwrap();
        let _ = ws_client.lock().await.send(Message::Text(msg.into())).await;
        Ok(())
    }

    async fn send_all(&mut self, message: String) -> Result<(), Error> {
        tracing::info!("WebSocket write: {}", message);
        let ws_client = service_client().await;
        let msg = serde_json::to_string(&PublishMessage {
            peers: "all".into(),
            content: message,
        })
        .unwrap();
        let _ = ws_client.lock().await.send(Message::Text(msg.into())).await;
        Ok(())
    }

    async fn health_check(&mut self) -> Result<String, Error> {
        let ws_client = service_client().await;
        ws_client.lock().await.send(Message::Ping(Bytes::new())).await.map_err(|e| Error {
            message: format!("Websocket service is unhealthy: {e}"),
        })?;
        Ok("websockets service is healthy".into())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PublishMessage {
    pub peers: String,
    pub content: String,
}

pub struct PeerInfo {
    sender: UnboundedSender<Message>,
    query: String,
}
type PeerMap = Arc<StdMutex<HashMap<SocketAddr, PeerInfo>>>;
static SERVICE_CLIENT: tokio::sync::OnceCell<
    tokio::sync::Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
> = OnceCell::const_new();
static PEER_MAP: OnceCell<PeerMap> = OnceCell::const_new();

#[allow(clippy::significant_drop_tightening)]
async fn accept_connection(peer_map: PeerMap, peer: SocketAddr, stream: TcpStream) {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(
        peer,
        PeerInfo {
            sender: tx,
            query: String::new(),
        },
    );

    let is_service_peer = peer.ip().is_loopback();

    let (outgoing, incoming) = ws_stream.split();
    let mut close_connection = false;

    let broadcast_incoming = incoming.try_for_each(|msg| {
        if is_service_peer {
            tracing::info!(
                "Received a message from service peer {}: {}",
                peer,
                msg.to_text().unwrap()
            );
            let message: PublishMessage = match serde_json::from_str(msg.to_text().unwrap()) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Failed to parse message from service client: {}", e);
                    return future::ok(());
                }
            };
            let peers = peer_map.lock().unwrap();
            let recipents = if message.peers == "all" {
                peers.values().collect::<Vec<&PeerInfo>>()
            } else {
                let target_peers: Vec<SocketAddr> =
                    message.peers.split(',').filter_map(|s| s.parse().ok()).collect();
                let mut filtered_peers: Vec<&PeerInfo> = Vec::new();
                for addr in &target_peers {
                    if let Some(peer_info) = peers.get(addr) {
                        filtered_peers.push(peer_info);
                    }
                }
                filtered_peers
            };

            for recp in recipents {
                recp.sender
                    .unbounded_send(Message::Text(Utf8Bytes::from(message.content.clone())))
                    .unwrap();
            }
        } else if let Message::Text(text) = msg {
            // Handle client filter subscription
            let json_msg: Result<serde_json::Value, _> = serde_json::from_str(&text);
            if json_msg.is_ok() {
                tracing::info!("Setting filter for peer {}: {}", peer, text);
                if let Some(peer_info) = peer_map.lock().unwrap().get_mut(&peer) {
                    peer_info.query = text.to_string();
                }
            } else {
                tracing::error!("Expected filter json object, got unknown text instead: {text}");
                close_connection = true;
            }
        }

        if close_connection {
            return future::err(tokio_tungstenite::tungstenite::Error::ConnectionClosed);
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    tracing::info!("{} disconnected", &peer);
    peer_map.lock().unwrap().remove(&peer);
}

impl WebSockets {
    /// Provide http proxy service the specified wasm component.
    async fn run() -> Result<()> {
        let state = PeerMap::new(StdMutex::new(HashMap::new()));
        let _ = PEER_MAP.set(state);

        let addr = env::var("WEBSOCKETS_ADDR").unwrap_or_else(|_| DEF_WEBSOCKETS_ADDR.into());
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("websocket server listening on: {}", listener.local_addr()?);
        //connect_service_client(&addr).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            let peer = stream.peer_addr().expect("connected streams should have a peer address");
            tracing::info!("Peer address: {}", peer);

            tokio::spawn(accept_connection(Arc::clone(PEER_MAP.get().unwrap()), peer, stream));
        }
    }
}

async fn service_client() -> &'static Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    SERVICE_CLIENT
        .get_or_init(|| async {
            let addr = env::var("WEBSOCKETS_ADDR").unwrap_or_else(|_| DEF_WEBSOCKETS_ADDR.into());
            let (client, _) = connect_async(format!("ws://{addr}")).await.unwrap();
            tokio::sync::Mutex::new(client)
        })
        .await
}
