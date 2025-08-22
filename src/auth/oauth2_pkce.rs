use std::{collections::HashMap, fmt::Display, str::FromStr, sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::StdRng, Rng, SeedableRng};
use reqwest::{Client, Url};
use sha2::Digest;
use tokio::sync::mpsc::channel;
use tokio_util::sync::CancellationToken;
use anyhow::{bail, Result, anyhow};
use warp::{http::Response, Filter};
use std::net::SocketAddr;

use crate::{auth::{AuthResult, ServiceAuthenticator, TokenResult}, util::{build_query_string, open_link}};

const CODE_VERIFIER_LEN: usize = 64;
const STATE_LEN: usize = 5;

pub enum CodeChallengeMethod {
    Sha256,
    Plain
}

impl Display for CodeChallengeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Self::Sha256 => "S256",
            Self::Plain => "plain"
        };
        write!(f, "{}", repr)
    }
}

pub struct OAuth2PKCEAuthenticator {
    http_client: Client,
    client_id: String,
    scope: String,
    auth_url: Url,
    redirect_url: Url,
    challenge_method: CodeChallengeMethod
}

type QueryParams = HashMap<String, String>;

impl OAuth2PKCEAuthenticator {
    pub fn new(client_id: impl Into<String>, scope: impl Into<String>, auth_url: Url, redirect_url: Url, method: CodeChallengeMethod) -> Self {
        Self {
            http_client: Client::new(),
            client_id: client_id.into(),
            scope: scope.into(),
            auth_url,
            redirect_url,
            challenge_method: method
        }
    }

    fn generate_verifier(&self, code_len: usize) -> String {
        static CODE_CHARS: &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let rnd = StdRng::from_os_rng();
        
        rnd
            .random_iter()
            .take(code_len)
            .map(|x: u32| CODE_CHARS.chars().nth(x as usize % CODE_CHARS.len()).unwrap())
            .collect()
    }

    fn generate_challenge(&self, verifier: &str) -> String {
        URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(&verifier))
    }

    async fn receive_auth_code(&self, state: &str, cancel_token: CancellationToken) -> Result<String> {
        let bind_host = self.redirect_url.authority();
        let bind_path = self.redirect_url.path();
        let state = Arc::new(state.to_owned());

        // Channel for propagating the auth codes and errors
        let (tx, mut rx) = channel::<Result<String>>(1);

        let kill_server_token = cancel_token.child_token();
        let auth_res_handler = warp::get()
            .and(warp::path(bind_path.to_owned()))
            .and(warp::query::<QueryParams>())
            .map({
                let kill_server_token = kill_server_token.clone();
                move |mut params: QueryParams| {
                    let res = if params.remove("state").unwrap_or_default() != *state {
                        "state missing or invalid".to_owned()
                    } else if let Some(error) = params.remove("error") {
                        error.to_owned()
                    } else {
                        if let Some(code) = params.remove("code") {
                            let _ = tx.send(Ok(code));
                            kill_server_token.cancel();
                            return Response::builder().status(200).body("Success".into()).unwrap();
                        }
                        "'code' or 'error' missing".into()
                    };
                    let _ = tx.send(Err(anyhow!(res.clone())));
                    kill_server_token.cancel();
                    return Response::builder().status(500).body(res).unwrap();
                }
            });
        
        warp::serve(auth_res_handler)
            .bind(SocketAddr::from_str(bind_host)?)
            .await
            .graceful(async move {
                kill_server_token.cancelled().await
            })
            .run().await;

        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                bail!("Auth cancelled");
            },
            res = rx.recv() => {
                return match res {
                    Some(res) => res,
                    None => Err(anyhow!("BUG: Receiver closed before response was received"))
                };
            }
        }
    }

    async fn send_token_req(&self, body: &HashMap<&str, &str>) -> Result<TokenResult> {
        let res = self
            .http_client
            .post(self.auth_url.clone().join("api/token")?)
            .form(&body)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?;
        let res = res.json::<TokenResult>().await?;
        Ok(res)
    }
}

#[async_trait]
impl ServiceAuthenticator for OAuth2PKCEAuthenticator {
    async fn authenticate(&mut self, cancel: CancellationToken) -> Result<AuthResult> {
        let code_verifier = self.generate_verifier(CODE_VERIFIER_LEN);
        let challenge = match self.challenge_method {
            CodeChallengeMethod::Sha256 => &self.generate_challenge(&code_verifier),
            CodeChallengeMethod::Plain => &code_verifier
        };
        let state = self.generate_verifier(STATE_LEN);

        // Build auth url and redirect the user to for authorization
        let query = build_query_string([
            ("client_id", self.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", self.redirect_url.as_str()),
            ("state", &state),
            ("scope", &self.scope),
            ("code_challenge_method", &self.challenge_method.to_string()),
            ("code_challenge", &challenge)
        ]);
        let mut auth_url = self.auth_url.clone().join("authorize")?;
        auth_url.set_query(Some(&query));
        open_link(&auth_url);

        let auth_code = self.receive_auth_code(&state, cancel.clone()).await?;

        // Request access token
        let body: HashMap<&str, &str> = [
            ("grant_type", "authorization_code"),
            ("code", &auth_code),
            ("redirect_uri", self.redirect_url.as_str()),
            ("client_id", &self.client_id),
            ("code_verifier", &code_verifier)
        ].into_iter().collect();
        let res = self.send_token_req(&body).await?;
        Ok(AuthResult::Token(res))
    }

    async fn refresh(&mut self, refresh_token: &str) -> Result<AuthResult> {
        let body = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id)
        ].into_iter().collect();
        let res = self.send_token_req(&body).await?;
        Ok(AuthResult::Token(res))
    }
}
