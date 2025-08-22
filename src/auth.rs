use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use anyhow::Result;

mod oauth2_pkce;

#[async_trait]
pub trait ServiceAuthenticator {
    async fn authenticate(&mut self, cancel: CancellationToken) -> Result<AuthResult>;
    async fn refresh(&mut self, refresh_token: &str) -> Result<AuthResult>;
}

#[derive(Deserialize)]
pub struct TokenResult {
    access_token: String,
    refresh_token: String,
    expires_in: u64
}

pub enum AuthResult {
    Token(TokenResult)
}
