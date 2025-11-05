//! #WASI HTTP Host
//!
//! This module implements a host-side service for `wasi:http`

mod proxy;
mod server;

use anyhow::Result;
pub use proxy::WasiHttpCtx;
use runtime::{Host, Server, State};
use wasmtime::component::Linker;
pub use wasmtime_wasi_http::p3::{WasiHttpCtxView, WasiHttpView};

#[derive(Debug)]
pub struct WasiHttp;

impl<T> Host<T> for WasiHttp
where
    T: WasiHttpView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> Result<()> {
        wasmtime_wasi_http::p3::add_to_linker(linker)
    }
}

impl<S> Server<S> for WasiHttp
where
    S: State,
    <S as State>::StoreData: WasiHttpView,
{
    async fn run(&self, state: &S) -> Result<()> {
        server::serve(state).await
    }
}
