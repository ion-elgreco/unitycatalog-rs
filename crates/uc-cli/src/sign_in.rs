//! The sign-ins of `uc auth login`: the access token that each server gave,
//! by the server's origin, in `tokens.json` in uc's state folder
//! ([`config::state_dir`]). Only the user can read the file. The commands,
//! the TUI and `uc web` send the token of their server when no token is
//! given ([`config::token`]); `uc auth logout` removes it.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Context as _;
use uc_common::settings;

use crate::args::Global;
use crate::commands::shell_word;
use crate::config::{self, TokenSource};

/// The file of the sign-ins.
pub struct SignIns {
    path: PathBuf,
}

impl SignIns {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The user's: `tokens.json` in [`config::state_dir`]; none without a
    /// state folder.
    pub fn of_user() -> Option<Self> {
        config::state_dir().map(|dir| Self::new(dir.join("tokens.json")))
    }

    /// The token of the sign-in to `origin`.
    pub fn token(&self, origin: &str) -> Option<String> {
        self.read().remove(origin)
    }

    /// Keep `token` as the sign-in to `origin`, in place of an earlier one.
    pub fn save(&self, origin: &str, token: &str) -> anyhow::Result<()> {
        let mut tokens = self.read();
        tokens.insert(origin.to_string(), token.to_string());
        self.write(&tokens)
    }

    /// Remove the sign-in to `origin`; whether there was one.
    pub fn remove(&self, origin: &str) -> anyhow::Result<bool> {
        let mut tokens = self.read();
        if tokens.remove(origin).is_none() {
            return Ok(false);
        }
        self.write(&tokens)?;
        Ok(true)
    }

    /// The tokens by origin; none when the file is missing or not valid.
    fn read(&self) -> BTreeMap<String, String> {
        std::fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Write `tokens` to a new file that only the user can read, then put
    /// it in place of the old one: a reader never gets half a file.
    fn write(&self, tokens: &BTreeMap<String, String>) -> anyhow::Result<()> {
        let failed = || format!("Cannot write {}", self.path.display());
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).with_context(failed)?;
        }
        let new = self.path.with_extension("json.new");
        // A new file gets the mode; an old one would keep its own.
        let _ = std::fs::remove_file(&new);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&new).with_context(failed)?;
        file.write_all(&serde_json::to_vec_pretty(tokens)?)
            .with_context(failed)?;
        std::fs::rename(&new, &self.path).with_context(failed)
    }
}

/// What a 401 from `origin` means for the token of `source` (none: no
/// token was sent), and how to sign in: `login` is [`login_command`].
pub fn advice(origin: &str, source: Option<TokenSource>, login: &str) -> String {
    match source {
        None => format!("{origin} needs a sign-in. Sign in with: {login}"),
        Some(TokenSource::SignIn) => {
            format!("Your sign-in to {origin} is not valid anymore. Sign in again with: {login}")
        }
        Some(TokenSource::Option) => format!(
            "{origin} does not accept the token of --auth_token. Without it, uc uses the sign-in \
             of: {login}"
        ),
        Some(TokenSource::Setting) => format!(
            "{origin} does not accept the token of {}. Without it, uc uses the sign-in of: {login}",
            settings::name("cli.token")
        ),
    }
}

/// `uc auth login` with the `--config` and `--server` of `global`: the
/// command that signs in to the same server.
pub fn login_command(global: &Global) -> String {
    let mut command = String::from("uc");
    if let Some(config) = &global.config {
        command.push_str(" --config ");
        command.push_str(&shell_word(&config.to_string_lossy()));
    }
    if let Some(server) = &global.server {
        command.push_str(" --server ");
        command.push_str(&shell_word(server));
    }
    command.push_str(" auth login");
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_ins_are_kept_by_server() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("uc").join("tokens.json");
        let sign_ins = SignIns::new(&file);
        assert_eq!(sign_ins.token("http://a:8080"), None);
        assert!(!sign_ins.remove("http://a:8080").unwrap());

        sign_ins.save("http://a:8080", "one").unwrap();
        sign_ins.save("http://b:8080", "two").unwrap();
        sign_ins.save("http://a:8080", "three").unwrap();
        assert_eq!(sign_ins.token("http://a:8080").as_deref(), Some("three"));
        assert_eq!(sign_ins.token("http://b:8080").as_deref(), Some("two"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        assert!(sign_ins.remove("http://a:8080").unwrap());
        assert_eq!(sign_ins.token("http://a:8080"), None);
        assert_eq!(sign_ins.token("http://b:8080").as_deref(), Some("two"));

        // A file that is not valid has no sign-ins, and the next one
        // replaces it.
        std::fs::write(&file, "not json").unwrap();
        assert_eq!(sign_ins.token("http://b:8080"), None);
        sign_ins.save("http://b:8080", "four").unwrap();
        assert_eq!(sign_ins.token("http://b:8080").as_deref(), Some("four"));
    }

    #[test]
    fn the_advice_names_the_token_and_the_command() {
        let global = |args: &[&str]| {
            use clap::Parser;
            crate::args::Cli::parse_from(std::iter::once("uc").chain(args.iter().copied())).global
        };
        assert_eq!(login_command(&global(&[])), "uc auth login");
        assert_eq!(
            login_command(&global(&[
                "--config",
                "my dir/uc.toml",
                "--server",
                "http://h:9"
            ])),
            "uc --config 'my dir/uc.toml' --server http://h:9 auth login"
        );

        let login = "uc auth login";
        let origin = "http://h:8080";
        assert_eq!(
            advice(origin, None, login),
            "http://h:8080 needs a sign-in. Sign in with: uc auth login"
        );
        assert_eq!(
            advice(origin, Some(TokenSource::SignIn), login),
            "Your sign-in to http://h:8080 is not valid anymore. Sign in again with: uc auth login"
        );
        assert!(
            advice(origin, Some(TokenSource::Option), login)
                .starts_with("http://h:8080 does not accept the token of --auth_token.")
        );
        assert!(
            advice(origin, Some(TokenSource::Setting), login)
                .contains("the token of cli.token (UC_CLI_TOKEN).")
        );
    }
}
