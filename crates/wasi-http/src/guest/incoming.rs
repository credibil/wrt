use axum::http::StatusCode;
use axum::response::IntoResponse;
use tower::ServiceExt;
use wasip3::http::types as p3;
use wasip3::http_compat::{http_from_wasi_request, http_into_wasi_response};

/// Type alias for axum-compatible Result.
// pub type HttpResult<T> = anyhow::Result<T, HttpError>;

/// Serve an incoming request using the provided router.
///
/// # Errors
///
/// Returns a [`p3::ErrorCode`] if the request could not be served.
pub async fn serve(
    router: axum::Router, request: p3::Request,
) -> Result<p3::Response, p3::ErrorCode> {
    let http_req = http_from_wasi_request(request)?;
    tracing::debug!("serving request: {:?}", http_req.headers());

    // forward request to axum router to handle
    let http_resp =
        router.oneshot(http_req).await.map_err(|e| error!("issue processing request: {e}"))?;

    tracing::debug!("guest response: {http_resp:?}");
    http_into_wasi_response(http_resp)
}

pub struct HttpError {
    status: StatusCode,
    error: String,
}

impl From<anyhow::Error> for HttpError {
    fn from(e: anyhow::Error) -> Self {
        let error = format!("{e}, caused by: {}", e.root_cause());
        let status =
            e.downcast_ref().map_or(StatusCode::INTERNAL_SERVER_ERROR, fabric::Error::status);
        Self { status, error }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.error).into_response()
    }
}

macro_rules! error {
    ($fmt:expr, $($arg:tt)*) => {
        p3::ErrorCode::InternalError(Some(format!($fmt, $($arg)*)))
    };
     ($err:expr $(,)?) => {
        p3::ErrorCode::InternalError(Some(format!($err)))
    };
}
pub(crate) use error;
