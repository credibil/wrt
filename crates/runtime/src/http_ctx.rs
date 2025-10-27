use bytes::Bytes;
use futures::Future;
use futures::FutureExt;

use http::uri;
use http_body_util::BodyExt;
use http_body_util::Full;
use http_body_util::combinators::BoxBody;
use wasmtime_wasi::TrappableError;
use wasmtime_wasi_http::body;

use wasmtime_wasi_http::p3::RequestOptions;
use wasmtime_wasi_http::p3::WasiHttpCtx;
use wasmtime_wasi_http::p3::bindings::http::types::{self, ErrorCode};

pub type HttpResult<T> = Result<T, HttpError>;
pub type HttpError = TrappableError<ErrorCode>;

pub type HeaderResult<T> = Result<T, HeaderError>;
pub type HeaderError = TrappableError<types::HeaderError>;

pub type RequestOptionsResult<T> = Result<T, RequestOptionsError>;
pub type RequestOptionsError = TrappableError<types::RequestOptionsError>;

pub struct HttpCtx;
impl WasiHttpCtx for HttpCtx {
    fn send_request(
        &mut self, request: http::Request<BoxBody<Bytes, ErrorCode>>,
        options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
    ) -> Box<
        dyn Future<
                Output = HttpResult<(
                    http::Response<BoxBody<Bytes, ErrorCode>>,
                    Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
                )>,
            > + Send,
    > {
        Box::new(async move {
            let (parts, body) = request.into_parts();
            let body_bytes = body.collect().await.unwrap().to_bytes();

            let resp = reqwest::Client::new()
                .request(parts.method, parts.uri.to_string())
                .headers(parts.headers)
                .body(body_bytes)
                .send()
                .await
                .unwrap();

            let converted: http::Response<reqwest::Body> = resp.into();
            let (parts, body) = converted.into_parts();
            let body = body.map_err(convert_error).boxed();
            let response = http::Response::from_parts(parts, body);

            Ok((response, fut))
        })
    }
}

#[allow(clippy::needless_pass_by_value)]
fn convert_error(e: reqwest::Error) -> ErrorCode {
    if e.is_timeout() {
        ErrorCode::ConnectionTimeout
    } else if e.is_connect() {
        ErrorCode::ConnectionRefused
    } else if e.is_request() {
        ErrorCode::HttpRequestUriInvalid
    // } else if e.is_body() {
    //     ErrorCode::HttpRequestBodyRead
    } else {
        ErrorCode::InternalError(Some(e.to_string()))
    }

    // reqwest::Error::from(e)
}
