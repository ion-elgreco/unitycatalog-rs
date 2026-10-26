//! `uc serve`, `uc dev` and `uc web`: uc-server and the web UI on this
//! machine. `uc` runs the `uc-server` binary that the Python package
//! `unitycatalog-rs-server` installs next to it, and serves the web UI that
//! `unitycatalog-rs-ui` installs (see `uc_common::install`).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow, bail};
use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderName, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use uc_common::settings::{self, Settings};

use crate::args::{DevArgs, Global, WebArgs};
use crate::sign_in::SignIns;
use crate::{browser, config};

/// How long `uc dev` waits for uc-server to answer, and to stop.
const START_TIMEOUT: Duration = Duration::from_secs(60);
const STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// The largest request body that `uc web` sends on.
const MAX_BODY: usize = 64 * 1024 * 1024;

const NO_SERVER: &str =
    "uc-server is not installed. Install it with: pip install 'unitycatalog-rs[server]'";
const NO_UI: &str =
    "The web UI is not installed. Install it with: pip install 'unitycatalog-rs[ui]'";

/// The uc-server binary: `cli.server_bin` (`UC_CLI_SERVER_BIN`), else
/// `uc-server` next to `uc` (as the Python wheels install it), else on
/// `PATH`.
fn server_binary(settings: &Settings) -> anyhow::Result<PathBuf> {
    if let Some(path) = settings.cli.server_bin.clone() {
        return Ok(path);
    }
    let name = format!("uc-server{}", std::env::consts::EXE_SUFFIX);
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| std::fs::canonicalize(&exe).unwrap_or(exe))
        .and_then(|exe| Some(exe.parent()?.join(&name)));
    let on_path: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(&name))
                .collect()
        })
        .unwrap_or_default();
    beside
        .into_iter()
        .chain(on_path)
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow!(NO_SERVER))
}

/// The settings file of `uc`, for uc-server: the same, wherever it runs.
fn config_file(settings: &Settings) -> Option<PathBuf> {
    let file = settings.file.as_ref()?;
    Some(std::fs::canonicalize(file).unwrap_or_else(|_| file.clone()))
}

/// `uc serve`: run uc-server with `args`; its exit code.
pub async fn serve(settings: &Settings, args: Vec<String>) -> anyhow::Result<i32> {
    let server = server_binary(settings)?;
    let mut command = std::process::Command::new(&server);
    command.args(&args);
    if let Some(file) = config_file(settings) {
        command.env("UC_CONFIG", file);
    }
    // uc-server takes this process over, with its signals and exit code.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = command.exec();
        bail!("cannot run {}: {e}", server.display())
    }
    #[cfg(not(unix))]
    {
        let mut child = tokio::process::Command::from(command)
            .spawn()
            .with_context(|| format!("cannot run {}", server.display()))?;
        // Ctrl+C reaches uc-server too: wait for it to stop.
        loop {
            tokio::select! {
                status = child.wait() => return Ok(status?.code().unwrap_or(1)),
                _ = tokio::signal::ctrl_c() => {}
            }
        }
    }
}

/// `uc dev`: uc-server on localhost, with the catalog in memory, its
/// managed data in `args.dir` and the web UI; uc-server's exit code.
pub async fn dev(settings: &Settings, args: DevArgs) -> anyhow::Result<i32> {
    // From here on, Ctrl+C and SIGTERM stop uc-server first.
    let mut stop = StopSignal::new()?;
    let server = server_binary(settings)?;
    std::fs::create_dir_all(&args.dir)
        .with_context(|| format!("cannot make {}", args.dir.display()))?;
    let dir = std::fs::canonicalize(&args.dir)?;
    // The UI of `ui.dir`, else next to `uc`, else next to uc-server; none
    // with `ui.enabled = false`.
    let ui = match settings.ui.enabled {
        false => None,
        true => uc_common::install::ui_dir(settings.ui.dir.clone()).or_else(|| {
            if settings.ui.dir.is_some() {
                return None;
            }
            let server = std::fs::canonicalize(&server).ok()?;
            Some(uc_common::install::share_dir_of(&server)?.join("ui")).filter(|d| d.is_dir())
        }),
    };

    // A server that already listens on the port would answer for this one.
    let bind = SocketAddr::from(([127, 0, 0, 1], args.port));
    std::net::TcpListener::bind(bind).with_context(|| {
        format!(
            "port {} is in use; choose another one with --port",
            args.port
        )
    })?;
    let mut command = tokio::process::Command::new(&server);
    command
        .env("UC_SERVER_BIND", bind.to_string())
        .env("UC_SERVER_DATA_DIR", &dir)
        .kill_on_drop(true);
    if let Some(file) = config_file(settings) {
        command.env("UC_CONFIG", file);
    }
    // The catalog is in Turso in memory, unless the settings name a database.
    let in_memory = settings.db.url.is_none();
    if in_memory {
        command.env("UC_DB_URL", "memory");
    }
    // Managed tables and models of a catalog without a storage root, such
    // as one made in the web UI, go to `storage` in the data folder, unless
    // the settings name a root.
    let storage = url::Url::from_file_path(dir.join("storage"))
        .map_err(|()| anyhow!("{} is not an absolute path", dir.display()))?;
    if settings.storage.tables_root.is_none() {
        command.env("UC_STORAGE_TABLES_ROOT", storage.as_str());
    }
    if settings.storage.models_root.is_none() {
        command.env("UC_STORAGE_MODELS_ROOT", storage.as_str());
    }
    match &ui {
        Some(ui) => command.env("UC_UI_DIR", ui),
        None => command.env("UC_UI_ENABLED", "false"),
    };
    let mut child = command
        .spawn()
        .with_context(|| format!("cannot run {}", server.display()))?;
    if !wait_until_up(&mut child, bind, &mut stop).await? {
        return stop_server(&mut child).await;
    }

    // uc-server's banner shows the URLs and the data folder.
    match &ui {
        Some(_) => {}
        None if !settings.ui.enabled => {
            println!("No web UI: {} is false.", settings::name("ui.enabled"))
        }
        None => println!("No web UI. {NO_UI}"),
    }
    if in_memory {
        println!(
            "The catalog is in memory: it is gone when uc dev stops. To keep it, set {} to file:<path>.",
            settings::name("db.url")
        );
    }
    println!("Press Ctrl+C to stop it.");
    if ui.is_some() && !args.no_browser {
        browser::open(&format!("http://localhost:{}", args.port));
    }
    tokio::select! {
        status = child.wait() => Ok(status?.code().unwrap_or(1)),
        _ = stop.recv() => stop_server(&mut child).await,
    }
}

/// Wait until uc-server answers on `addr`: true; false on a stop signal.
async fn wait_until_up(
    child: &mut tokio::process::Child,
    addr: SocketAddr,
    stop: &mut StopSignal,
) -> anyhow::Result<bool> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let url = format!("http://{addr}/");
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            bail!("uc-server stopped: {status}");
        }
        if let Ok(response) = client.get(&url).send().await
            && response.status().is_success()
        {
            return Ok(true);
        }
        if start.elapsed() > START_TIMEOUT {
            bail!(
                "uc-server does not answer on {url} after {} seconds",
                START_TIMEOUT.as_secs()
            );
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            _ = stop.recv() => return Ok(false),
        }
    }
}

/// Stop uc-server, and wait until it has stopped; its exit code. In a
/// terminal, Ctrl+C reaches uc-server too; a signal sent to `uc` alone does
/// not.
async fn stop_server(child: &mut tokio::process::Child) -> anyhow::Result<i32> {
    terminate(child);
    match tokio::time::timeout(STOP_TIMEOUT, child.wait()).await {
        Ok(status) => Ok(status?.code().unwrap_or(1)),
        Err(_) => {
            child.kill().await?;
            Ok(1)
        }
    }
}

/// Ask uc-server to stop: SIGTERM, after which it finishes its requests.
#[cfg(unix)]
fn terminate(child: &tokio::process::Child) {
    if let Some(pid) = child.id() {
        // SAFETY: kill(2) only sends a signal; it touches no memory.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    }
}

/// On Windows, Ctrl+C reaches uc-server through the console.
#[cfg(not(unix))]
fn terminate(_: &tokio::process::Child) {}

/// Ctrl+C, or SIGTERM on Unix. `uc` handles them from `new` on: one that
/// comes before `recv` waits for it, and does not end `uc` at once.
struct StopSignal {
    #[cfg(unix)]
    interrupt: tokio::signal::unix::Signal,
    #[cfg(unix)]
    terminate: tokio::signal::unix::Signal,
    #[cfg(windows)]
    ctrl_c: tokio::signal::windows::CtrlC,
}

impl StopSignal {
    fn new() -> std::io::Result<Self> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            Ok(Self {
                interrupt: signal(SignalKind::interrupt())?,
                terminate: signal(SignalKind::terminate())?,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                ctrl_c: tokio::signal::windows::ctrl_c()?,
            })
        }
    }

    async fn recv(&mut self) {
        #[cfg(unix)]
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
        }
        #[cfg(windows)]
        self.ctrl_c.recv().await;
    }
}

/// `uc web`: the web UI on localhost. Its API calls (`/api/…`) go on to
/// the server of the `uc` commands ([`config::server`]), so the UI and the
/// API share one origin. They are signed in as the other `uc` commands, with
/// their token ([`config::token`]). The server's API docs (`/docs`,
/// `/api-docs/…`), which the UI links to, come from the server too.
pub async fn web(global: &Global, settings: &Settings, args: WebArgs) -> anyhow::Result<()> {
    let mut stop = StopSignal::new()?;
    let ui = match settings.ui.enabled {
        true => uc_common::install::ui_dir(settings.ui.dir.clone()),
        false => None,
    }
    .ok_or_else(|| anyhow!(NO_UI))?;
    let origin = config::origin(&config::server(settings, global.server.as_deref()))?;
    let proxy = Arc::new(Proxy {
        origin: origin.clone(),
        token: config::token(
            settings,
            global.auth_token.as_deref(),
            &origin,
            SignIns::of_user().as_ref(),
        )
        .map(|(token, _)| token),
        // Redirects go back to the browser.
        client: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
    });
    let app = Router::new()
        .route("/api/{*rest}", axum::routing::any(forward))
        // Swagger UI and its OpenAPI document. `/docs/{*rest}` does not match
        // `/docs/`.
        .route("/docs", get(forward))
        .route("/docs/", get(forward))
        .route("/docs/{*rest}", get(forward))
        .route("/api-docs/{*rest}", get(forward))
        .with_state(proxy)
        .route("/auth/mode", get(mode))
        // The server's sign-in sets its cookie for the server's own address,
        // not for this one.
        .route(
            "/auth/{*rest}",
            get(|| async { (StatusCode::NOT_FOUND, NO_SIGN_IN) }),
        )
        // The UI's files, as uc-server serves them; client-side routes load
        // the UI.
        .fallback_service(uc_common::ui::files(&ui));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", args.port))
        .await
        .with_context(|| {
            format!(
                "cannot listen on port {}; choose another one with --port",
                args.port
            )
        })?;
    let url = format!("http://localhost:{}", args.port);
    println!("The web UI runs on {url}. Its API calls go to {origin}.");
    println!("Press Ctrl+C to stop it.");
    if !args.no_browser {
        browser::open(&url);
    }
    axum::serve(listener, app)
        .with_graceful_shutdown(async move { stop.recv().await })
        .await?;
    Ok(())
}

struct Proxy {
    origin: String,
    /// The token of the `uc` commands, for the UI's own calls.
    token: Option<String>,
    client: reqwest::Client,
}

const NO_SIGN_IN: &str = "uc web signs in as the other uc commands do: with the token of \
    --auth_token, else of UC_CLI_TOKEN, else of token in the [cli] table of uc.toml, else of \
    the sign-in of uc auth login.";

/// `GET /auth/mode`, which uc-server answers with its sign-in: here the UI
/// signs in with the token of the `uc` commands (`token`), so it has no
/// "Sign in" and no "Log out".
async fn mode() -> Response {
    let body = Json(serde_json::json!({ "mode": "token", "logout_url": null }));
    ([(header::CACHE_CONTROL, "no-store")], body).into_response()
}

/// Whether the browser sent `request` from a page of this address (Fetch
/// Metadata), not from another site: only those get the `uc` token, so
/// other sites cannot use it through this port.
fn from_this_page(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get("sec-fetch-site")
        .is_some_and(|site| site == "same-origin")
}

/// Headers for one connection only, which a proxy does not pass on
/// (RFC 9110, section 7.6.1).
fn hop_by_hop(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Send an API call of the UI on to the server, and its response back.
async fn forward(State(proxy): State<Arc<Proxy>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path_and_query().map_or("/", |p| p.as_str());
    let url = format!("{}{path}", proxy.origin);
    let body = match axum::body::to_bytes(body, MAX_BODY).await {
        Ok(body) => body,
        Err(e) => return (StatusCode::PAYLOAD_TOO_LARGE, e.to_string()).into_response(),
    };
    let mut call = proxy.client.request(parts.method, &url).body(body);
    for (name, value) in &parts.headers {
        if !hop_by_hop(name) && name != header::HOST && name != header::CONTENT_LENGTH {
            call = call.header(name, value);
        }
    }
    if let Some(token) = &proxy.token
        && !parts.headers.contains_key(header::AUTHORIZATION)
        && from_this_page(&parts.headers)
    {
        call = call.bearer_auth(token);
    }
    let response = match call.send().await {
        Ok(response) => response,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("{url}: {e}")).into_response(),
    };
    let mut back = Response::builder().status(response.status());
    for (name, value) in response.headers() {
        if !hop_by_hop(name) {
            back = back.header(name, value);
        }
    }
    back.body(Body::from_stream(response.bytes_stream()))
        .unwrap_or_else(|e| (StatusCode::BAD_GATEWAY, e.to_string()).into_response())
}
