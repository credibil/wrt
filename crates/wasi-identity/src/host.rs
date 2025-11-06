//! # Host implementation for WASI Vault Service
//!
//! This module implements the host-side logic for the WASI Vault service.

mod resource;
mod identity_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use self::wasi::identity::identity::Error;

    wasmtime::component::bindgen!({
        world: "identity",
        path: "wit",
        imports: {
            default: async | store | tracing | trappable,
        },
        trappable_error_type: {
            "wasi:identity/types/error" => Error,
        },
    });
}

use std::fmt::Debug;
use std::sync::Arc;

use runtime::Host;
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::identity::identity;
pub use crate::host::resource::*;

impl<T> Host<T> for WasiIdentity
where
    T: WasiIdentityView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        identity::add_to_linker::<_, Self>(linker, T::identity)
    }
}

#[derive(Debug)]
pub struct WasiIdentity;
impl HasData for WasiIdentity {
    type Data<'a> = WasiIdentityCtxView<'a>;
}

/// A trait which provides internal WASI Key-Value context.
///
/// This is implemented by the resource-specific provider of Key-Value
/// functionality. For example, an in-memory store, or a Redis-backed store.
pub trait WasiIdentityCtx: Debug + Send + Sync + 'static {
    fn open_locker(&self, identifier: String) -> FutureResult<Arc<dyn Locker>>;
}

/// View into [`WasiIdentityCtx`] implementation and [`ResourceTable`].
pub struct WasiIdentityCtxView<'a> {
    /// Mutable reference to the WASI Key-Value context.
    pub ctx: &'a mut dyn WasiIdentityCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI Key-Value state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiIdentityView: Send {
    /// Return a [`WasiIdentityCtxView`] from mutable reference to self.
    fn identity(&mut self) -> WasiIdentityCtxView<'_>;
}
