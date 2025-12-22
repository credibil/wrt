//! # OpenTelemetry Wasm Guest
//!
//! This module demonstrates comprehensive OpenTelemetry instrumentation in a
//! WebAssembly guest. It showcases both the `tracing` API and the native
//! OpenTelemetry API for distributed tracing and metrics.
//!
//! ## Two Approaches
//!
//! 1. **Tracing API**: Ergonomic, Rust-idiomatic, uses `#[instrument]` macro
//! 2. **OpenTelemetry API**: Direct OTel SDK access for advanced use cases
//!
//! Both approaches export telemetry through the host's WASI OTel implementation,
//! which forwards to an OpenTelemetry Collector via OTLP.
//!
//! ## What Gets Exported
//!
//! - **Spans**: Timing and context for operations (traces)
//! - **Events**: Timestamped log entries within spans
//! - **Attributes**: Key-value metadata on spans
//! - **Metrics**: Counters and gauges for measurements

#![cfg(target_arch = "wasm32")]

use axum::routing::{options, post};
use axum::{Json, Router};
use http::Method;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{KeyValue, global};
use serde_json::{Value, json};
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    /// Main request handler demonstrating various telemetry patterns.
    ///
    /// This handler showcases:
    /// 1. Tracing-based metrics (counters, gauges)
    /// 2. OpenTelemetry metrics API
    /// 3. OpenTelemetry spans with attributes and events
    /// 4. Nested spans (parent-child relationships)
    /// 5. Mixing tracing and OTel spans
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        // =====================================================================
        // TRACING-BASED METRICS
        // =====================================================================
        // The `tracing` crate supports metrics via special field prefixes.
        // These are automatically exported as OpenTelemetry metrics.

        // Monotonic counter: always increases (good for request counts, errors)
        tracing::info!(monotonic_counter.tracing_counter = 1, key1 = "value 1");

        // Gauge: current value that can go up or down (connections, queue size)
        tracing::info!(gauge.tracing_gauge = 1);

        // =====================================================================
        // OPENTELEMETRY METRICS API
        // =====================================================================
        // Direct access to the OTel metrics API for more control.

        // Get a meter (named metric producer)
        let meter = global::meter("my_meter");

        // Create and increment a counter with attributes
        let counter = meter.u64_counter("otel_counter").build();
        counter.add(1, &[KeyValue::new("key1", "value 1")]);

        // =====================================================================
        // OPENTELEMETRY SPANS
        // =====================================================================
        // Spans represent timed operations in a distributed trace.

        // Get a tracer (named span producer)
        let tracer = global::tracer("basic");

        // Create a span for a logical operation
        tracer.in_span("main-operation", |cx| {
            // Access the current span from the context
            let span = cx.span();

            // Attributes: key-value metadata about the operation
            // Use for things like user IDs, request parameters, etc.
            span.set_attribute(KeyValue::new("my-attribute", "my-value"));

            // Events: timestamped log entries within the span
            // Use for significant moments (cache hit, retry, etc.)
            span.add_event("main span event", vec![KeyValue::new("foo", "1")]);

            // Nested span: creates parent-child relationship in traces
            // Child spans help break down complex operations
            tracer.in_span("child-operation", |cx| {
                cx.span().add_event("sub span event", vec![KeyValue::new("bar", "1")]);
            });

            // =====================================================================
            // MIXING TRACING AND OTEL
            // =====================================================================
            // You can create tracing spans within OpenTelemetry spans.
            // Both will appear in your traces with proper parent-child links.
            tracing::info_span!("child info span").in_scope(|| {
                tracing::info!("info event");
            });
        });

        // =====================================================================
        // REQUEST HANDLING
        // =====================================================================
        // Wrap the actual request handling in a tracing span
        tracing::info_span!("handler span")
            .in_scope(|| {
                tracing::info!("received request");

                // Build router with CORS support for browser testing
                let router = Router::new()
                    .layer(
                        CorsLayer::new()
                            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                            .allow_headers(Any)
                            .allow_origin(Any),
                    )
                    .route("/", post(handler))
                    .route("/", options(handle_options));

                wasi_http::serve(router, request)
            })
            .await
    }
}

/// Simple JSON echo handler.
///
/// Demonstrates that regular business logic handlers can also be instrumented.
/// The `#[wasi_otel::instrument]` macro automatically creates a span for this
/// function with timing information.
#[axum::debug_handler]
#[wasi_otel::instrument]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    tracing::info!("handling request: {:?}", body);
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}

/// Handle CORS preflight OPTIONS requests.
///
/// Browsers send OPTIONS requests before cross-origin POST/PUT/DELETE.
/// This handler satisfies those preflight checks.
async fn handle_options() -> Result<()> {
    Ok(())
}
