//! # WebAssembly Runtime

use std::env;
use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::{Context, Result};
use cfg_if::cfg_if;
use credibil_otel::Telemetry;
use tracing::instrument;
use wasmtime::component::{Component, InstancePre, Linker};
use wasmtime::{Config, Engine};
use wasmtime_wasi::WasiView;

use crate::traits::Host;

/// Runtime for a wasm component.
pub struct RuntimeNext<T: WasiView + 'static> {
    wasm: PathBuf,
    tracing: bool,
    linker: Option<Linker<T>>,
    component: Option<Component>,
    _marker: PhantomData<T>,
}

impl<T: WasiView> RuntimeNext<T> {
    /// Create a new Runtime instance from the provided file reference.
    ///
    /// The file can either be a serialized (pre-compiled) wasmtime `Component`
    /// or a standard `wasm32-wasip2` wasm component.
    #[must_use]
    pub const fn new(wasm: PathBuf) -> Self {
        Self {
            wasm,
            tracing: true,
            linker: None,
            component: None,
            _marker: PhantomData,
        }
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
    pub fn compile(self) -> Result<Self> {
        if self.tracing {
            self.init_tracing()?;
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

        // file can be a serialized component or a wasm file
        cfg_if! {
            if #[cfg(feature = "jit")] {
                // SAFETY:
                // Attempt to load as a serialized component with fallback to wasm
                let component = match unsafe { Component::deserialize_file(&engine, &self.wasm) } {
                    Ok(component) => component,
                    Err(_) => Component::from_file(&engine, &self.wasm)?,
                };
            } else {
                // load as a serialized component with no fallback (Cranelift is unavailable)
                let component = unsafe { Component::deserialize_file(&engine, &self.wasm)? };
            }
        }

        // register services with runtime's Linker
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi::p3::add_to_linker(&mut linker)?;

        tracing::info!("runtime intialized");

        Ok(Self {
            wasm: self.wasm,
            tracing: self.tracing,
            linker: Some(linker),
            component: Some(component),
            _marker: PhantomData,
        })
    }

    /// Link a WASI host to the runtime.
    pub fn link<H: Host<T>>(&mut self, _: H) -> Result<()> {
        H::add_to_linker(self.linker.as_mut().unwrap())?;
        Ok(())
    }

    /// Ppre-instantiate component.
    pub fn pre_instantiate(&mut self) -> Result<InstancePre<T>> {
        let component = self.component.as_ref().unwrap();
        self.linker.as_ref().unwrap().instantiate_pre(component)
    }

    fn init_tracing(&self) -> Result<()> {
        let file_name = self.wasm.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
        let (prefix, _) = file_name.rsplit_once('.').unwrap_or((file_name, ""));

        // initialize telemetry
        let mut builder = Telemetry::new(prefix);
        if let Ok(endpoint) = env::var("OTEL_GRPC_ADDR") {
            builder = builder.endpoint(endpoint);
        }
        builder.build().context("initializing telemetry")
    }
}
