# Blobstore Example

This example implements a simple blobstore using `wasi-blobstore`.

## Quick Start

This example uses the default implementation of `wasi-blobstore`.

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Run the example

```bash
cargo make run-example blobstore
```

Or manually:

```bash
# Build the guest
cargo build --example blobstore-wasm --target wasm32-wasip2

# Run the guest
set -a && source .env && set +a
cargo run --example blobstore -- run ./target/wasm32-wasip2/debug/examples/blobstore_wasm.wasm
```

### Test the guest

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
