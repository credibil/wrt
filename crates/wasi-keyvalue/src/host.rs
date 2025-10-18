//! # WASI Key-Value Service

mod impls;
mod resource;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use self::wasi::keyvalue::store::Error;
    pub use super::{BucketProxy, Cas, ClientProxy};

    wasmtime::component::bindgen!({
        world: "keyvalue",
        path: "wit",
        imports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:keyvalue/store/client": ClientProxy,
            "wasi:keyvalue/store/bucket": BucketProxy,
            "wasi:keyvalue/atomics/cas": Cas,
        },
        trappable_error_type: {
            "wasi:keyvalue/store/error" => Error,
        },
    });
}

use std::sync::Arc;

use futures::lock::Mutex;
pub use resource::*;
use runtime::{RunState, Service};
use std::collections::HashMap;
use std::sync::LazyLock;
use wasmtime::component::{HasData, Linker, ResourceTableError};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::keyvalue::store::Error;
use self::generated::wasi::keyvalue::{atomics, batch, store};

pub use crate::host::resource::{BucketProxy, Cas, ClientProxy};

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub struct WasiKeyValue;

impl WasiKeyValue {
    /// Register a messaging client with the host
    ///
    /// # Errors
    ///
    /// If the client could not be registered
    pub async fn client(self, client: impl Client + 'static) -> Self {
        CLIENTS.lock().await.insert(client.name(), Arc::new(client));
        self
    }
}

impl Service for WasiKeyValue {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> anyhow::Result<()> {
        store::add_to_linker::<_, Data>(linker, Host::new)?;
        atomics::add_to_linker::<_, Data>(linker, Host::new)?;
        batch::add_to_linker::<_, Data>(linker, Host::new)?;
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

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}
