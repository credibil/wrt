//! # WASI Key-Value Service

mod atomics_impl;
mod batch_impl;
mod resource;
mod store_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use self::wasi::keyvalue::store::Error;
    pub use super::{BucketProxy, Cas};

    wasmtime::component::bindgen!({
        world: "keyvalue",
        path: "wit",
        imports: {
            default: async | tracing | trappable,
        },
        with: {
            "wasi:keyvalue/store/bucket": BucketProxy,
            "wasi:keyvalue/atomics/cas": Cas,
        },
        trappable_error_type: {
            "wasi:keyvalue/store/error" => Error,
        },
    });
}

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use futures::lock::Mutex;
pub use resource::*;
use runtime::{AddResource, RunState, WasiHost};
use wasmtime::component::{HasData, Linker, ResourceTableError};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::keyvalue::store::Error;
use self::generated::wasi::keyvalue::{atomics, batch, store};

static CLIENTS: LazyLock<Mutex<HashMap<&str, Arc<dyn Client>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub type Result<T, E = Error> = anyhow::Result<T, E>;

#[derive(Debug)]
pub struct WasiKeyValue;

impl<T: Client + 'static> AddResource<T> for WasiKeyValue {
    async fn resource(self, resource: T) -> anyhow::Result<Self> {
        CLIENTS.lock().await.insert(resource.name(), Arc::new(resource));
        Ok(self)
    }
}

impl WasiHost for WasiKeyValue {
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
