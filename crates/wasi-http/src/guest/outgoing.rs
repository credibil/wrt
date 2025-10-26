use std::any::Any;

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http_body_util::BodyExt;
use wasip3::http::handler;
use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

// use wasmtime_wasi_http::types::OutgoingRequestConfig;
use crate::guest::cache::Cache;

/// Send an HTTP request using the WASI HTTP proxy handler.
///
/// # Errors
///
/// Returns an error if the request could not be sent.
pub async fn handle<T>(request: http::Request<T>) -> Result<http::Response<Bytes>>
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let maybe_cache = Cache::maybe_from(&request)?;

    // check cache when indicated by request
    if let Some(cache) = maybe_cache.as_ref()
        && let Some(hit) = cache.maybe_get()?
    {
        tracing::debug!("cache hit");
        return Ok(hit);
    }

    let wasi_req =
        http_into_wasi_request(request).map_err(|e| anyhow!("Issue converting request: {e}"))?;
    let wasi_resp =
        handler::handle(wasi_req).await.map_err(|e| anyhow!("Issue calling proxy: {e}"))?;
    let http_resp = http_from_wasi_response(wasi_resp)
        .map_err(|e| anyhow!("Issue converting response: {e}"))?;

    // convert body
    let (parts, body) = http_resp.into_parts();
    let collected = body.collect().await.context("failed to collect body")?;
    let bytes = collected.to_bytes();
    let response = http::Response::from_parts(parts, bytes);

    // cache response when indicated by request
    if let Some(cache) = maybe_cache {
        cache.maybe_put(&response)?;
        tracing::debug!("response cached");
    }

    Ok(response)
}

// pub struct IncomingBody(BoxBody<Bytes, anyhow::Error>);
