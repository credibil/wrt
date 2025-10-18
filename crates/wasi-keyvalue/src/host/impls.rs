mod atomics;
mod batch;
mod store;

use std::sync::Arc;

use crate::host::CLIENTS;
use crate::host::generated::wasi::keyvalue::store::Error;
use crate::host::resource::ClientProxy;
use anyhow::anyhow;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

impl ClientProxy {
    async fn try_from(name: &str) -> anyhow::Result<Self> {
        let clients = CLIENTS.lock().await;
        let Some(client) = clients.get(name) else {
            return Err(anyhow!("client '{name}' is not registered"))?;
        };
        Ok(Self(Arc::clone(client)))
    }
}
