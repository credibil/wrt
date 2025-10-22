//! # Host implementation for WASI Vault Service
//!
//! This module implements the host-side logic for the WASI Vault service.

mod resource;
mod vault_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use self::wasi::vault::vault::Error;
    pub use super::LockerProxy;

    wasmtime::component::bindgen!({
        world: "vault",
        path: "wit",
        imports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:vault/vault/locker": LockerProxy,
        },
        trappable_error_type: {
            "wasi:vault/vault/error" => Error,
        },
    });
}

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use futures::lock::Mutex;
use runtime::{AddResource, RunState, WasiHost};
use wasmtime::component::{HasData, Linker, ResourceTableError};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::vault::vault;
use self::generated::wasi::vault::vault::Error;
pub use crate::host::resource::*;

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct WasiVault;

impl<T: Client + 'static> AddResource<T> for WasiVault {
    async fn resource(self, resource: T) -> anyhow::Result<Self> {
        CLIENTS.lock().await.insert(resource.name(), Arc::new(resource));
        Ok(self)
    }
}

impl WasiHost for WasiVault {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> anyhow::Result<()> {
        vault::add_to_linker::<_, Data>(linker, Host::new)
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

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}
