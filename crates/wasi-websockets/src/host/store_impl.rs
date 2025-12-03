use anyhow::{Result, anyhow};
pub use runtime::FutureResult;
use wasmtime::component::{Accessor, Resource};

use crate::host::WasiWebSockets;
use crate::host::generated::wasi::websockets::store::{HostServerWithStore, HostWithStore};
use crate::host::generated::wasi::websockets::types::Peer;
use crate::host::resource::WebSocketProxy;

impl HostWithStore for WasiWebSockets {
    async fn get_server<T>(accessor: &Accessor<T, Self>) -> Result<Resource<WebSocketProxy>> {
        let server = accessor.with(|mut store| store.get().ctx.serve()).await?;
        let proxy = WebSocketProxy(server);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }
}

impl HostServerWithStore for WasiWebSockets {
    async fn get_peers<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>,
    ) -> Result<Vec<Peer>> {
        let ws_server = use_server(accessor, &self_)?;
        Ok(ws_server.get_peers())
    }

    async fn send_peers<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>, message: String,
        peers: Vec<String>,
    ) -> Result<()> {
        let ws_server = use_server(accessor, &self_)?;
        let result = ws_server.send_peers(message, peers).await;
        match result {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn send_all<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>, message: String,
    ) -> Result<()> {
        let ws_server = use_server(accessor, &self_)?;
        let result = ws_server.send_all(message).await;
        match result {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn health_check<T>(
        accessor: &Accessor<T, Self>, self_: Resource<WebSocketProxy>,
    ) -> Result<String> {
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
        Ok(())
    }
}

pub fn use_server<T>(
    accessor: &Accessor<T, WasiWebSockets>, self_: &Resource<WebSocketProxy>,
) -> Result<WebSocketProxy> {
    accessor.with(|mut store| {
        let server =
            store.get().table.get(self_).map_err(|_e| anyhow!("Failed to get WebSocket server"))?;
        Ok::<_, anyhow::Error>(server.clone())
    })
}
