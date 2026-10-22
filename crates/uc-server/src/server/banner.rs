//! The lines that uc-server prints when it starts: its name and version,
//! the URLs of the web UI and the API, and the data folder. Next to the
//! logo at a terminal that shows colors.

use std::io::IsTerminal as _;
use std::net::SocketAddr;

use super::config::ServerConfig;

/// Print the banner of a server on `bind`.
pub fn print(cfg: &ServerConfig, bind: SocketAddr) {
    let lines = lines(cfg, bind);
    match std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        true => println!("{}", uc_common::logo::ansi(&lines)),
        false => println!("{}\n", lines.join("\n")),
    }
}

fn lines(cfg: &ServerConfig, bind: SocketAddr) -> Vec<String> {
    let host = match bind.ip().is_unspecified() || bind.ip().is_loopback() {
        true => "localhost".to_string(),
        false => bind.ip().to_string(),
    };
    let url = format!("http://{host}:{}", bind.port());
    let mut lines = vec![
        format!("unitycatalog-rs {}", env!("CARGO_PKG_VERSION")),
        String::new(),
    ];
    if cfg.http.ui_dir.is_some() {
        lines.push(format!("UI:   {url}"));
    }
    lines.push(format!("API:  {url}{}", uc_common::API_BASE));
    lines.push(format!("Data: {}", cfg.data_dir.display()));
    lines
}
