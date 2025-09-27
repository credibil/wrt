use axum::routing::{options, post};
use axum::{Json, Router};
use http::Method;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::{KeyValue, global};
use sdk_http::Result;
use serde_json::{Value, json};
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};

struct HttpGuest;

impl Guest for HttpGuest {
    #[sdk_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    fn handle(request: IncomingRequest, response: ResponseOutparam) {
        // tracing metrics
        tracing::info!(monotonic_counter.tracing_counter = 1, key1 = "value 1");
        tracing::info!(gauge.tracing_gauge = 1);

        // otel metrics
        let meter = global::meter("my_meter");
        let counter = meter.u64_counter("otel_counter").build();
        counter.add(1, &[KeyValue::new("key1", "value 1")]);

        // otel span
        let tracer = global::tracer("basic");
        tracer.in_span("main-operation", |cx| {
            let span = cx.span();
            span.set_attribute(KeyValue::new("my-attribute", "my-value"));
            span.add_event("main span event", vec![KeyValue::new("foo", "1")]);

            tracer.in_span("child-operation", |cx| {
                cx.span().add_event("sub span event", vec![KeyValue::new("bar", "1")]);
            });

            // tracing span within otel span
            tracing::info_span!("child info span").in_scope(|| {
                tracing::info!("info event");
            });
        });

        let out = tracing::info_span!("handler span").in_scope(|| {
            tracing::info!("received request");
            let router = Router::new()
                .layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                        .allow_headers(Any)
                        .allow_origin(Any),
                )
                .route("/", post(handler))
                .route("/", options(handle_options));
            sdk_http::serve(router, request)
        });

        ResponseOutparam::set(response, out);
    }
}

// A simple "Hello, World!" endpoint that returns the client's request.
#[axum::debug_handler]
#[sdk_otel::instrument]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    tracing::info!("handling request: {:?}", body);
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}

// Handle preflight OPTIONS requests for CORS.
async fn handle_options() -> Result<()> {
    Ok(())
}

wasi::http::proxy::export!(HttpGuest);
