# Redis Key Value Example

This demonstrates using Redis as a cache for HTTP responses.

A simple HTTP server receives requests as a trigger (with examples for GET and POST). Outgoing
requests are made with `Cache-Control` and `If-None-Match` headers to demonstrate how caching is
controlled. (See [*Cache Control*](#cache-control) below).

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

#### Build

```bash
cargo build --example cache-redis --features keyvalue,redis --target wasm32-wasip2 --release
```

#### Run

```bash
docker compose --file ./examples/cache-redis/compose.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## Explanation

Using POST should cause an error: the [If-None-Match] header is omitted to demonstrate that the 
caching implementation requires the guest to set this header alongside the [Cache-Control] header.

## Cache Control

Use the [Cache-Control] header to influence the use of a pass-through cache. The following directives
are currently supported:

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

> [!IMPORTANT]
> Currently, the [Cache-Control] header requires a corresponding [If-None-Match] header with a 
> single `<etag_value>` to use as the cache key. 

[Cache-Control]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control
[If-None-Match]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/If-None-Match