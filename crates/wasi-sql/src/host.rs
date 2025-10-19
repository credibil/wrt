//! # Host implementation for WASI Vault Service
//!
//! This module implements the host-side logic for the WASI Vault service.

mod readwrite_impl;
mod resource;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use super::{ConnectionProxy, Statement};
    pub use anyhow::Error;

    wasmtime::component::bindgen!({
        world: "sql",
        path: "wit",
        imports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:sql/types/connection": ConnectionProxy,
            "wasi:sql/types/statement": Statement,
            "wasi:sql/types/error": Error,
        },
        trappable_error_type: {
            "wasi:sql/types/error" => Error,
        },
    });
}

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use futures::lock::Mutex;
use runtime::{RunState, Service};
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::sql::{readwrite, types};
pub use crate::host::resource::*;

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct WasiSql;

impl WasiSql {
    /// Register SQL connection client implementations with the host.
    ///
    /// # Errors
    ///
    /// If the client could not be registered
    pub async fn resource(self, client: impl Client + 'static) -> Self {
        CLIENTS.lock().await.insert(client.name(), Arc::new(client));
        self
    }
}

impl Service for WasiSql {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> anyhow::Result<()> {
        readwrite::add_to_linker::<_, Data>(linker, Host::new)?;
        types::add_to_linker::<_, Data>(linker, Host::new)
    }
}

struct Data;
impl HasData for Data {
    type Data<'a> = Host<'a>;
}

pub struct Host<'a> {
    table: &'a mut ResourceTable,
}

impl Host<'_> {
    const fn new(c: &mut RunState) -> Host<'_> {
        Host { table: &mut c.table }
    }
}
