mod producer;
mod request_reply;
mod types;

use std::sync::Arc;

use anyhow::{Result, anyhow};

use crate::host::CLIENTS;
pub use crate::host::generated::wasi::messaging::types::Error;
use crate::host::resource::ClientProxy;

impl ClientProxy {
    async fn try_from(name: &str) -> Result<Self> {
        let clients = CLIENTS.lock().await;
        let Some(client) = clients.get(name) else {
            return Err(anyhow!("client '{name}' is not registered"))?;
        };
        Ok(Self(Arc::clone(client)))
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
