use axum::response::{IntoResponse, Response};
use http::StatusCode;

pub struct ApiError(pub credibil_error::Error);

impl From<credibil_error::Error> for ApiError {
    fn from(err: credibil_error::Error) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.code() as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, self.0.description().to_string()).into_response()
    }
}
