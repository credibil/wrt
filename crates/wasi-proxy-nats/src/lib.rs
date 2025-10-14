//! # WASI Http Service with NATS JetStream cache.
//!
//! This module implements a runtime service for `wasi:http`
//! (<https://github.com/WebAssembly/wasi-http>). It manages caching and
//! forwarding entirely on the host.
//!
//! To control caching the following HTTP headers are supported:
//! - `Cache-Control`
//!
//! If no `Cache-Control` header is present, this host service will act exactly
//! like our standard `wasi-http` service, and just pass the request to the
//! wasm guest for handling.

use std::clone::Clone;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use async_nats::jetstream::kv::Config;
use async_nats::{AuthError, ConnectOptions, jetstream};
use futures::future::{BoxFuture, FutureExt};
use http::uri::{PathAndQuery, Uri};
// use http_body_util::{BodyExt, Full};
// use hyper::body::{Body, Bytes, Incoming};
use hyper::body::Incoming;
use hyper::header::{FORWARDED, HOST};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use runtime::RunState;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{Instrument, info_span};
use wasmtime::Store;
use wasmtime::component::{InstancePre, Linker};
use wasmtime_wasi_http::WasiHttpView;
use wasmtime_wasi_http::bindings::ProxyPre;
use wasmtime_wasi_http::bindings::http::types::Scheme;
use wasmtime_wasi_http::body::HyperOutgoingBody;
use wasmtime_wasi_http::io::TokioIo;

const DEF_HTTP_ADDR: &str = "0.0.0.0:8080";
const DEF_NATS_ADDR: &str = "nats:4222";
const DEF_NATS_BUCKET: &str = "credibil_cache";

#[derive(Debug)]
pub struct Http;

impl runtime::Service for Http {
    fn add_to_linker(&self, linker: &mut Linker<RunState>) -> Result<()> {
        wasmtime_wasi_http::add_only_http_to_linker_async(linker)
    }

    /// Provide http proxy service the specified wasm component.
    fn start(&self, pre: InstancePre<RunState>) -> BoxFuture<'static, Result<()>> {
        Self::run(pre).boxed()
    }
}

impl Http {
    /// Provide http proxy service the specified wasm component.
    async fn run(pre: InstancePre<RunState>) -> Result<()> {
        // bail if server is not required
        let component_type = pre.component().component_type();
        let mut exports = component_type.imports(pre.engine());
        if !exports.any(|e| e.0.starts_with("wasi:http")) {
            tracing::debug!("http server not required");
            return Ok(());
        }

        let addr = env::var("HTTP_ADDR").unwrap_or_else(|_| DEF_HTTP_ADDR.into());
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("http server listening on: {}", listener.local_addr()?);

        let handler = Handler {
            proxy_pre: ProxyPre::new(pre.clone())?,
            nats_client: connect_nats().await?,
        };

        // listen for requests until terminated
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let handler = handler.clone();

            tokio::spawn(async move {
                let mut http1 = http1::Builder::new();
                http1.keep_alive(true);

                if let Err(e) = http1
                    .serve_connection(
                        io,
                        service_fn(|request| {
                            handler.handle(request).instrument(info_span!("http-request"))
                        }),
                    )
                    .await
                {
                    tracing::error!("connection error: {e:?}");
                }
            });
        }
    }
}

#[derive(Debug, Default)]
struct CacheControl {
    // Store the response in cache if true.
    pub store: bool,
    // Length of time to cache the response.
    pub age: u64,
    // ETag to use as the cache key.
    pub etag: String,
}

#[derive(Clone)]
struct Handler {
    proxy_pre: ProxyPre<RunState>,
    nats_client: async_nats::Client,
}

impl Handler {
    async fn handle(&self, request: Request<Incoming>) -> Result<Response<HyperOutgoingBody>> {
        tracing::debug!("handling request: {request:?}");
        let (request, scheme) = prepare_request(request)?;

        // determine cache control (if any)
        if let Some(control) = cache_headers(request.headers())? {
            tracing::debug!("cache control: {control:?}");

            // If we have a cache age request greater than zero, we should check
            // the cache first.

            let res = self.forward_to_guest(request, scheme).await?;

            // If we got a good response and we are required to cache it, do so.
            if res.status().is_success() && control.store {
                tracing::debug!("caching response for etag: {}", control.etag);
            }

            Ok(res)
        } else {
            tracing::debug!("no cache control. Passing to guest.");
            self.forward_to_guest(request, scheme).await
        }
    }

    /// Forward a prepared request to the guest component and return its response.
    async fn forward_to_guest(
        &self, request: Request<Incoming>, scheme: Scheme,
    ) -> Result<Response<HyperOutgoingBody>> {
        let mut store = Store::new(self.proxy_pre.engine(), RunState::new());

        tracing::trace!("sending request: {request:#?}");

        let (sender, receiver) = oneshot::channel();
        let incoming = store.data_mut().new_incoming_request(scheme, request)?;
        let outgoing = store.data_mut().new_response_outparam(sender)?;

        let proxy = self.proxy_pre.instantiate_async(&mut store).await?;
        let task =
            proxy.wasi_http_incoming_handler().call_handle(&mut store, incoming, outgoing).await;

        match receiver.await {
            Ok(Ok(resp)) => {
                tracing::debug!("request success: {resp:?}");
                Ok(resp)
            }
            Ok(Err(e)) => {
                tracing::debug!("request error: {e:?}");
                Err(e.into())
            }
            Err(_) => {
                let e = match task {
                    Err(e) => e,
                    Ok(()) => anyhow!("task failed without error"),
                };
                tracing::debug!("request error: {e:?}");
                Err(anyhow!("guest did not invoke `response-outparam::set`: {e}"))
            }
        }
    }

    // /// Put a serialized response into the cache and return an unconsumed
    // /// response that can be sent back to the client.
    // async fn put_cache(
    //     &self, key: &str, response: Response<HyperOutgoingBody>, _age: Option<u64>,
    // ) -> Result<Response<HyperOutgoingBody>> {
    //     if let Some(size) = response.size_hint().upper() {
    //         // don't cache responses larger than 1 MiB
    //         if size > 1024 * 1024 {
    //             bail!("response too large to cache");
    //         }
    //     } else {
    //         // no upper bound, don't cache
    //         bail!("unable to determine response size");
    //     }
    //     let (parts, body) = response.into_parts();
    //     let body_bytes = body.collect().await?.to_bytes();
    //     let full_body = Full::<Bytes>::new(body_bytes.clone());
    //     let mapped_body = full_body.map_err(|e| match e {});
    //     let box_body = mapped_body.boxed();
    //     let fwd_response = Response::from_parts(parts, HyperOutgoingBody::from(box_body));

    //     let cache_value = serialize_response(&parts, &body_bytes)?;

    //     let kv = self.kv().await?;
    //     // kv.put(key, response.into_body().into_bytes().await?).await?;
    //     Ok(fwd_response)
    // }

    /// Get a handle to a `JetStream` key-value bucket
    async fn kv(&self) -> Result<jetstream::kv::Store> {
        let bucket_id = env::var("NATS_BUCKET").unwrap_or_else(|_| DEF_NATS_BUCKET.into());
        let js = jetstream::new(self.nats_client.clone());
        let bucket = if let Ok(bucket) = js.get_key_value(&bucket_id).await {
            bucket
        } else {
            js.create_key_value(Config {
                bucket: bucket_id.clone(),
                history: 1,
                max_age: Duration::from_secs(10 * 60), // configurable?
                max_bytes: 100 * 1024 * 1024,          // 100 MiB, configurable?
                ..Default::default()
            })
            .await
            .map_err(|e| {
                tracing::error!("failed to create nats kv bucket '{bucket_id}': {e}");
                anyhow!("failed to create nats kv bucket '{bucket_id}': {e}")
            })?
        };
        Ok(bucket)
    }
}

// Prepare the request for the guest.
// Prepare the request for the guest.
fn prepare_request(mut request: Request<Incoming>) -> Result<(Request<Incoming>, Scheme)> {
    // let req_id = self.next_id.fetch_add(1, Ordering::Relaxed);

    // rebuild Uri with scheme and authority explicitly set so they are passed to the Guest
    let uri = request.uri_mut();
    let p_and_q = uri.path_and_query().map_or_else(|| PathAndQuery::from_static("/"), Clone::clone);
    let mut uri_builder = Uri::builder().path_and_query(p_and_q);

    if let Some(forwarded) = request.headers().get(FORWARDED) {
        // running behind a proxy (that we have configured)
        for tuple in forwarded.to_str()?.split(';') {
            let tuple = tuple.trim();
            if let Some(host) = tuple.strip_prefix("host=") {
                uri_builder = uri_builder.authority(host);
            } else if let Some(proto) = tuple.strip_prefix("proto=") {
                uri_builder = uri_builder.scheme(proto);
            }
        }
    } else {
        // should be running locally
        let Some(host) = request.headers().get(HOST) else {
            return Err(anyhow!("missing host header"));
        };
        uri_builder = uri_builder.authority(host.to_str()?);
        uri_builder = uri_builder.scheme("http");
    }

    // update the uri with the new scheme and authority
    let (mut parts, body) = request.into_parts();
    parts.uri = uri_builder.build()?;
    let request = hyper::Request::from_parts(parts, body);

    let scheme = match request.uri().scheme_str() {
        Some("http") => Scheme::Http,
        Some("https") => Scheme::Https,
        _ => return Err(anyhow!("unsupported scheme")),
    };

    Ok((request, scheme))
}

/// Parse the `Cache-Control` header into a strongly typed representation.
fn cache_headers(headers: &http::HeaderMap) -> Result<Option<CacheControl>> {
    let Some(cache_header) = headers.get(http::header::CACHE_CONTROL) else {
        return Ok(None);
    };

    let raw = cache_header.to_str()?.trim();
    if raw.is_empty() {
        bail!("Cache-Control header is empty");
    }

    let mut control = CacheControl::default();
    let mut no_cache = false;
    let mut max_age = false;
    let mut no_store = false;

    for directive in raw.split(',') {
        let directive = directive.trim();
        if directive.is_empty() {
            continue;
        }

        let directive_lower = directive.to_ascii_lowercase();

        if directive_lower == "no-store" {
            if no_cache || max_age || control.store {
                return Err(anyhow!("`no-store` cannot be combined with other cache directives"));
            }
            no_store = true;
            control.store = false;
            continue;
        }

        if directive_lower == "no-cache" {
            if no_store {
                return Err(anyhow!("`no-cache` cannot be combined with `no-store`"));
            }
            if max_age {
                return Err(anyhow!("`no-cache` cannot be combined with `max-age`"));
            }
            no_cache = true;
            control.store = true;
            continue;
        }

        if directive_lower.starts_with("max-age=") {
            if no_store {
                return Err(anyhow!("`max-age` cannot be combined with `no-store`"));
            }
            if no_cache {
                return Err(anyhow!("`max-age` cannot be combined with `no-cache`"));
            }

            let seconds = directive[8..].trim();
            let age = seconds
                .parse::<u64>()
                .map_err(|err| anyhow!("invalid `max-age` value `{seconds}`: {err}"))?;
            control.store = true;
            control.age = age;
            max_age = true;
        }
        // ignore other directives
    }

    if control.store {
        let Some(etag) = headers.get(http::header::IF_NONE_MATCH) else {
            bail!(
                "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`"
            );
        };
        let etag = etag.to_str()?.trim();
        if etag.is_empty() {
            bail!("`If-None-Match` header is empty");
        }
        if etag.contains(',') {
            bail!("multiple `etag` values in `If-None-Match` header are not supported");
        }
        if etag.starts_with("W/") {
            bail!("weak `etag` values in `If-None-Match` header are not supported");
        }
        control.etag = etag.to_string();
    }

    Ok(Some(control))
}

async fn connect_nats() -> Result<async_nats::Client> {
    let addr = env::var("NATS_ADDR").unwrap_or_else(|_| DEF_NATS_ADDR.into());
    let jwt = env::var("NATS_JWT").ok();
    let seed = env::var("NATS_SEED").ok();

    let mut opts = ConnectOptions::new();
    if let Some(jwt) = jwt {
        let key_pair = nkeys::KeyPair::from_seed(&seed.unwrap_or_default())
            .map_err(|e| anyhow!("failed to create KeyPair: {e}"))?;
        let key_pair = Arc::new(key_pair);
        opts = opts.jwt(jwt, move |nonce| {
            let key_pair = Arc::clone(&key_pair);
            async move { key_pair.sign(&nonce).map_err(AuthError::new) }
        });
    }
    let client = opts.connect(addr).await.map_err(|e| {
        tracing::error!("failed to connect to nats: {e}");
        anyhow!("failed to connect to nats: {e}")
    })?;
    tracing::info!("connected to nats");

    Ok(client)
}

#[cfg(test)]
mod tests {
    use http::HeaderMap;
    use http::header::{CACHE_CONTROL, IF_NONE_MATCH};

    use super::*;

    #[test]
    fn returns_none_when_header_missing() {
        let headers = HeaderMap::new();
        let result = cache_headers(&headers).expect("parsing succeeds without header");
        assert!(result.is_none());
    }

    #[test]
    fn parses_max_age_with_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, "max-age=120".parse().expect("valid header value"));
        headers.insert(IF_NONE_MATCH, "\"strong-etag\"".parse().expect("valid etag"));

        let control =
            cache_headers(&headers).expect("parsing succeeds").expect("cache control present");

        assert!(control.store);
        assert_eq!(control.age, 120);
        assert_eq!(control.etag, "\"strong-etag\"");
    }

    #[test]
    fn requires_etag_when_store_enabled() {
        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, "no-cache".parse().expect("valid header value"));

        let Err(_) = cache_headers(&headers) else {
            panic!("expected missing etag error");
        };
    }

    #[test]
    fn rejects_conflicting_directives() {
        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, "no-cache, max-age=10".parse().expect("valid header value"));
        headers.insert(IF_NONE_MATCH, "\"etag\"".parse().expect("valid etag"));

        let Err(_) = cache_headers(&headers) else {
            panic!("expected conflicting directives error");
        };
    }

    #[test]
    fn rejects_weak_etag_value() {
        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, "no-cache".parse().expect("valid header value"));
        headers.insert(IF_NONE_MATCH, "W/\"weak-etag\"".parse().expect("valid header value"));

        let Err(_) = cache_headers(&headers) else {
            panic!("expected weak etag rejection");
        };
    }

    #[test]
    fn rejects_multiple_etag_values() {
        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, "no-cache".parse().expect("valid header value"));
        headers.insert(IF_NONE_MATCH, "\"etag1\", \"etag2\"".parse().expect("valid header value"));

        let Err(_) = cache_headers(&headers) else {
            panic!("expected multiple etag values rejection");
        };
    }
}
