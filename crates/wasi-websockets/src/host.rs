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
use std::convert::Infallible;
use std::env;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::Result;
use futures_channel::mpsc::{UnboundedSender, unbounded};
use futures_util::stream::TryStreamExt;
use futures_util::{SinkExt, StreamExt, future, pin_mut};
use hyper::body::Incoming;
use hyper::header::{
    CONNECTION, HeaderValue, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION,
    UPGRADE,
};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Method, Request, Response, StatusCode, Version};
use hyper_util::rt::TokioIo;
use runtime::{Host, Server, State};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, OnceCell};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::{Bytes, Message, Utf8Bytes};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tungstenite::protocol::Role;
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::websockets::handler;
use self::generated::wasi::websockets::types::{Error, Peer};

const DEF_WEBSOCKETS_ADDR: &str = "0.0.0.0:80";

impl<T> Host<T> for WasiWebSockets
where
    T: WebSocketsView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> Result<()> {
        handler::add_to_linker::<_, Self>(linker, T::start)
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
        run(state).await
    }
}

pub trait WebSocketsView: Send {
    fn start(&mut self) -> WasiWebSocketsView<'_>;
}

#[derive(Clone, Debug)]
pub struct WasiWebSockets;
impl HasData for WasiWebSockets {
    type Data<'a> = WasiWebSocketsView<'a>;
}

/// View into [`WasiWebSockets`] implementation and [`ResourceTable`].
pub struct WasiWebSocketsView<'a> {
    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

impl handler::Host for WasiWebSocketsView<'_> {
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
        tracing::info!("WebSocket write: {message} for peers: {:?}", peers);
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishMessage {
    pub peers: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    sender: UnboundedSender<Message>,
    query: String,
}

type PeerMap = Arc<StdMutex<HashMap<SocketAddr, PeerInfo>>>;
static PEER_MAP: OnceCell<PeerMap> = OnceCell::const_new();

static SERVICE_CLIENT: OnceCell<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>> =
    OnceCell::const_new();

/// Accept a new websocket connection
#[allow(clippy::significant_drop_tightening)]
async fn accept_connection(
    peer_map: PeerMap, peer: SocketAddr, ws_stream: WebSocketStream<TokioIo<Upgraded>>,
) {
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

type Body = http_body_util::Full<hyper::body::Bytes>;
/// Handle incoming HTTP requests and upgrade to ``WebSocket`` if appropriate
#[allow(clippy::unused_async)]
#[allow(clippy::map_unwrap_or)]
async fn handle_request(
    peer_map: PeerMap, mut req: Request<Incoming>, addr: SocketAddr,
) -> Result<Response<Body>, Infallible> {
    let upgrade = HeaderValue::from_static("Upgrade");
    let websocket = HeaderValue::from_static("websocket");
    let headers = req.headers();
    let key = headers.get(SEC_WEBSOCKET_KEY);
    let derived = key.map(|k| derive_accept_key(k.as_bytes()));
    if req.method() != Method::GET
        || req.version() < Version::HTTP_11
        || !headers
            .get(CONNECTION)
            .and_then(|h| h.to_str().ok())
            .map(|h| h.split([' ', ',']).any(|p| p.eq_ignore_ascii_case(upgrade.to_str().unwrap())))
            .unwrap_or(false)
        || !headers
            .get(UPGRADE)
            .and_then(|h| h.to_str().ok())
            .map(|h| h.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false)
        || !headers.get(SEC_WEBSOCKET_VERSION).map(|h| h == "13").unwrap_or(false)
        || key.is_none()
        || req.uri() != "/"
    {
        let mut resp =
            Response::new(Body::from("This service only supports WebSocket connections.\n"));
        *resp.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(resp);
    }
    let ver = req.version();
    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                let upgraded = TokioIo::new(upgraded);
                accept_connection(
                    peer_map,
                    addr,
                    WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await,
                )
                .await;
            }
            Err(e) => tracing::error!("upgrade error: {e}"),
        }
    });
    let mut res = Response::new(Body::default());
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = ver;
    res.headers_mut().append(CONNECTION, upgrade);
    res.headers_mut().append(UPGRADE, websocket);
    res.headers_mut().append(SEC_WEBSOCKET_ACCEPT, derived.unwrap().parse().unwrap());
    // Let's add an additional header to our response to the client.
    res.headers_mut().append("MyCustomHeader", ":)".parse().unwrap());
    res.headers_mut().append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());
    Ok(res)
}

#[allow(clippy::missing_panics_doc)]
#[allow(clippy::missing_errors_doc)]
pub async fn run<S>(_: &S) -> Result<()>
where
    S: State,
    <S as State>::StoreData: WebSocketsView,
{
    let state = PeerMap::new(StdMutex::new(HashMap::new()));
    let _ = PEER_MAP.set(Arc::<StdMutex<HashMap<SocketAddr, PeerInfo>>>::clone(&state));

    let addr = env::var("WEBSOCKETS_ADDR").unwrap_or_else(|_| DEF_WEBSOCKETS_ADDR.into());
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("websocket server listening on: {}", listener.local_addr()?);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let peer = stream.peer_addr().expect("connected streams should have a peer address");
        tracing::info!("Peer address: {}", peer);
        let state_ref = Arc::<StdMutex<HashMap<SocketAddr, PeerInfo>>>::clone(&state);

        tokio::spawn(async move {
            let io = TokioIo::new(stream);

            let service = service_fn(move |req| {
                handle_request(
                    Arc::<StdMutex<HashMap<SocketAddr, PeerInfo>>>::clone(&state_ref),
                    req,
                    peer_addr,
                )
            });

            let conn = http1::Builder::new().serve_connection(io, service).with_upgrades();

            if let Err(err) = conn.await {
                tracing::error!("failed to serve connection: {err:?}");
            }
        });
    }
}

/// Get the singleton websocket service client
async fn service_client() -> &'static Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    SERVICE_CLIENT
        .get_or_init(|| async {
            let addr = env::var("WEBSOCKETS_ADDR").unwrap_or_else(|_| DEF_WEBSOCKETS_ADDR.into());
            let (client, _) = connect_async(format!("ws://{addr}")).await.unwrap();
            tokio::sync::Mutex::new(client)
        })
        .await
}
