use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use crate::host::store_impl::WebSocketServer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishMessage {
    pub peers: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct WebSocketProxy(pub Arc<dyn WebSocketServer>);

impl Deref for WebSocketProxy {
    type Target = Arc<dyn WebSocketServer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
