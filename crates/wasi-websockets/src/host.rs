//! # WASI Websockets Service
//!
//! This module implements a runtime server for websockets
#![allow(missing_docs)]
mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]
    //pub use anyhow::Error;

    wasmtime::component::bindgen!({
        world: "websockets",
        path: "wit",
        imports: {
            default: async | tracing,
        },
    });
}

use futures_channel::mpsc::{UnboundedSender, unbounded};
use futures_util::{SinkExt, StreamExt, future, pin_mut, stream::TryStreamExt};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, OnceCell};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, accept_async, connect_async};

use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use runtime::RunState;
use wasmtime::component::{HasData, InstancePre, Linker};
use wasmtime_wasi::ResourceTable;

const DEF_HTTP_ADDR: &str = "0.0.0.0:80";

use self::generated::wasi::websockets::{handler, types::Error};

pub struct WebSocketMessage {
    pub content: String,
}

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
    async fn send(&mut self, message: String) -> Result<(), Error> {
        tracing::info!("WebSocket write: {}", message);
        let ws_client = SERVICE_CLIENT
            .get_or_init(|| async {
                let addr = env::var("WEBSOCKETS_ADDR").unwrap_or_else(|_| DEF_HTTP_ADDR.into());
                connect_service_client(format!("ws://{addr}").as_str()).await.unwrap()
            })
            .await;
        let _ = ws_client.lock().await.send(Message::Text(Utf8Bytes::from(message))).await;
        Ok(())
    }

    async fn health_check(&mut self) -> Result<String, Error> {
        Ok("websockets service is healthy".into())
    }
}

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<StdMutex<HashMap<SocketAddr, Tx>>>;
static SERVICE_CLIENT: tokio::sync::OnceCell<
    tokio::sync::Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
> = OnceCell::const_new();
static PEER_MAP: OnceCell<PeerMap> = OnceCell::const_new();

#[allow(clippy::significant_drop_tightening)]
async fn accept_connection(peer_map: PeerMap, peer: SocketAddr, stream: TcpStream) {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(peer, tx);

    let is_service_peer = peer.ip().is_loopback();

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        if is_service_peer {
            tracing::info!("Received a message from {}: {}", peer, msg.to_text().unwrap());
            let peers = peer_map.lock().unwrap();

            // We want to broadcast the message to everyone except ourselves.
            let broadcast_recipients = peers
                .iter()
                .filter(|(peer_addr, _)| peer_addr != &&peer)
                .map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                recp.unbounded_send(msg.clone()).unwrap();
            }
        } else {
            tracing::info!("Ignoring message from non-service peer {}", peer);
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

        let addr = env::var("WEBSOCKETS_ADDR").unwrap_or_else(|_| DEF_HTTP_ADDR.into());
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

async fn connect_service_client(
    addr: &str,
) -> Result<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>> {
    let (client, _) = connect_async(addr).await?;
    Ok(tokio::sync::Mutex::new(client))
}
