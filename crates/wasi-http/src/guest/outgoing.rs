use std::any::Any;

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http_body_util::{BodyExt, Collected};
use wasip3::http::handler;
use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

// use wasmtime_wasi_http::types::OutgoingRequestConfig;
// use crate::guest::cache::{CACHE_BUCKET, Cache};

/// Send an HTTP request using the WASI HTTP proxy handler.
///
/// # Errors
///
/// Returns an error if the request could not be sent.
pub async fn handle<T>(request: http::Request<T>) -> Result<http::Response<Collected<Bytes>>>
where
    T: http_body::Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let wasi_req =
        http_into_wasi_request(request).map_err(|e| anyhow!("Issue converting request: {e}"))?;
    let wasi_resp =
        handler::handle(wasi_req).await.map_err(|e| anyhow!("Issue calling proxy: {e}"))?;
    let http_resp = http_from_wasi_response(wasi_resp)
        .map_err(|e| anyhow!("Issue converting response: {e}"))?;

    // convert body
    let (parts, body) = http_resp.into_parts();
    let body = body.collect().await.context("failed to collect body")?;

    Ok(http::Response::from_parts(parts, body))
}

// pub struct IncomingBody(BoxBody<Bytes, anyhow::Error>);

// // caching
// let bucket = self.cache.as_deref().unwrap_or(CACHE_BUCKET);
// let mut cache = Cache::new(bucket);

// match cache.headers(&request.headers()) {
//     Ok(()) => Ok(()),
//     Err(e) => {
//         let err = format!("issue setting cache headers: {e}");
//         tracing::error!(err);
//         Err(anyhow!(err))
//     }
// }?;

// let response = if cache.should_use_cache() {
//     tracing::debug!("cache-first enabled, checking cache");

//     let fut_resp = match cache.get() {
//         Ok(Some(resp)) => {
//             tracing::debug!("response found in cache");
//             return Ok(resp);
//         }
//         Ok(None) => {
//             tracing::debug!("no cached response found, fetching from origin");
//             outgoing_handler::handle(request, None)
//                 .map_err(|e| anyhow!("making request: {e}"))?

//             // handler::handle(request).await.context("making request")?
//         }
//         Err(e) => {
//             tracing::error!("retrieving cached response: {e}, fetching from origin");
//             outgoing_handler::handle(request, None)
//                 .map_err(|e| anyhow!("making request: {e}"))?

//             // handler::handle(request).await.context("making request")?
//         }
//     };

//     Self::process_response(&fut_resp)
// } else {
//     tracing::debug!("resource-first enabled, fetching from origin");

//     let fut_resp = outgoing_handler::handle(request, None)
//         .map_err(|e| anyhow!("making request: {e}"))?;
//     Self::process_response(&fut_resp)
// }?;

// // TODO: spawn task for storing cache and return response immediately
// if cache.should_store() {
//     tracing::debug!("storing response in cache");
//     if let Err(e) = cache.put(&response) {
//         tracing::error!("storing response in cache failed: {e}");
//     }
// }
// Ok(response)
