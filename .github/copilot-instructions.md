# Copilot Instructions for Tempo

## Project Overview
Tempo is a modular WASI component runtime built on [wasmtime](https://github.com/bytecodealliance/wasmtime), designed to run and compose WebAssembly components with host services. It is used by Credibil as a flexible, extensible platform for running WASI-based applications, with a focus on dynamic service inclusion and cloud-native workflows.

## Architecture & Key Patterns
- **Crate Structure:**
  - Core runtime logic is in `src/` (e.g., `runtime.rs`, `state.rs`, `cli.rs`).
  - Host service implementations are in `crates/`, e.g.:
    - `wasi-blobstore`: WASI blobstore service backed by NATS JetStream ObjectStore.
    - `sdk-http`, `sdk-otel`, etc.: SDKs and service integrations for guests.
  - WASI interface definitions and bindings are in `wit/` and `crates/wit-bindings/`.
- **Service Pattern:**
  - Each host service implements a `Service` struct and the `warp::Service` trait, providing an `add_to_linker` method for wasmtime integration.
  - Services use resource tables for managing handles to NATS, object stores, etc.
- **Component Communication:**
  - Host/guest communication is via WASI interfaces (see `wit/`).
  - NATS JetStream is used for blobstore and messaging services.

## Developer Workflows
- **Build All:**
  - Use `make build` (delegates to `cargo make build`).
- **Run Example Runtimes:**
  - Add a `.env` file to `examples/runtimes/` (see `.env.example`).
  - Start with: `docker compose --file ./examples/runtimes/compose.yaml up`
- **Build and Run Guests:**
  - Example: `cargo build --package blobstore --target wasm32-wasip2 --release`
  - Run: `cargo run -- run ./target/wasm32-wasip2/release/blobstore.wasm`
- **Pre-compile and Run:**
  - `cargo run -- compile ./target/wasm32-wasip2/release/blobstore.wasm --output ./blobstore.bin`
  - `cargo run -- run ./blobstore.bin`

## Conventions & Integration
- **Error Handling:** Use `anyhow::Result` for fallible operations. Errors are logged with `tracing`.
- **Async:** All host service methods are async and use `tokio`.
- **Resource Management:** Use `ResourceTable` for managing handles to external resources (NATS, object stores, etc.).
- **WIT Bindings:** WASI interfaces are bound using `wasmtime::component::bindgen!` macros in each service crate.
- **External Services:**
  - NATS is required for blobstore/messaging. Configure via `.env` in `examples/runtimes/`.

## References
- Main runtime: `src/`
- Host services: `crates/`
- WASI bindings: `wit/`, `crates/wit-bindings/`
- Example guests: `examples/guests/`
- Example runtimes: `examples/runtimes/`

---

**If you are unsure about a workflow or integration, check the relevant `README.md` in the corresponding crate or example directory.**
