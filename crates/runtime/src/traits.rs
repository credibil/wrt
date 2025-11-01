//! # Service
//!
//! This module contains traits implemented by concrete WASI services.
//!
//! Each service is a module that provides a concrete implementation in support
//! of a specific set of WASI interfaces.

use std::fmt::Debug;

use anyhow::Result;

use wasmtime::component::{InstancePre, Linker};

pub trait State: Clone + Send + Sync + 'static {
    type StoreData: Send + 'static;

    #[must_use]
    fn new_store(&self) -> Self::StoreData;

    fn instance_pre(&self) -> &InstancePre<Self::StoreData>;
}

/// Implemented by all WASI hosts in order to allow the runtime to link their
/// dependencies.
pub trait Host<T>: Debug + Sync + Send {
    /// Link the host's dependencies prior to component instantiation.
    ///
    /// # Errors
    ///
    /// Returns an linking error(s) from the service's generated bindings.
    fn add_to_linker(linker: &mut Linker<T>) -> Result<()>;
}

/// Implemented by WASI hosts that are servers in order to allow the runtime to
/// start them.
pub trait Server<S: State>: Debug + Sync + Send {
    /// Start the service.
    ///
    /// This is typically implemented by services that instantiate (or run)
    /// wasm components.
    #[allow(unused_variables)]
    fn run(&self, state: &S) -> impl Future<Output = Result<()>> {
        async { Ok(()) }
    }
}

/// WASI hosts that can be run implement this trait in order to allow the runtime to
/// start them.
pub trait Resource: Sized + Sync + Send {
    // type Connection;

    /// Connect to the resource.
    fn connect() -> impl Future<Output = Result<Self>>;
}
