//! The command line, parsed with clap: `uc <entity> <operation> [options]`
//! ([`crate::commands::Entity`]), `uc tui`, `uc serve`, `uc dev` and `uc web`.
//! `--server`, `--auth_token` and `--output` go before or after the
//! command.

use clap::{ArgAction, Args, Parser, Subcommand};

use crate::commands::Entity;
use crate::output::Format;

/// Unity Catalog client. With no command, it opens the terminal UI.
#[derive(Debug, Parser)]
#[command(
    name = "uc",
    version,
    disable_version_flag = true,
    after_help = CONNECTION
)]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', long, action = ArgAction::Version)]
    version: (),
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Option<Command>,
}

const CONNECTION: &str = "Without --server and --auth_token, the server and token come from the \
    variables UC_CLI_SERVER and UC_CLI_TOKEN, else from server and token in the [cli] table of \
    uc.toml (--config, else UC_CONFIG, else uc.toml in this folder or a folder above). The server \
    defaults to http://localhost:8080. Without a token, uc sends the one of its sign-in to the \
    server: uc auth login signs in, and uc auth logout signs out.";

// The options of every command.
#[derive(Debug, Args)]
#[command(next_help_heading = "Global options")]
pub struct Global {
    /// UC server to connect to
    #[arg(long, global = true)]
    pub server: Option<String>,
    /// Token to authorize requests
    #[arg(long = "auth_token", global = true)]
    pub auth_token: Option<String>,
    /// Output format
    #[arg(long, global = true, value_enum, default_value_t = Format::Table)]
    pub output: Format,
    /// Settings file, instead of UC_CONFIG or uc.toml
    #[arg(long, global = true)]
    pub config: Option<std::path::PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(flatten)]
    Entity(Box<Entity>),
    /// Browse the catalog in a terminal UI (also `uc` with no command)
    Tui,
    /// Run uc-server; `uc serve --help` lists its options
    #[command(disable_help_flag = true)]
    Serve {
        /// The options of uc-server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run uc-server on localhost, with the web UI
    Dev(DevArgs),
    /// Serve the web UI on localhost; its API calls go to --server
    Web(WebArgs),
}

#[derive(Debug, Args)]
#[command(rename_all = "snake_case")]
pub struct DevArgs {
    /// The port of the API and the web UI
    #[arg(long, default_value_t = 8080)]
    pub port: u16,
    /// The folder of the database and the keys (server.data_dir)
    #[arg(long, default_value = ".uc")]
    pub dir: std::path::PathBuf,
    /// Do not open the web UI in a browser
    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Debug, Args)]
#[command(rename_all = "snake_case")]
pub struct WebArgs {
    /// The port of the web UI
    #[arg(long, default_value_t = 3000)]
    pub port: u16,
    /// Do not open the web UI in a browser
    #[arg(long)]
    pub no_browser: bool,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("uc").chain(args.iter().copied()))
    }

    #[test]
    fn definitions_are_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn opens_the_tui() {
        // `uc` alone or `uc tui`, with the common options.
        assert!(parse(&["--server", "http://h"]).unwrap().command.is_none());
        let cli = parse(&["tui", "--auth_token", "t"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Tui)));
        assert_eq!(cli.global.auth_token.as_deref(), Some("t"));
    }
}
