//! `uc` — Unity Catalog client.
//!
//! Three modes:
//!   - **CLI** ([`commands`]): `uc <entity> <operation> [options]`, with
//!     upstream's `bin/uc` syntax and output. The commands call the server
//!     with `uc_client`, print with [`output`] and work on data with
//!     [`data`].
//!   - **TUI** ([`tui`]): `uc` alone or `uc tui` opens an interactive
//!     resource browser.
//!   - **Local server** ([`local`]): `uc serve`, `uc dev` and `uc web` run
//!     uc-server and the web UI on this machine.
//!
//! [`args`] parses the command line. [`config`] gives the server and token
//! of all modes; [`sign_in`] keeps the sign-ins of `uc auth login`.

mod args;
mod browser;
mod commands;
mod config;
mod data;
mod local;
mod output;
mod sign_in;
mod tui;

use args::{Cli, Command};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    // The settings: flags, then `UC_<GROUP>_<KEY>` variables, then
    // `uc.toml` (`uc_common::settings`).
    let settings = match uc_common::settings::Settings::load(cli.global.config.as_deref()) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    match cli.command {
        Some(Command::Entity(command)) => {
            std::process::exit(commands::run(&cli.global, &settings, *command).await)
        }
        Some(Command::Serve { args }) => std::process::exit(local::serve(&settings, args).await?),
        Some(Command::Dev(args)) => std::process::exit(local::dev(&settings, args).await?),
        Some(Command::Web(args)) => local::web(&cli.global, &settings, args).await,
        Some(Command::Tui) | None => tui::run(&cli.global, &settings).await,
    }
}
