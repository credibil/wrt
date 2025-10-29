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

use std::fmt::Debug;
use std::sync::Arc;

use wasmtime::component::{HasData, Linker, ResourceTableError};
use wasmtime_wasi::ResourceTable;
use runtime::{ WasiHost};

use self::generated::wasi::keyvalue::store::Error;
use self::generated::wasi::keyvalue::{atomics, batch, store};
pub use self::resource::*;

pub type Result<T, E = Error> = anyhow::Result<T, E>;


/// Add all of the `wasi:keyvalue` world's interfaces to a
/// [`wasmtime::component::Linker`].
///
/// # Errors
///
/// Will return an error if one or more of the interfaces could not be added to
/// the linker.
pub fn add_to_linker<T: WasiKeyValueView + 'static>(linker: &mut Linker<T>) -> anyhow::Result<()> {
    store::add_to_linker::<_, WasiKeyValue<T>>(linker, |x| WasiKeyValueImpl(x))?;
    atomics::add_to_linker::<_, WasiKeyValue<T>>(linker, |x| WasiKeyValueImpl(x))?;
    batch::add_to_linker::<_, WasiKeyValue<T>>(linker, |x| WasiKeyValueImpl(x))
}

#[repr(transparent)]
struct WasiKeyValueImpl<T>(pub T);

struct WasiKeyValue<T>(T);
impl<T: 'static> HasData for WasiKeyValue<T> {
    type Data<'a> = WasiKeyValueImpl<&'a mut T>;
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}

/// A trait which provides internal WASI Key-Value state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiKeyValueView: Send {
    /// Return a [`WasiKeyValueCtxView`] from mutable reference to self.
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_>;
}

impl<T: WasiKeyValueView> WasiKeyValueView for WasiKeyValueImpl<T> {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        self.0.keyvalue()
    }
}

impl<T: ?Sized + WasiKeyValueView> WasiKeyValueView for &mut T {
    fn keyvalue(&mut self) -> WasiKeyValueCtxView<'_> {
        T::keyvalue(self)
    }
}

/// View into [`WasiKeyValueCtx`] implementation and [`ResourceTable`].
pub struct WasiKeyValueCtxView<'a> {
    /// Mutable reference to the WASI Key-Value context.
    pub ctx: &'a mut dyn WasiKeyValueCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI Key-Value context.
///
/// This is implemented by the resource-specific provider of Key-Value
/// functionality. For example, an in-memory store, or a Redis-backed store.
pub trait WasiKeyValueCtx: Debug + Send + Sync + 'static {
    fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>>;
}
