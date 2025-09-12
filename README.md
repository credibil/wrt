# Tempo Wasm Runtime

Tempo wraps [`wasmtime`](https://github.com/bytecodealliance/wasmtime) to provide Credibil with a simple, ergonomic runtime for WASI components.

We consider this a stop-gap solution until production-grade runtimes support dynamic inclusion of host-based services.

## Example Runtimes

There are a number of examples provided in the `examples` directory and a Docker compose file that can be used to run them.

To get started, add a `.env` file to the [examples/runtimes](./examples/runtimes) directory (see `.env.example`) and run:

```bash
docker compose --file ./examples/runtimes/compose.yaml up
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

## Example Guests

Example guests can be found in the [examples](./examples) directory. Instructions for building and running each example can be found in the respective README files.