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

use wasmtime::component::{HasData, Linker, ResourceTableError};
use wasmtime_wasi::ResourceTable;

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

pub trait WasiKeyValueView: Send {
    /// Returns the table used to manage resources.
    fn table(&mut self) -> &mut ResourceTable;

    fn client(&mut self) -> impl Client;
}

impl<T: WasiKeyValueView> WasiKeyValueView for WasiKeyValueImpl<T> {
    fn table(&mut self) -> &mut ResourceTable {
        self.0.table()
    }

    fn client(&mut self) -> impl Client {
        self.0.client()
    }
}

impl<T: ?Sized + WasiKeyValueView> WasiKeyValueView for &mut T {
    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }

    fn client(&mut self) -> impl Client {
        T::client(self)
    }
}
