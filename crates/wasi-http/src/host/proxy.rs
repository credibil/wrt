use base64ct::{Base64, Encoding};
use bytes::Bytes;
use futures::Future;
use http_body_util::BodyExt;
use http_body_util::combinators::BoxBody;
use wasmtime_wasi::TrappableError;
use wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::p3::{self, RequestOptions};

pub type HttpResult<T> = Result<T, HttpError>;
pub type HttpError = TrappableError<ErrorCode>;

// pub type HeaderResult<T> = Result<T, HeaderError>;
// pub type HeaderError = TrappableError<types::HeaderError>;
// pub type RequestOptionsResult<T> = Result<T, RequestOptionsError>;
// pub type RequestOptionsError = TrappableError<types::RequestOptionsError>;

pub struct WasiHttpCtx;

impl p3::WasiHttpCtx for WasiHttpCtx {
    fn send_request(
        &mut self, request: http::Request<BoxBody<Bytes, ErrorCode>>,
        _options: Option<RequestOptions>,
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
            let (mut parts, body) = request.into_parts();
            let collected =
                body.collect().await.map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;

            // build reqwest::Request
            let mut builder = reqwest::Client::builder();

            // check for client certificate in headers
            if let Some(encoded_cert) = parts.headers.remove("Client-Cert") {
                tracing::debug!("using client certificate");
                let encoded_str = encoded_cert
                    .to_str()
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                let pem_bytes = Base64::decode_vec(encoded_str)
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                let identity = reqwest::Identity::from_pem(&pem_bytes)
                    .map_err(|e| ErrorCode::InternalError(Some(e.to_string())))?;
                builder = builder.use_rustls_tls().identity(identity);
            }

            let client = builder.build().map_err(into_error)?;
            let resp = client
                .request(parts.method, parts.uri.to_string())
                .headers(parts.headers)
                .body(collected.to_bytes())
                .send()
                .await
                .map_err(into_error)?;

            let converted: http::Response<reqwest::Body> = resp.into();
            let (parts, body) = converted.into_parts();
            let body = body.map_err(into_error).boxed();
            let response = http::Response::from_parts(parts, body);

            Ok((response, fut))
        })
    }
}

#[allow(clippy::needless_pass_by_value)]
fn into_error(e: reqwest::Error) -> ErrorCode {
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
}
