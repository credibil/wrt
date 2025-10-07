//! # WebAssembly Runtime

use std::path::PathBuf;
use std::{env, vec};

use anyhow::Result;
use cfg_if::cfg_if;
use credibil_otel::Telemetry;
use futures::future::{BoxFuture, FutureExt};
use tracing::instrument;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine};

use crate::traits::Service;

pub struct RuntimeBuilder {
    wasm: PathBuf,
    services: Vec<Box<dyn Service>>,
    telemetry: bool,
}

impl RuntimeBuilder {
    /// Create a new Runtime instance from the provided file reference.
    ///
    /// The file can either be a serialized (pre-compiled) wasmtime `Component`
    /// or a standard `wasm32-wasip2` wasm component.
    ///
    /// Set telemetry to true to enable OpenTelemetry tracing.
    #[must_use]
    pub fn new(wasm: PathBuf, telemetry: bool) -> Self {
        let res = Self {
            wasm,
            services: vec![],
            telemetry,
        };
        res.init()
    }

    /// Register a service with the runtime.
    ///
    /// The service must have implemented the [`Service`] trait in order to
    /// register.
    #[must_use]
    pub fn register<S: Service + 'static>(mut self, service: S) -> Self {
        self.services.push(Box::new(service));
        self
    }

    /// Return the runtime instance
    #[must_use]
    pub fn build(self) -> Runtime {
        Runtime {
            wasm: self.wasm,
            services: self.services,
        }
    }

    /// Initialise telemetry for the runtime.
    fn init(self) -> Self {
        if self.telemetry {
            let file_name = self.wasm.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
            let (prefix, _) = file_name.rsplit_once('.').unwrap_or((file_name, ""));

            // initialize telemetry
            let mut builder = Telemetry::new(prefix);
            if let Ok(endpoint) = env::var("OTEL_GRPC_ADDR") {
                builder = builder.endpoint(endpoint);
            }
            builder.build().unwrap_or_else(|e| {
                tracing::warn!("failed to initialize telemetry: {e}");
            });
        }
        self
    }
}

/// Runtime for a wasm component.
pub struct Runtime {
    wasm: PathBuf,
    services: Vec<Box<dyn Service>>,
}

impl Runtime {
    /// Start the runtime, instantiating each registered service on its own
    /// thread.
    ///
    /// This function will block until a shutdown signal is received from the OS.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue processing the shutdown signal.
    pub async fn serve(self) -> Result<()> {
        self.init_runtime()?;

        // wait for shutdown signal
        Ok(tokio::signal::ctrl_c().await?)
    }

    #[instrument(name = "runtime", skip(self))]
    fn init_runtime(self) -> Result<()> {
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

        for service in &self.services {
            service.add_to_linker(&mut linker)?;
        }

        // start services
        let instance_pre = linker.instantiate_pre(&component)?;
        for service in self.services {
            let instance_pre = instance_pre.clone();
            tokio::spawn(async move {
                if let Err(e) = service.start(instance_pre).await {
                    tracing::warn!("issue starting {service:?} service: {e}");
                }
            });
        }

        tracing::info!("runtime intialized");

        Ok(())
    }
}

impl IntoFuture for Runtime {
    type IntoFuture = BoxFuture<'static, Result<()>>;
    type Output = Result<()>;

    fn into_future(self) -> Self::IntoFuture {
        self.serve().boxed()
    }
}
