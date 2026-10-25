//! Where the server and token come from: the option wins, then the
//! variables `UC_CLI_SERVER` and `UC_CLI_TOKEN`, else `server` and `token`
//! in the `[cli]` table of uc.toml. `uc_common::settings` reads those two
//! sources into `Settings::cli`. Without a token, `uc` sends the one of its
//! sign-in to the server ([`crate::sign_in`]).

use std::path::PathBuf;

use uc_client::Client;
use uc_common::API_BASE;
use uc_common::settings::Settings;
use url::Url;

use crate::sign_in::SignIns;

const DEFAULT_SERVER: &str = "http://localhost:8080";

/// Where the token of the requests comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenSource {
    /// `--auth_token`.
    Option,
    /// `cli.token`: `UC_CLI_TOKEN`, else `token` in `[cli]`.
    Setting,
    /// The sign-in of `uc auth login` to the server.
    SignIn,
}

/// The option's value if given (none when empty), else the setting's.
fn resolve(option: Option<&str>, setting: &Option<String>) -> Option<String> {
    match option {
        Some(value) => Some(value.to_string()).filter(|v| !v.is_empty()),
        None => setting.clone().filter(|v| !v.trim().is_empty()),
    }
}

/// `--server`, else `Settings::cli.server`, else localhost:8080.
pub fn server(settings: &Settings, option: Option<&str>) -> String {
    resolve(option, &settings.cli.server).unwrap_or_else(|| DEFAULT_SERVER.to_string())
}

/// The token of `--auth_token`, else of `Settings::cli.token`, and which of
/// the two gave it.
pub fn given_token(settings: &Settings, option: Option<&str>) -> Option<(String, TokenSource)> {
    let source = match option {
        Some(_) => TokenSource::Option,
        None => TokenSource::Setting,
    };
    resolve(option, &settings.cli.token).map(|token| (token, source))
}

/// The token of the requests to `origin`: the given one ([`given_token`]),
/// else the one of the sign-in to `origin` in `sign_ins`.
pub fn token(
    settings: &Settings,
    option: Option<&str>,
    origin: &str,
    sign_ins: Option<&SignIns>,
) -> Option<(String, TokenSource)> {
    match given_token(settings, option) {
        Some(given) => Some(given),
        // `--auth_token ''` sends no token.
        None if option.is_some() => None,
        None => sign_ins?
            .token(origin)
            .map(|token| (token, TokenSource::SignIn)),
    }
}

/// The API client of the server at `url`, which sends `token`. `url` is the
/// server's root, such as `http://localhost:8080`, or the root with the
/// API's path, as the TUI shows the server.
pub fn client(url: &str, token: Option<&str>) -> anyhow::Result<Client> {
    let root = root(url)?;
    let client = match token {
        Some(token) => Client::new_with_token(root, token),
        None => Client::new_unauthenticated(root),
    };
    Ok(client.with_user_agent(concat!("uc/", env!("CARGO_PKG_VERSION"))))
}

/// The root of the server at `url`, which the API's paths start from.
fn root(url: &str) -> anyhow::Result<Url> {
    let mut root = Url::parse(url).map_err(|e| anyhow::anyhow!("invalid base URL: {e}"))?;
    let path = root.path().trim_end_matches('/');
    if let Some(above) = path.strip_suffix(API_BASE) {
        root.set_path(&format!("{above}/"));
    }
    Ok(root)
}

/// uc's folder for what it keeps between runs, such as the sign-ins and the
/// TUI's place: `uc` in `XDG_STATE_HOME`, else in `~/.local/state`, else in
/// `LOCALAPPDATA` (Windows).
pub fn state_dir() -> Option<PathBuf> {
    let var = |name: &str| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let base = var("XDG_STATE_HOME")
        .or_else(|| var("HOME").map(|home| home.join(".local").join("state")))
        .or_else(|| var("LOCALAPPDATA"))?;
    Some(base.join("uc"))
}

/// `scheme://host:port` of a `--server` value. As upstream, the path is
/// ignored and a missing port is 8080 for http and 443 for https.
pub fn origin(server: &str) -> anyhow::Result<String> {
    let (scheme, rest) = server.split_once("://").unwrap_or(("http", server));
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority.rsplit('@').next().unwrap_or_default();
    // A bracketed IPv6 host keeps its colons.
    let (host, port) = match authority.rfind(':') {
        Some(i) if !authority[i..].contains(']') => (&authority[..i], Some(&authority[i + 1..])),
        _ => (authority, None),
    };
    if host.is_empty() {
        anyhow::bail!("Invalid server URL: {server}");
    }
    let scheme = scheme.to_ascii_lowercase();
    let port = match port.filter(|p| !p.is_empty()) {
        Some(port) => port
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("Invalid server URL: {server}"))?,
        None if scheme == "https" => 443,
        None => 8080,
    };
    Ok(format!("{scheme}://{host}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_follow_upstream() {
        assert_eq!(origin("http://localhost").unwrap(), "http://localhost:8080");
        assert_eq!(
            origin("https://uc.example.com/").unwrap(),
            "https://uc.example.com:443"
        );
        assert_eq!(origin("http://h:80").unwrap(), "http://h:80");
        assert_eq!(
            origin("http://localhost:8080/api/2.1/unity-catalog/").unwrap(),
            "http://localhost:8080"
        );
        assert_eq!(origin("[::1]:9000").unwrap(), "http://[::1]:9000");
        assert_eq!(origin("localhost").unwrap(), "http://localhost:8080");
        assert!(origin("http://").is_err());
    }

    #[test]
    fn clients_call_the_root_of_the_server() {
        let at = |url| root(url).unwrap().to_string();
        assert_eq!(at("http://h:9"), "http://h:9/");
        assert_eq!(at("http://h:9/api/2.1/unity-catalog/"), "http://h:9/");
        assert_eq!(at("http://h:9/uc/api/2.1/unity-catalog"), "http://h:9/uc/");
        assert_eq!(
            root("not a url").unwrap_err().to_string(),
            "invalid base URL: relative URL without a base"
        );
        assert!(client("http://h:9", Some("t")).is_ok());
    }

    #[test]
    fn the_option_then_the_setting_then_the_sign_in() {
        let dir = tempfile::TempDir::new().unwrap();
        let sign_ins = SignIns::new(dir.path().join("tokens.json"));
        sign_ins.save("http://h:8080", "signed-in").unwrap();
        let mut settings = Settings::default();
        let to =
            |settings: &Settings, option| token(settings, option, "http://h:8080", Some(&sign_ins));

        assert_eq!(
            to(&settings, None),
            Some(("signed-in".into(), TokenSource::SignIn))
        );
        // Only the server's own sign-in.
        assert_eq!(
            token(&settings, None, "http://other:8080", Some(&sign_ins)),
            None
        );
        assert_eq!(token(&settings, None, "http://h:8080", None), None);

        settings.cli.token = Some("set".into());
        assert_eq!(
            to(&settings, None),
            Some(("set".into(), TokenSource::Setting))
        );
        assert_eq!(
            to(&settings, Some("given")),
            Some(("given".into(), TokenSource::Option))
        );
        // `--auth_token ''` sends no token.
        assert_eq!(to(&settings, Some("")), None);
        settings.cli.token = None;
        assert_eq!(to(&settings, Some("")), None);
    }
}
