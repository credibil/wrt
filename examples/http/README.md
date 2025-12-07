# HTTP Server Example

This example implements a simple HTTP server using `wasi-http`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Run the example

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Then run:

```bash
cargo make run-example http
```

Or manually:

```bash
# Build the WASI guest
cargo build --example http-wasm --target wasm32-wasip2

# Run
set -a && source .env && set +a
cargo run --example http -- run ./target/wasm32-wasip2/debug/examples/http_wasm.wasm
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
