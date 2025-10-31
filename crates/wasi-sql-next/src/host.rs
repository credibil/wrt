//! # Host implementation for WASI Vault Service
//!
//! This module implements the host-side logic for the WASI Vault service.

mod readwrite_impl;
mod resource;
mod types_impl;

mod generated {
    #![allow(clippy::trait_duplication_in_bounds)]

    pub use anyhow::Error;

    pub use super::{ConnectionProxy, Statement};

    wasmtime::component::bindgen!({
        world: "sql",
        path: "wit",
        imports: {
            default: async | store | tracing | trappable,
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

use std::fmt::Debug;
use std::sync::Arc;

use runtime::Host;
use wasmtime::component::{HasData, Linker};
use wasmtime_wasi::ResourceTable;

use self::generated::wasi::sql::{readwrite, types};
pub use crate::host::resource::*;

impl<T> Host<T> for WasiSql
where
    T: WasiSqlView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        readwrite::add_to_linker::<_, Self>(linker, T::sql)?;
        types::add_to_linker::<_, Self>(linker, T::sql)
    }
}

#[derive(Debug)]
pub struct WasiSql;
impl HasData for WasiSql {
    type Data<'a> = WasiSqlCtxView<'a>;
}

/// A trait which provides internal WASI Key-Value context.
///
/// This is implemented by the resource-specific provider of Key-Value
/// functionality. For example, an in-memory store, or a Redis-backed store.
pub trait WasiSqlCtx: Debug + Send + Sync + 'static {
    fn open_connection(&self, identifier: String) -> FutureResult<Arc<dyn Connection>>;
}

/// View into [`WasiSqlCtx`] implementation and [`ResourceTable`].
pub struct WasiSqlCtxView<'a> {
    /// Mutable reference to the WASI Key-Value context.
    pub ctx: &'a mut dyn WasiSqlCtx,

    /// Mutable reference to table used to manage resources.
    pub table: &'a mut ResourceTable,
}

/// A trait which provides internal WASI Key-Value state.
///
/// This is implemented by the `T` in `Linker<T>` — a single type shared across
/// all WASI components for the runtime build.
pub trait WasiSqlView: Send {
    /// Return a [`WasiSqlCtxView`] from mutable reference to self.
    fn sql(&mut self) -> WasiSqlCtxView<'_>;
}
