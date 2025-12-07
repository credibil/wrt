# Examples

The `examples` directory contains several example projects demonstrating each WASI component
in combination with implemented resources. For example `wasi-keyvalue` combined with either
a Redis or NATS JetStream backend.

## Running Examples

You can easily run any example using `cargo make`. This will automatically build the WASM guest
and run the host runtime with the correct arguments.

```bash
cargo make run-example <example-name>
```

For example:

```bash
cargo make run-example blobstore
cargo make run-example http
```

Some examples require external services (like Redis or MongoDB). Please check the individual
README files for instructions on starting these services (usually via `docker compose`).

## Manual Execution

Each example includes a dedicated host runtime binary. If you prefer to run manually without `cargo make`:

```bash
# Build the guest
cargo build --example <example>-wasm --target wasm32-wasip2

# Run the host
cargo run --example <example> -- run ./target/wasm32-wasip2/debug/examples/<example>_wasm.wasm
```
