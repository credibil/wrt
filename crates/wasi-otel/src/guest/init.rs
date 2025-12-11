//! Initialise OpenTelemetry

use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use cfg_if::cfg_if;
use opentelemetry::{KeyValue, Value};
use opentelemetry_sdk::Resource;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::guest::generated::wasi::otel::{resource, types};

cfg_if! {
    if #[cfg(feature = "metrics" )] {
        use opentelemetry_sdk::metrics::SdkMeterProvider;
        use tracing_opentelemetry::MetricsLayer;
        use crate::guest::metrics;
    }
}
cfg_if! {
    if #[cfg(feature = "tracing" )] {
        use opentelemetry_sdk::trace::SdkTracerProvider;
        use tracing_opentelemetry::layer as tracing_layer;
        use tracing_subscriber::EnvFilter;
        use opentelemetry::trace::TracerProvider;
        use crate::guest::tracing;
    }
}

pub static INIT: OnceLock<bool> = OnceLock::new();

pub fn init() -> Result<ExitGuard> {
    // get WASI host telemetry resource
    let resource: Resource = resource::resource().into();

    // create subscriber layers
    let filter_layer = EnvFilter::from_default_env()
        .add_directive("hyper=off".parse()?)
        .add_directive("h2=off".parse()?)
        .add_directive("tonic=off".parse()?);
    let fmt_layer = tracing_subscriber::fmt::layer();
    let registry = Registry::default().with(filter_layer).with(fmt_layer);

    let mut guard = ExitGuard::default();

    // initialize tracing
    #[cfg(feature = "tracing")]
    let registry = {
        let tracer_provider = tracing::init(resource.clone());
        let tracing_layer = tracing_layer().with_tracer(tracer_provider.tracer("global"));
        guard.tracing = tracer_provider;
        registry.with(tracing_layer)
    };

    // initialize metrics
    #[cfg(feature = "metrics")]
    let registry = {
        let meter_provider = metrics::init(resource);
        let metrics_layer = MetricsLayer::new(meter_provider.clone());
        guard.metrics = meter_provider;
        registry.with(metrics_layer)
    };

    registry.try_init().context("issue initializing subscriber")?;
    INIT.set(true).map_err(|_e| anyhow!("wasi-otel already initialized"))?;

    Ok(guard)
}

/// [`ExitGuard`] provides a guard to export telemetry data on drop.
#[derive(Debug, Default)]
pub struct ExitGuard {
    #[cfg(feature = "tracing")]
    tracing: SdkTracerProvider,
    #[cfg(feature = "metrics")]
    metrics: SdkMeterProvider,
}

impl Drop for ExitGuard {
    fn drop(&mut self) {
        #[cfg(feature = "tracing")]
        if let Err(e) = self.tracing.shutdown() {
            ::tracing::error!("failed to export tracing: {e}");
        }
        #[cfg(feature = "metrics")]
        if let Err(e) = self.metrics.shutdown() {
            ::tracing::error!("failed to export metrics: {e}");
        }
    }
}

impl From<types::Resource> for Resource {
    fn from(value: types::Resource) -> Self {
        let attrs = value.attributes.into_iter().map(Into::into).collect::<Vec<_>>();
        let builder = Self::builder();

        if let Some(schema_url) = value.schema_url {
            builder.with_schema_url(attrs, schema_url).build()
        } else {
            builder.with_attributes(attrs).build()
        }
    }
}

impl From<types::KeyValue> for KeyValue {
    fn from(value: types::KeyValue) -> Self {
        Self::new(value.key, value.value)
    }
}

impl From<types::Value> for Value {
    fn from(value: types::Value) -> Self {
        match value {
            types::Value::Bool(v) => Self::Bool(v),
            types::Value::S64(v) => Self::I64(v),
            types::Value::F64(v) => Self::F64(v),
            types::Value::String(v) => Self::String(v.into()),
            types::Value::BoolArray(items) => Self::Array(opentelemetry::Array::Bool(items)),
            types::Value::S64Array(items) => Self::Array(opentelemetry::Array::I64(items)),
            types::Value::F64Array(items) => Self::Array(opentelemetry::Array::F64(items)),
            types::Value::StringArray(items) => Self::Array(opentelemetry::Array::String(
                items.into_iter().map(Into::into).collect(),
            )),
        }
    }
}
