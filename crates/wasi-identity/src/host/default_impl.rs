use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use fromenv::FromEnv;
use futures::FutureExt;
use futures::lock::Mutex;
use oauth2::basic::{BasicClient, BasicTokenType};
use oauth2::reqwest::{self, redirect};
use oauth2::{
    ClientId, ClientSecret, EmptyExtraTokenFields, Scope, StandardTokenResponse, TokenResponse,
    TokenUrl,
};
use runtime::Resource;
use tracing::instrument;

use crate::host::WasiIdentityCtx;
pub use crate::host::generated::wasi::identity::credentials::{AccessToken, Datetime};
use crate::host::resource::{FutureResult, Identity};

// "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token"

#[derive(Debug, Clone, FromEnv)]
pub struct ConnectOptions {
    #[env(from = "IDENTITY_CLIENT_ID")]
    pub client_id: String,
    #[env(from = "IDENTITY_CLIENT_SECRET")]
    pub client_secret: String,
    #[env(from = "IDENTITY_TOKEN_URL")]
    pub token_url: String,
}

impl runtime::FromEnv for ConnectOptions {
    fn from_env() -> Result<Self> {
        Self::from_env().finalize().map_err(|e| anyhow!("issue loading connection options: {e}"))
    }
}

#[derive(Debug, Clone)]
pub struct DefaultIdentityCtx {
    token_manager: TokenManager,
}

impl Resource for DefaultIdentityCtx {
    type ConnectOptions = ConnectOptions;

    #[instrument]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        let token_manager = TokenManager::new(options);
        Ok(Self { token_manager })
    }
}

impl WasiIdentityCtx for DefaultIdentityCtx {
    fn get_identity(&self, _name: String) -> FutureResult<Arc<dyn Identity>> {
        let token_manager = self.token_manager.clone();

        async move { Ok(Arc::new(token_manager) as Arc<dyn Identity>) }.boxed()
    }
}

#[derive(Debug, Clone)]
struct TokenManager {
    options: Arc<ConnectOptions>,
    token: Arc<Mutex<CachedToken>>,
}

#[derive(Debug, Clone)]
struct CachedToken(AccessToken);

impl CachedToken {
    const fn new(token: AccessToken) -> Self {
        Self(token)
    }

    fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp().cast_unsigned();
        self.0.expiration.seconds <= now
    }
}

impl Default for CachedToken {
    fn default() -> Self {
        Self(AccessToken {
            token: String::new(),
            expiration: Datetime {
                seconds: 0,
                nanoseconds: 0,
            },
        })
    }
}

impl From<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for CachedToken {
    fn from(token_resp: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
        let token = token_resp.access_token().secret().clone();
        let expires_in = token_resp.expires_in().unwrap_or(Duration::from_secs(3600));
        let expires = Utc::now()
            + chrono::Duration::from_std(expires_in)
                .unwrap_or_else(|_| chrono::Duration::seconds(3600));

        Self::new(AccessToken {
            token,
            expiration: Datetime {
                seconds: expires.timestamp().cast_unsigned(),
                nanoseconds: 0,
            },
        })
    }
}

impl Identity for TokenManager {
    fn get_token(&self, scopes: Vec<String>) -> FutureResult<AccessToken> {
        let token_manager = self.clone();
        async move { token_manager.token(&scopes).await }.boxed()
    }
}

impl TokenManager {
    fn new(options: ConnectOptions) -> Self {
        Self {
            options: Arc::new(options),
            token: Arc::new(Mutex::new(CachedToken::default())),
        }
    }

    async fn token(&self, scopes: &[String]) -> Result<AccessToken> {
        // use cached token
        let cached_token = self.token.lock().await;
        if !cached_token.is_expired() {
            return Ok(cached_token.0.clone());
        }

        // fetch actual token
        let oauth2_client = BasicClient::new(ClientId::new(self.options.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.options.client_secret.clone()))
            .set_token_uri(TokenUrl::new(self.options.token_url.clone())?);

        let http_client =
            reqwest::ClientBuilder::new().redirect(redirect::Policy::none()).build()?;

        let mut token_req = oauth2_client.exchange_client_credentials();
        for scope in scopes {
            token_req = token_req.add_scope(Scope::new(scope.clone()));
        }
        let token_resp = token_req.request_async(&http_client).await?;

        // cache new token
        let cached_token = CachedToken::from(token_resp);
        self.token.lock().await.clone_from(&cached_token);

        Ok(cached_token.0)
    }
}
