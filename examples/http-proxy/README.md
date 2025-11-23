# HTTP Proxy Example

This example implements a simple HTTP server using `wasi-http`. It uses a `wasi-keyvalue` store 
(Redis or NATS) to cache responses for proxied HTTP requests.

HTTP GET and POST requests are made to the http service which subsequently makes outgoing requests 
using `Cache-Control` and `If-None-Match` headers. 

See [*Cache Control*](#cache-control) below.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example http-proxy --target wasm32-wasip2 --release
```

### Run using Cargo

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./examples/docker/opentelemetry.yaml up
```

Run the guest using either NATS or Redis:

```bash
set -a && source .env && set +a

# with NATS
cargo run --features http,otel,keyvalue,nats -- run ./target/wasm32-wasip2/release/examples/http_proxy.wasm

# with Redis
cargo run --features http,otel,keyvalue,redis -- run ./target/wasm32-wasip2/release/examples/http_proxy.wasm
```

### Run using Docker Compose

Docker Compose provides an easy way to run the example with all dependencies.

```bash
# with NATS
docker compose --file ./examples/http/nats.yaml up

# with Redis
docker compose --file ./examples/http/redis.yaml up
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Implementing Caching

Use the [Cache-Control] header to influence the use of a pass-through cache. The following 
directives are currently supported:

* `no-cache` - make a request to the resource and then cache the result for future requests.
  Usually used alongside `max-age` for key-value stores that support ttl.

* `no-store` - make a request to the resource and do not update the cache. This has the same
  effect as leaving out the `Cache-Control` header altogether. No other directive can be used with
  this one otherwise an error will be returned.

* `max-age=n` - try the cache first and return the result if it exists. If the record doesn't 
  exist, go to the resource then cache the result with an expiry of now plus *n* seconds (for 
  key-value stores that support ttl).

Multiple directives can be combined in a comma-delimited list:

```
Cache-Control: max-age=86400,forward=https://example.com/api/v1/records/2934875
```

> [!WARNING]
> Currently, the [Cache-Control] header requires a corresponding [If-None-Match] header with a 
> single `<etag_value>` to use as the cache key.

In the example guest an HTTP POST will cause an error: the [If-None-Match] header has been omitted
to demonstrate that the caching implementation requires the guest to set this header alongside the
[Cache-Control] header.

[Cache-Control]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control
[If-None-Match]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/If-None-Match