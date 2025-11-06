use std::net::SocketAddr;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub sender: UnboundedSender<Message>,
    pub query: String,
}

pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, PeerInfo>>>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublishMessage {
    pub peers: String,
    pub content: String,
}
