# Proxy-with-Cache Example

## Quick Start

To get started add a `.env` file to this folder. See the `.env.example` file in the workspace root for a template.

In a console, build and run the `proxy` example:

```bash
# build the guest.
cargo build --example proxy --target wasm32-wasip2 --release

Run the services in Docker containers using docker compose.

```bash
docker compose --file ./examples/proxy/compose.yaml up
```

In a separate console, call the guest:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

## Invoking a cache

Use a `Cache-Control` header to influence the use of a pass-through cache. The following directives are supported :

* `no-cache` - make a request to the resource and then cache the result for future requests.
* `no-store` - make a request to the resource and do not update the cache. This is the same effect as leaving out the `Cache-Control` header altogether.
* `max-age=n` - try the cache first and return the result if it exists. If the record doesn't exist, go to the resource then cache the result with an expiry of now plus *n* seconds.
* `forward=<URI>` - the address of the upstream resource.

Directives are combined by separating with commas:

`Cache-Control: max-age=86400,forward=https://example.com/api/v1/records/2934875`

See [`Cache-Control` header specifications](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control)
