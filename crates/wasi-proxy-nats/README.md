# WASI HTTP Host with Cache

This crate provides a WASI-enabled HTTP host for use in Credibil WebAssembly components. It includes a host-side cache using NATS JetStream, with control over caching using the Cache-Control header.

## Invoking a cache

Use a `Cache-Control` header to influence the use of a pass-through cache. The following directives are supported :

* `no-cache` - make a request to the resource and then cache the result for future requests.
* `no-store` - make a request to the resource and do not update the cache. This is the same effect as leaving out the `Cache-Control` header altogether.
* `max-age=n` - try the cache first and return the result if it exists. If the record doesn't exist, go to the resource then cache the result with an expiry of now plus *n* seconds.

Directives are combined by separating with commas:

`Cache-Control: max-age=86400,forward=https://example.com/api/v1/records/2934875`

There are some checks for combinations of directives that don't make sense.

See [`Cache-Control` header specifications](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control)

It is up to the guest to provide a cache key based on business logic that cannot be known by the runtime. To set a cache key, use an [`If-None-Match` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/If-None-Match). Although the specification allows for multiple Entity Tags (`etag`), this implementation only supports a single `etag`.
