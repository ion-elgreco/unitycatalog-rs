//! `auth login`, as upstream's `AuthCli` and `Oauth2CliExchange`: an
//! identity token, given or got in the browser, is exchanged at the
//! control API's `/auth/tokens` for a Unity Catalog access token. Unlike
//! upstream, uc keeps the token as its sign-in to the server
//! ([`crate::sign_in`]), which `auth logout` removes.

use std::collections::HashMap;

use anyhow::{anyhow, bail};
use clap::Subcommand;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uc_client::error::Error;
use uc_client::models::auth::v1::OAuthTokenExchangeInfo;
use uc_common::settings::{self, Cli, Settings};

use super::{CallError, Context, error_body};
use crate::browser;
use crate::config;
use crate::sign_in::SignIns;

const TOKEN_EXCHANGE: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";
const ID_TOKEN: &str = "urn:ietf:params:oauth:token-type:id_token";

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum AuthOp {
    /// Sign in to the server; the commands after it use the sign-in
    Login {
        /// The identity token; without it, sign in in the browser
        #[arg(long)]
        identity_token: Option<String>,
    },
    /// Remove the sign-in to the server
    Logout,
}

/// Run `op` for the server at `origin`.
pub async fn run(
    c: &Context<'_>,
    op: AuthOp,
    settings: &Settings,
    origin: &str,
) -> anyhow::Result<String> {
    let sign_ins = SignIns::of_user();
    // Later commands send the token of `cli.token`, not the sign-in; that
    // of `--auth_token` is only for this one.
    let setting = config::given_token(settings, None).map(|_| settings::name("cli.token"));
    match op {
        AuthOp::Login { identity_token } => {
            let response = login(c, identity_token, &settings.cli).await?;
            let token = Some(response.access_token.as_str())
                .filter(|token| !token.is_empty())
                .ok_or_else(|| anyhow!("The server's answer has no access_token"))?;
            match sign_ins.map(|sign_ins| sign_ins.save(origin, token)) {
                Some(Ok(())) => match expiry(&response) {
                    Some(until) => eprintln!("Signed in to {origin} until {until}."),
                    None => eprintln!("Signed in to {origin}."),
                },
                Some(Err(e)) => eprintln!("uc cannot keep the sign-in: {e:#}"),
                None => eprintln!("uc cannot keep the sign-in: set XDG_STATE_HOME or HOME."),
            }
            if let Some(setting) = setting {
                eprintln!("While {setting} is set, uc sends its token, not this sign-in.");
            }
            Ok(c.json(&json!({ "access_token": token })))
        }
        AuthOp::Logout => {
            let removed = match &sign_ins {
                Some(sign_ins) => sign_ins.remove(origin)?,
                None => false,
            };
            match removed {
                true => eprintln!("Signed out of {origin}."),
                false => eprintln!("Not signed in to {origin}."),
            }
            if let Some(setting) = setting {
                eprintln!("While {setting} is set, uc still sends its token.");
            }
            Ok(String::new())
        }
    }
}

/// When the token of the exchange's `response` expires, in local time; none
/// when the answer does not say.
fn expiry(response: &OAuthTokenExchangeInfo) -> Option<String> {
    let seconds = Some(response.expires_in).filter(|&seconds| seconds != 0)?;
    let life = chrono::TimeDelta::try_seconds(seconds)?;
    let until = chrono::Local::now().checked_add_signed(life)?;
    Some(until.format("%Y-%m-%d %H:%M").to_string())
}

/// The server's answer to the token exchange.
async fn login(
    c: &Context<'_>,
    identity_token: Option<String>,
    settings: &Cli,
) -> anyhow::Result<OAuthTokenExchangeInfo> {
    let identity_token = match identity_token {
        Some(token) => token,
        None => browser_login(settings).await?,
    };
    match c
        .client
        .exchange_token(TOKEN_EXCHANGE, ACCESS_TOKEN, identity_token, ID_TOKEN)
        .await
    {
        Ok(response) => Ok(response),
        Err(Error::Api(api)) => bail!("Error authenticating - {}", error_body(&api)),
        Err(e) => Err(CallError(e).into()),
    }
}

/// The OAuth authorization-code flow in the browser, with the provider and
/// client of the `cli.oauth_*` settings; the identity token. Its messages go
/// to standard error: standard output has only the result.
async fn browser_login(settings: &Cli) -> anyhow::Result<String> {
    let required = |value: &Option<String>, key: &str| {
        value
            .clone()
            .ok_or_else(|| anyhow!("{} is not set", settings::name(key)))
    };
    let authorization_url = required(
        &settings.oauth_authorization_url,
        "cli.oauth_authorization_url",
    )?;
    let token_url = required(&settings.oauth_token_url, "cli.oauth_token_url")?;
    let client_id = settings.oauth_client_id.clone().unwrap_or_default();
    let client_secret = settings.oauth_client_secret.clone().unwrap_or_default();
    let port = settings.oauth_redirect_port.unwrap_or(0);
    // The browser on this machine only (RFC 8252, section 7.3).
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let port = listener.local_addr()?.port();
    eprintln!("Listening on port: {port}");
    let redirect_url = format!("http://localhost:{port}");
    let state = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_url)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid profile email")
        .append_pair("state", &state)
        .finish();
    let auth_url = format!("{authorization_url}?{query}");
    eprintln!("Attempting to open the authorization page in your default browser.");
    eprintln!("If the browser does not open, you can manually open the following URL:");
    eprintln!();
    eprintln!("{auth_url}");
    eprintln!();
    browser::open(&auth_url);

    let code = loop {
        let (mut socket, _) = listener.accept().await?;
        let mut buffer = vec![0u8; 16 * 1024];
        let read = socket.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..read]);
        let target = request.split_whitespace().nth(1).unwrap_or_default();
        let params: HashMap<String, String> = target
            .split_once('?')
            .map(|(_, q)| {
                url::form_urlencoded::parse(q.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_default();
        // Only the answer to this sign-in, with its state: not a browser's
        // other request, such as a favicon, nor a code that another page
        // sends to this port.
        let code = match (params.get("code"), params.get("state")) {
            (Some(code), Some(given)) if *given == state => code.clone(),
            _ => {
                let _ = socket
                    .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n")
                    .await;
                continue;
            }
        };
        let reply = "User validated with identity provider.";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{reply}",
            reply.len()
        );
        socket.write_all(response.as_bytes()).await?;
        break code;
    };
    eprintln!("Received authentication response.");

    let response = reqwest::Client::new()
        .post(&token_url)
        .basic_auth(&client_id, Some(&client_secret))
        .header("accept", "application/json")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_url.as_str()),
        ])
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("Error authenticating - {body}");
    }
    eprintln!("Received token response.");
    let tokens: Value = serde_json::from_str(&body)?;
    tokens["id_token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("The token response has no id_token"))
}
