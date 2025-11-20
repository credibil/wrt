# Http-with-Cache Example

This demonstrates the feature of being able to cache responses in whatever key-value store the
runtime provides (nominally Redis).

A simple HTTP server receives requests as a trigger (with examples for GET and POST). Outgoing
requests are made with `Cache-Control` and `If-None-Match` headers to demonstrate how caching is
controlled. (See [*Invoking a Cache*](http://...#invoking-a-cache) below).

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

In a console, build and run the `proxy` example:

```bash
# build the guest.
cargo build --example redis --features keyvalue,redis --target wasm32-wasip2 --release
```

Run the services in Docker containers using docker compose.

```bash
docker compose --file ./examples/redis/compose.yaml up
```

Use Postman or in a separate console, call the guest:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

This will start a wasm runtime running a simple HTTP server instrumented with logging and metrics.

Using POST should cause an error: the [If-None-Match] header is omitted to demonstrate that the 
caching implementation requires the guest to set this header alongside the [Cache-Control] header.

<a name="invoking-a-cache"></a>
## Invoking a Cache

See the example code for 

Use a [Cache-Control] header to influence the use of a pass-through cache. The following directives
are supported :

* `no-cache` - make a request to the resource and then cache the result for future requests.
  Usually used alongside `max-age` for key-value stores that support ttl.

* `no-store` - make a request to the resource and do not update the cache. This has the same
  effect as leaving out the `Cache-Control` header altogether. No other directive can be used with
  this one otherwise an error will be returned.

* `max-age=n` - try the cache first and return the result if it exists. If the record doesn't 
  exist, go to the resource then cache the result with an expiry of now plus *n* seconds (for 
  key-value stores that support ttl).

Directives are combined by separating with commas:

`Cache-Control: max-age=86400,forward=https://example.com/api/v1/records/2934875`

If caching is invoked, an [If-None-Match] header is required, otherwise an error is returned. This
is used to set the cache key. Only a single ETag is supported and weak-match ETags are not 
supported.

[Cache-Control]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control
[If-None-Match]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/If-None-Match