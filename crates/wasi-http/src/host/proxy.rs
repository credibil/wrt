use anyhow::{Result, anyhow};
use base64ct::{Base64, Encoding};
use bytes::Bytes;
use fromenv::FromEnv;
use futures::Future;
use http_body_util::BodyExt;
use http_body_util::combinators::UnsyncBoxBody;
use kernel::Backend;
use tracing::instrument;
use wasmtime_wasi::TrappableError;
use wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::p3::{self, RequestOptions};

pub type HttpResult<T> = Result<T, HttpError>;
pub type HttpError = TrappableError<ErrorCode>;

// pub type HeaderResult<T> = Result<T, HeaderError>;
// pub type HeaderError = TrappableError<types::HeaderError>;
// pub type RequestOptionsResult<T> = Result<T, RequestOptionsError>;
// pub type RequestOptionsError = TrappableError<types::RequestOptionsError>;

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "HTTP_ADDR", default = "http://localhost:8080")]
    pub addr: String,
}

impl kernel::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

#[derive(Debug, Clone)]
pub struct DefaultHttp;

impl Backend for DefaultHttp {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        Ok(Self)
    }
}

impl p3::WasiHttpCtx for DefaultHttp {
    fn send_request(
        &mut self, request: http::Request<UnsyncBoxBody<Bytes, ErrorCode>>,
        _options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), ErrorCode>> + Send>,
    ) -> Box<
        dyn Future<
                Output = HttpResult<(
                    http::Response<UnsyncBoxBody<Bytes, ErrorCode>>,
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
            let body = body.map_err(into_error).boxed_unsync();
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

// use std::env;

// use azure_core::credentials::TokenCredential;
// use azure_identity::{ManagedIdentityCredential, ManagedIdentityCredentialOptions, UserAssignedId};

// async fn access_token() -> Result<String, anyhow::Error> {
//     let user_assigned_id = env::var("AZURE_IDENTITY").ok().map(UserAssignedId::ClientId);
//     let options = ManagedIdentityCredentialOptions { user_assigned_id, ..Default::default() };
//     let credential = ManagedIdentityCredential::new(Some(options))?;

//     let token = {
//         let access_token = credential.get_token(&[], None).await?;
//         let token = access_token.token.secret().to_string();
//         token
//     };
//     Ok(token)
// }
