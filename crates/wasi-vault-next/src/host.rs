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
            default: async | store | tracing | trappable,
        },
        with: {
            "wasi:vault/vault/locker": LockerProxy,
        },
        trappable_error_type: {
            "wasi:vault/vault/error" => Error,
        },
    });
}

use std::fmt::Debug;
use std::sync::Arc;

use runtime::Host;
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::vault::vault;
pub use crate::host::resource::*;

impl<T> Host<T> for WasiVault
where
    T: WasiVaultView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        vault::add_to_linker::<_, Self>(linker, T::vault)
    }
}

#[derive(Debug)]
pub struct WasiVault;
impl HasData for WasiVault {
    type Data<'a> = WasiVaultCtxView<'a>;
}

/// A trait which provides internal WASI Key-Value context.
///
/// This is implemented by the resource-specific provider of Key-Value
/// functionality. For example, an in-memory store, or a Redis-backed store.
pub trait WasiVaultCtx: Debug + Send + Sync + 'static {
    fn open_locker(&self, identifier: String) -> FutureResult<Arc<dyn Locker>>;
}

/// View into [`WasiVaultCtx`] implementation and [`ResourceTable`].
pub struct WasiVaultCtxView<'a> {
    /// Mutable reference to the WASI Key-Value context.
    pub ctx: &'a mut dyn WasiVaultCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI Key-Value state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiVaultView: Send {
    /// Return a [`WasiVaultCtxView`] from mutable reference to self.
    fn vault(&mut self) -> WasiVaultCtxView<'_>;
}
