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
            default: async | store | tracing | trappable,
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

use std::fmt::Debug;
use std::sync::Arc;

use runtime::Host;
use wasmtime::component::{HasData, Linker, ResourceTableError};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::keyvalue::store::Error;
use self::generated::wasi::keyvalue::{atomics, batch, store};
pub use self::resource::*;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

impl<T> Host<T> for WasiKeyValue
where
    T: WasiKeyValueView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        store::add_to_linker::<_, Self>(linker, T::keyvalue)?;
        atomics::add_to_linker::<_, Self>(linker, T::keyvalue)?;
        batch::add_to_linker::<_, Self>(linker, T::keyvalue)
    }
}

#[derive(Debug)]
pub struct WasiKeyValue;
impl HasData for WasiKeyValue {
    type Data<'a> = WasiKeyValueCtxView<'a>;
}

/// A trait which provides internal WASI Key-Value context.
///
/// This is implemented by the resource-specific provider of Key-Value
/// functionality. For example, an in-memory store, or a Redis-backed store.
pub trait WasiKeyValueCtx: Debug + Send + Sync + 'static {
    fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>>;
}

/// View into [`WasiKeyValueCtx`] implementation and [`ResourceTable`].
pub struct WasiKeyValueCtxView<'a> {
    /// Mutable reference to the WASI Key-Value context.
    pub ctx: &'a mut dyn WasiKeyValueCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI Key-Value state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiKeyValueView: Send {
    /// Return a [`WasiKeyValueCtxView`] from mutable reference to self.
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_>;
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}
