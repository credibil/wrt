# Credibil Wasm Runtime

The Credibil Wasm runtime provides a thin wrapper around [`wasmtime`](https://github.com/bytecodealliance/wasmtime) 
for ergonomic integration of host-based services for WASI components.

We consider this a stop-gap solution until production-grade runtimes support dynamic inclusion of host-based 
services.

## Example Runtimes

There are a number of examples provided in the `examples` directory and a Docker compose file that can be used 
to run them.

To get started, add a `.env` file to the [examples/runtimes](./examples/runtimes) directory (see `.env.example`)
and run:

```bash
docker compose --file ./examples/runtimes/compose.yaml up
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

## Example Guests

Example guests can be found in the [examples](./examples) directory. Instructions for building and running each example can be found in the respective README files.

## Architecture & Key Patterns

- **Crate Structure:**
  - Core runtime logic is in `src/` (e.g., `runtime.rs`, `state.rs`, `cli.rs`).
  - Host service implementations are in `crates/`, e.g.:
    - `wasi-blobstore-nats`: WASI blobstore service backed by NATS JetStream ObjectStore.
    - `sdk-http`, `sdk-otel`, etc.: SDKs and service integrations for guests.
  - WASI interface definitions and bindings are in `wit/` and `crates/wit-bindings/`.
  - 
- **Service Pattern:**
  - Each host service implements a `Service` struct and the `runtime::Service` trait, providing an `add_to_linker` method for wasmtime integration.
  - Services use resource tables for managing handles to NATS, object stores, etc.
  - 
- **Component Communication:**
  - Host/guest communication is via WASI interfaces (see `wit/`).
  - NATS JetStream is used for blobstore and messaging services.

## Developer Workflows

- **Build All:**
  - Use `make build` (delegates to `cargo make build`).
  - 
- **Run Example Runtimes:**
  - Add a `.env` file to `examples/runtimes/` (see `.env.example`).
  - Start with: `docker compose --file ./examples/runtimes/compose.yaml up`
  - 
- **Build and Run Guests:**
  - Example: `cargo build --package blobstore --target wasm32-wasip2 --release`
  - Run: `cargo run -- run ./target/wasm32-wasip2/release/blobstore.wasm`
  - 
- **Pre-compile and Run:**
  - `cargo run -- compile ./target/wasm32-wasip2/release/blobstore.wasm --output ./blobstore.bin`
  - `cargo run -- run ./blobstore.bin`

## Conventions & Integration

- **Error Handling:** Use `anyhow::Result` for fallible operations. Errors are logged with `tracing`.
- 
- **Async:** All host service methods are async and use `tokio`.
- 
- **Resource Management:** Use `ResourceTable` for managing handles to external resources (NATS, object stores, etc.).
- 
- **WIT Bindings:** WASI interfaces are bound using `wasmtime::component::bindgen!` macros in each service crate.
- 
- **External Services:**
  - NATS is required for blobstore/messaging. Configure via `.env` in `examples/runtimes/`.