//! Command-line options, after upstream `OptionParser`: `-p/--port`,
//! `--obs-port`, `-v/--version` and `-h/--help`; and `--config`, the
//! settings file (`uc_common::settings`). Two commands of this project:
//! `token` and `keygen`.

use std::time::Duration;

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Run(Options),
    /// `uc-server token`: print a token of the user `admin`.
    Token(TokenOptions),
    /// `uc-server keygen`: print a new key, or write it to the file of
    /// `--output`.
    Keygen(KeygenOptions),
    /// Print the text and exit with the code.
    Exit(i32, String),
}

/// The options of `uc-server keygen`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct KeygenOptions {
    /// `--secrets`: a P-256 key for `secrets.key_file`, instead of an RSA
    /// key for `auth.signing_key_file`.
    pub secrets: bool,
    /// The new file to write it to, instead of the standard output.
    pub output: Option<std::path::PathBuf>,
}

/// The options of `uc-server token`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TokenOptions {
    /// When the token expires (default one hour).
    pub ttl: Option<Duration>,
    /// The settings file, instead of `UC_CONFIG` or `uc.toml`.
    pub config: Option<std::path::PathBuf>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// The API port (upstream default 8080).
    pub port: Option<u16>,
    /// The port of `/livez`, `/readyz` and `/metrics`; none without it.
    pub obs_port: Option<u16>,
    /// The settings file, instead of `UC_CONFIG` or `uc.toml`.
    pub config: Option<std::path::PathBuf>,
}

/// Upstream's help, as commons-cli formats it.
pub fn help() -> String {
    [
        "usage: uc-server",
        "    --config <arg>      The settings file. Default: UC_CONFIG, else uc.toml",
        "                        in this folder or a folder above.",
        " -h,--help              Print help message.",
        "    --obs-port <arg>    Enable health and metrics on this port (1-65535).",
        "                        Disabled when omitted.",
        " -p,--port <arg>        Port number to run the server on. Default is 8080.",
        " -v,--version           Display the version of the Unity Catalog server",
        "",
        "commands:",
        "    token [--ttl <arg>] [--config <arg>]",
        "                        Print a token of the user admin that expires",
        "                        after --ttl (default 1h). Needs",
        "                        auth.signing_key_file.",
        "    keygen [--secrets] [--output <arg>]",
        "                        Print a new RSA key for auth.signing_key_file,",
        "                        or with --secrets a P-256 key for",
        "                        secrets.key_file; --output writes it to a new",
        "                        file that only its owner reads.",
        "",
    ]
    .join("\n")
}

fn failed(reason: String) -> Command {
    Command::Exit(
        255,
        format!("\nParsing Failed. Reason: {reason}\n\n{}", help()),
    )
}

/// `--name=value` as the name and the value; other arguments as they are.
fn split(arg: &str) -> (String, Option<String>) {
    match arg.split_once('=') {
        Some((name, value)) if arg.starts_with("--") => (name.to_string(), Some(value.to_string())),
        _ => (arg.to_string(), None),
    }
}

/// Parse the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Command {
    let mut args = args.into_iter().peekable();
    match args.peek().map(String::as_str) {
        Some("token") => {
            args.next();
            return parse_token(args);
        }
        Some("keygen") => {
            args.next();
            return parse_keygen(args);
        }
        _ => {}
    }
    let mut options = Options::default();
    let mut port: Option<String> = None;
    let mut obs_port: Option<String> = None;
    while let Some(arg) = args.next() {
        let (name, inline) = split(&arg);
        let mut value = |label: &str| -> Result<String, Command> {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| failed(format!("Missing argument for option: {label}")))
        };
        match name.as_str() {
            "-h" | "--help" => return Command::Exit(0, help()),
            "-v" | "--version" => {
                return Command::Exit(0, format!("{}\n", env!("CARGO_PKG_VERSION")));
            }
            "-p" | "--port" => match value("p") {
                Ok(v) => port = Some(v),
                Err(exit) => return exit,
            },
            "--obs-port" => match value("obs-port") {
                Ok(v) => obs_port = Some(v),
                Err(exit) => return exit,
            },
            "--config" => match value("config") {
                Ok(v) => options.config = Some(v.into()),
                Err(exit) => return exit,
            },
            short if short.starts_with("-p") && short.len() > 2 => {
                port = Some(short[2..].to_string())
            }
            other => return failed(format!("Unrecognized option: {other}")),
        }
    }
    if let Some(port) = port {
        match port.trim().parse::<u16>() {
            Ok(p) => options.port = Some(p),
            Err(_) => return failed(format!("For input string: \"{port}\"")),
        }
    }
    if let Some(obs) = obs_port {
        match obs.trim().parse::<i64>() {
            Ok(p) => match u16::try_from(p) {
                Ok(p) if p >= 1 => options.obs_port = Some(p),
                _ => {
                    return Command::Exit(
                        1,
                        format!("Observability port must be between 1 and 65535: {p}\n"),
                    );
                }
            },
            Err(_) => return failed(format!("For input string: \"{obs}\"")),
        }
    }
    Command::Run(options)
}

/// The options of `uc-server token`: `--ttl`, `--config` and `--help`.
fn parse_token(mut args: impl Iterator<Item = String>) -> Command {
    let mut options = TokenOptions::default();
    while let Some(arg) = args.next() {
        let (name, inline) = split(&arg);
        let mut value = |label: &str| -> Result<String, Command> {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| failed(format!("Missing argument for option: {label}")))
        };
        match name.as_str() {
            "-h" | "--help" => return Command::Exit(0, help()),
            "--ttl" => match value("ttl") {
                Ok(v) => match uc_common::settings::parse_duration(v.trim()) {
                    Ok(ttl) => options.ttl = Some(ttl),
                    Err(e) => return failed(format!("--ttl: {e}")),
                },
                Err(exit) => return exit,
            },
            "--config" => match value("config") {
                Ok(v) => options.config = Some(v.into()),
                Err(exit) => return exit,
            },
            other => return failed(format!("Unrecognized option: {other}")),
        }
    }
    Command::Token(options)
}

/// The options of `uc-server keygen`: `--secrets`, `--output` and
/// `--help`.
fn parse_keygen(mut args: impl Iterator<Item = String>) -> Command {
    let mut options = KeygenOptions::default();
    while let Some(arg) = args.next() {
        let (name, inline) = split(&arg);
        match name.as_str() {
            "-h" | "--help" => return Command::Exit(0, help()),
            "--secrets" if inline.is_none() => options.secrets = true,
            "--output" => match inline.or_else(|| args.next()) {
                Some(path) => options.output = Some(path.into()),
                None => return failed("Missing argument for option: output".into()),
            },
            other => return failed(format!("Unrecognized option: {other}")),
        }
    }
    Command::Keygen(options)
}

/// Upstream `validateObservabilityPort`: not the API port.
pub fn validate_obs_port(api_port: u16, obs_port: u16) -> Result<(), String> {
    match api_port == obs_port {
        true => Err(format!(
            "Observability port ({obs_port}) is already used by an API listener"
        )),
        false => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Command {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_upstream_options() {
        assert_eq!(run(&[]), Command::Run(Options::default()));
        assert_eq!(
            run(&["-p", "9000"]),
            Command::Run(Options {
                port: Some(9000),
                ..Default::default()
            })
        );
        assert_eq!(
            run(&[
                "--port=9000",
                "--obs-port",
                "9464",
                "--config",
                "/etc/uc/uc.toml"
            ]),
            Command::Run(Options {
                port: Some(9000),
                obs_port: Some(9464),
                config: Some("/etc/uc/uc.toml".into())
            })
        );
        assert_eq!(
            run(&["-p9001"]),
            Command::Run(Options {
                port: Some(9001),
                ..Default::default()
            })
        );
        assert!(
            matches!(run(&["-h"]), Command::Exit(0, text) if text.starts_with("usage: uc-server"))
        );
        assert!(
            matches!(run(&["--version"]), Command::Exit(0, text) if text.trim() == env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn parses_the_commands() {
        assert_eq!(run(&["token"]), Command::Token(TokenOptions::default()));
        assert_eq!(
            run(&["token", "--ttl", "10m", "--config=/etc/uc/uc.toml"]),
            Command::Token(TokenOptions {
                ttl: Some(Duration::from_secs(600)),
                config: Some("/etc/uc/uc.toml".into()),
            })
        );
        assert!(
            matches!(run(&["token", "--ttl", "soon"]), Command::Exit(255, text) if text.contains("--ttl: 'soon' is not a duration"))
        );
        assert!(
            matches!(run(&["token", "-p", "1"]), Command::Exit(255, text) if text.contains("Unrecognized option: -p"))
        );
        assert!(matches!(run(&["token", "--help"]), Command::Exit(0, _)));
        assert_eq!(run(&["keygen"]), Command::Keygen(KeygenOptions::default()));
        assert_eq!(
            run(&["keygen", "--output", "/data/key.pem"]),
            Command::Keygen(KeygenOptions {
                secrets: false,
                output: Some("/data/key.pem".into()),
            })
        );
        assert_eq!(
            run(&["keygen", "--secrets", "--output=/data/secrets.pem"]),
            Command::Keygen(KeygenOptions {
                secrets: true,
                output: Some("/data/secrets.pem".into()),
            })
        );
        assert!(matches!(run(&["keygen", "x"]), Command::Exit(255, _)));
        assert!(matches!(
            run(&["keygen", "--output"]),
            Command::Exit(255, _)
        ));
        // Not a command after an option.
        assert!(matches!(run(&["-p", "1", "token"]), Command::Exit(255, _)));
    }

    #[test]
    fn refuses_bad_options() {
        assert!(
            matches!(run(&["--nope"]), Command::Exit(255, text) if text.contains("Unrecognized option: --nope"))
        );
        assert!(
            matches!(run(&["-p"]), Command::Exit(255, text) if text.contains("Missing argument for option: p"))
        );
        assert!(matches!(run(&["-p", "x"]), Command::Exit(255, _)));
        assert_eq!(
            run(&["--obs-port", "0"]),
            Command::Exit(
                1,
                "Observability port must be between 1 and 65535: 0\n".into()
            )
        );
        assert_eq!(
            validate_obs_port(8080, 8080),
            Err("Observability port (8080) is already used by an API listener".to_string())
        );
    }
}
