//! # WebAssembly Runtime

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use credibil_otel::Telemetry;
use tracing::instrument;
use wasmtime::component::{Component, InstancePre, Linker};
use wasmtime::{Config, Engine};
use wasmtime_wasi::WasiView;

use crate::traits::Host;

/// Runtime for a wasm component.
#[derive(Default)]
pub struct Runtime {
    tracing: bool,
}

impl Runtime {
    /// Create a new Runtime instance from the provided file reference.
    ///
    /// The file can either be a serialized (pre-compiled) wasmtime `Component`
    /// or a standard `wasm32-wasip2` wasm component.
    #[must_use]
    pub const fn new() -> Self {
        Self { tracing: true }
    }

    /// Enable or disable OpenTelemetry tracing support.
    #[must_use]
    pub const fn tracing(mut self, tracing: bool) -> Self {
        self.tracing = tracing;
        self
    }

    /// Build the Wasmtime `Engine` and `Linker` for this runtime.
    ///
    /// # Errors
    ///
    /// Will fail if the provided `wasm` file cannot be compiled/deserialized
    /// as a `Component` or the `Linker` cannot be initialized with WASI
    /// support.
    #[instrument(skip(self))]
    pub fn build<T: WasiView + 'static>(self, wasm: &PathBuf) -> Result<Compiled<T>> {
        if self.tracing {
            init_tracing(wasm)?;
        }
        tracing::info!("initializing runtime");

        let mut config = Config::new();
        config.async_support(true);
        config.wasm_component_model_async(true);
        let engine = Engine::new(&config)?;

        // TODO: cause executing WebAssembly to periodically yield
        //  1. enable `Config::epoch_interruption`
        //  2. Set `Store::epoch_deadline_async_yield_and_update`
        //  3. Call `Engine::increment_epoch` periodically

        // SAFETY: The caller should ensure only valid pre-compiled wasm files are provided.
        let component = unsafe { Component::deserialize_file(&engine, wasm) }.or_else(|e| {
            if cfg!(feature = "jit") {
                Component::from_file(&engine, wasm)
            } else {
                Err(anyhow!(
                    "Issue loading component: {e}. Enable `jit` feature to load wasm32 files."
                ))
            }
        })?;

        // register services with runtime's Linker
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi::p3::add_to_linker(&mut linker)?;

        tracing::info!("runtime intialized");

        Ok(Compiled { component, linker })
    }
}

/// A compiled WebAssembly component with its associated Linker.
pub struct Compiled<T: WasiView + 'static> {
    component: Component,
    linker: Linker<T>,
}

impl<T: WasiView> Compiled<T> {
    /// Link a WASI host to the runtime.
    ///
    /// # Errors
    ///
    /// Will fail if the host cannot be added to the Linker.
    pub fn link<H: Host<T>>(&mut self, _: H) -> Result<()> {
        H::add_to_linker(&mut self.linker)
    }

    /// Pre-instantiate component.
    ///
    /// # Errors
    ///
    /// Will fail if the component cannot be pre-instantiated.
    pub fn pre_instantiate(&mut self) -> Result<InstancePre<T>> {
        self.linker.instantiate_pre(&self.component)
    }
}

fn init_tracing(wasm: &Path) -> Result<()> {
    let name = wasm.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

    // initialize telemetry
    let mut builder = Telemetry::new(name);
    if let Ok(endpoint) = env::var("OTEL_GRPC_URL") {
        builder = builder.endpoint(endpoint);
    }
    builder.build().context("initializing telemetry")
}
