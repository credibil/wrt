use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tower::ServiceExt;
use wasip3::http::types::{self as wasi, ErrorCode};
use wasip3::http_compat::{http_from_wasi_request, http_into_wasi_response};

/// Serve an incoming request using the provided router.
///
/// # Errors
///
/// Returns a [`ErrorCode`] if the request could not be served.
pub async fn serve(
    router: axum::Router, request: wasi::Request,
) -> Result<wasi::Response, ErrorCode> {
    let http_req = http_from_wasi_request(request)?;
    tracing::debug!("serving request: {:?}", http_req.headers());

    // forward request to axum router to handle
    let http_resp =
        router.oneshot(http_req).await.map_err(|e| error!("issue processing request: {e}"))?;

    tracing::debug!("guest response: {http_resp:?}");
    http_into_wasi_response(http_resp)
}

/// Type alias for axum-compatible Result.
pub type Result<T, E = Error> = anyhow::Result<T, E>;

// axum error handling.
pub struct Error {
    status: StatusCode,
    error: serde_json::Value,
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: json!({"error": e.to_string()}),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.status, format!("{}", self.error)).into_response()
    }
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
