//! # WASI Blobstore Service

mod blobstore_impl;
mod container_impl;
mod resource;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use super::{Container, IncomingValue, OutgoingValue, StreamObjectNames};

    wasmtime::component::bindgen!({
        world: "blobstore",
        path: "wit",
        imports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:io": wasmtime_wasi::p2::bindings::io,

            "wasi:blobstore/types/incoming-value": IncomingValue,
            "wasi:blobstore/types/outgoing-value": OutgoingValue,
            "wasi:blobstore/container/container": Container,
            "wasi:blobstore/container/stream-object-names": StreamObjectNames,
        },
        trappable_error_type: {
            "wasi:blobstore/types/error" => anyhow::Error,
        },
    });
}

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use anyhow::Result;
use async_nats::jetstream::object_store::ObjectStore;
use bytes::Bytes;
use futures::lock::Mutex;
use runtime::RunState;
use wasmtime::component::{HasData, Linker, ResourceTable};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;

use self::generated::wasi::blobstore::{blobstore, container, types};
use crate::host::resource::Client;

pub type Container = ObjectStore;
pub type IncomingValue = Bytes;
pub type OutgoingValue = MemoryOutputPipe;
pub type StreamObjectNames = Vec<String>;

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct WasiBlobstore;

impl WasiBlobstore {
    /// Register a messaging client with the host
    ///
    /// # Errors
    ///
    /// If the client could not be registered
    pub async fn client(self, client: impl Client + 'static) {
        CLIENTS.lock().await.insert(client.name(), Arc::new(client));
    }
}

impl runtime::Service for WasiBlobstore {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> Result<()> {
        blobstore::add_to_linker::<_, Data>(linker, Host::new)?;
        container::add_to_linker::<_, Data>(linker, Host::new)?;
        types::add_to_linker::<_, Data>(linker, Host::new)?;
        Ok(())
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
