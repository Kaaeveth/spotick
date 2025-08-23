use std::{collections::HashMap, fmt::Display, net::{Ipv4Addr, SocketAddrV4}, sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::StdRng, Rng, SeedableRng};
use reqwest::{Client, Url};
use sha2::Digest;
use tokio::sync::{mpsc::channel};
use tokio_util::sync::CancellationToken;
use anyhow::{bail, Result, anyhow};
use warp::{http::Response, Filter};

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

    async fn receive_auth_code(&self, state: String, cancel_token: CancellationToken) -> Result<String> {
        // Channel for propagating the auth codes and errors
        let (tx, mut rx) = channel::<Result<String>>(1);

        let auth_res_handler = warp::get()
            .and(warp::path::end())
            .and(warp::query::<QueryParams>())
            .then({
                let state = Arc::new(state);
                move |mut params: QueryParams| {
                    let tx = tx.clone();
                    let state = state.clone();
                    async move {
                        let res = if params.remove("state").unwrap_or_default() != *state {
                            "state missing or invalid".to_owned()
                        } else if let Some(error) = params.remove("error") {
                            error.to_owned()
                        } else {
                            if let Some(code) = params.remove("code") {
                                let _ = tx.send(Ok(code)).await;
                                return Response::builder().status(200).body("Success".into()).unwrap();
                            }
                            "'code' or 'error' missing".into()
                        };
                        let _ = tx.send(Err(anyhow!(res.clone())));
                        return Response::builder().status(500).body(res).unwrap();
                    }
                }
            });
        
        let kill_server_token = cancel_token.child_token();
        let server_task = tokio::spawn({
            let kill_server_token = kill_server_token.clone();
            let port = self.redirect_url.port().unwrap_or(80);
            async move {
                let socket_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
                warp::serve(auth_res_handler)
                    .bind(socket_addr)
                    .await
                    .graceful({
                        async move {
                            kill_server_token.cancelled().await
                        }
                    })
                    .run()
                    .await;
            }
        });

        tokio::select! {
            biased;
            res = rx.recv() => {
                kill_server_token.cancel(); // Shutdown HTTP server
                return match res {
                    Some(res) => res,
                    None => Err(anyhow!("BUG: Receiver closed before response was received"))
                };
            },
            _ = cancel_token.cancelled() => {
                bail!("Auth cancelled");
            },
            _ = server_task => {
                bail!("BUG: HTTP Server shutdown unexpectedly");
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

        let auth_code = self.receive_auth_code(state, cancel.clone()).await?;

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

#[cfg(test)]
mod test {
    use anyhow::{ensure, Context};
    use tokio::sync::oneshot::channel;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn receive_correct_auth_code() -> Result<()> {
        let correct_auth_code = "6969";
        let redirect_url =  Url::parse("http://localhost:14588")?;
        let authenticator = OAuth2PKCEAuthenticator::new(
            "c1337",
            "user-read-private user-read-email",
            Url::parse("https://accounts.spotify.com/")?,
            redirect_url.clone(),
            CodeChallengeMethod::Sha256
        );
        let http_client = reqwest::Client::new();
        let state = authenticator.generate_verifier(CODE_VERIFIER_LEN);
        let cancel_token = CancellationToken::new();
        let (tx, rx) = channel::<Result<String>>();

        let auth_task = tokio::spawn({
            let state = state.clone();
            let cancel_token = cancel_token.clone();
            async move {
                let res = authenticator.receive_auth_code(state, cancel_token).await;
                let _ = tx.send(res);
            }
        });

        let send_auth_code_res = http_client
            .get(redirect_url)
            .query(&[("code", correct_auth_code), ("state", &state)])
            .timeout(Duration::from_secs(3))
            .send()
            .await.context("Timeout sending auth code")?
            .error_for_status().context("Error sending auth code")?;
        ensure!(send_auth_code_res.text().await?.as_str() == "Success", "Wrong response body received");

        let received_auth_code = rx
            .await
            .context("Error receiving auth code")?
            .context("Received error instead of auth code")?;
        ensure!(received_auth_code == correct_auth_code, "Wrong auth code received");
        
        auth_task.await?;

        Ok(())
    }
}
