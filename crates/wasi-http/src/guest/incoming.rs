use anyhow::Result;
use tower::ServiceExt;
use wasip3::http::types::{self as wasi, ErrorCode};
use wasip3::http_compat::{http_from_wasi_request, http_into_wasi_response};

/// Serve an incoming request using the provided router.
///
/// # Errors
///
/// Returns a [`wasi::http::types::ErrorCode`] if the request could not be served.
pub async fn serve(
    router: axum::Router, request: wasi::Request,
) -> Result<wasi::Response, ErrorCode> {
    let http_req = http_from_wasi_request(request)?;
    tracing::info!("serving request: {:?}", http_req.headers());

    // forward request to router to handle
    let http_resp =
        router.oneshot(http_req).await.map_err(|e| error!("issue processing request: {e}"))?;
    tracing::info!("guest response: {http_resp:?}");

    let response = http_into_wasi_response(http_resp)?;
    Ok(response)
}

macro_rules! error {
    ($fmt:expr, $($arg:tt)*) => {
        ErrorCode::InternalError(Some(format!($fmt, $($arg)*)))
    };
     ($err:expr $(,)?) => {
        ErrorCode::InternalError(Some(format!($err)))
    };
}
pub(crate) use error;
