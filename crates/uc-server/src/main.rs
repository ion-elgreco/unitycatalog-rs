use std::{path::PathBuf, process::ExitCode, sync::Arc, time::Duration};

use tokio::net::TcpListener;
use tracing::{info, warn};

use uc_common::settings::SignInMode;
use uc_metastore::SqlMetastore;
use uc_server::{
    AppState, ServerConfig,
    authn::{
        bootstrap,
        security::{ADMIN, SecurityContext, generate_key_pem},
        sign_in::SignIn,
    },
    build_app, build_observability_app,
    observability::telemetry::init_tracing,
    server::{banner, options, serve},
    storage::vendor::CloudVendor,
};

/// The life of a token of `uc-server token` without `--ttl`.
const TOKEN_TTL: Duration = Duration::from_secs(3600);

// The main thread is worker 0 of `server::serve`; the other workers start
// their own runtimes.
#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let done = match options::parse(std::env::args().skip(1)) {
        options::Command::Run(options) => run(options).await,
        options::Command::Token(options) => token(options),
        options::Command::Keygen(output) => keygen(output.as_deref()),
        options::Command::Exit(code, text) => {
            match code {
                0 => print!("{text}"),
                _ => eprint!("{text}"),
            }
            return ExitCode::from(code as u8);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

/// `uc-server keygen`: print a new signing key, or write it to `output`, a
/// new file that only its owner reads.
fn keygen(output: Option<&std::path::Path>) -> Result<(), String> {
    let pem = generate_key_pem().map_err(|e| e.message().to_string())?;
    let Some(path) = output else {
        print!("{pem}");
        return Ok(());
    };
    let cannot = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path).map_err(cannot)?;
    std::io::Write::write_all(&mut file, pem.as_bytes()).map_err(cannot)?;
    eprintln!("Wrote a new signing key to {}.", path.display());
    Ok(())
}

/// `uc-server token`: print a token of the user `admin`, signed with the
/// key of `auth.signing_key_file`.
fn token(options: options::TokenOptions) -> Result<(), String> {
    let cfg = ServerConfig::load(options.config.as_deref())?;
    let Some(path) = &cfg.auth.signing_key_file else {
        return Err(
            "uc-server token needs auth.signing_key_file (UC_AUTH_SIGNING_KEY_FILE): \
                    without it, the signing key exists only in the memory of the running server."
                .into(),
        );
    };
    let ttl = options.ttl.unwrap_or(TOKEN_TTL);
    let longest = cfg.auth.access_token_timeout;
    if ttl.is_zero() || ttl > longest {
        return Err(format!(
            "--ttl must be more than 0s and at most auth.access_token_timeout ({})",
            duration_text(longest)
        ));
    }
    let security = SecurityContext::from_pem_file(path)
        .map_err(|e| format!("cannot load the signing key: {}", e.message()))?;
    let token = security
        .create_access_token(ADMIN, ttl)
        .map_err(|e| e.message().to_string())?;
    println!("{token}");
    Ok(())
}

/// `d` as the settings write durations, such as `1d` or `1h 30m`.
fn duration_text(d: Duration) -> String {
    let (days, hours, minutes, seconds) = (
        d.as_secs() / 86_400,
        d.as_secs() / 3600 % 24,
        d.as_secs() / 60 % 60,
        d.as_secs() % 60,
    );
    let parts: Vec<String> = [(days, "d"), (hours, "h"), (minutes, "m"), (seconds, "s")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, unit)| format!("{n}{unit}"))
        .collect();
    match parts.is_empty() {
        true => "0s".into(),
        false => parts.join(" "),
    }
}

/// Start the server and serve until Ctrl+C or SIGTERM.
async fn run(options: options::Options) -> Result<(), String> {
    // The settings: flags, then `UC_<GROUP>_<KEY>` variables, then
    // `uc.toml` (`uc_common::settings`).
    let cfg = ServerConfig::load(options.config.as_deref())?;
    init_tracing(cfg.log_dir.as_deref());
    uc_storage::runtime::init_io(cfg.io_threads);
    uc_storage::runtime::set_workers(cfg.workers);
    // Upstream `-p`: the API port on all interfaces.
    let bind = match options.port {
        Some(port) => std::net::SocketAddr::new(cfg.bind.ip(), port),
        None => cfg.bind,
    };
    banner::print(&cfg, bind);
    // Upstream `--obs-port` (or `server.obs_port`): probes and metrics on
    // their own port; none without it.
    let obs_port = options.obs_port.or(cfg.obs_port);
    if let Some(port) = obs_port {
        options::validate_obs_port(bind.port(), port)?;
    }
    let invalid_sign_in = |e: String| format!("invalid sign-in settings: {e}");
    let sign_in = cfg.sign_in().map_err(invalid_sign_in)?;
    let auth = cfg.auth_settings();
    auth.validate().map_err(invalid_sign_in)?;
    let seeds = bootstrap::seed_users(&cfg.auth)?;
    let authorization = cfg.auth.enabled;
    info!(
        %bind,
        settings = %cfg.file.as_ref().map_or("none".into(), |f| f.display().to_string()),
        data_dir = %cfg.data_dir.display(),
        database = %cfg.database.describe(),
        enforce_authz = authorization,
        "starting uc-server",
    );

    let metastore = SqlMetastore::connect(cfg.metastore_config())
        .await
        .map_err(|e| format!("failed to open the catalog database: {e}"))?;

    let mut state = AppState::new(metastore).with_authz(authorization);
    if authorization {
        uc_server::authn::security::initialize_admin(&state.metastore)
            .await
            .map_err(|e| format!("cannot create the admin user: {e}"))?;
    }
    // `[[auth.users]]` and `auth.admin_principals`: the users that are
    // missing, and the missing privileges of `[[auth.users]]`.
    if !seeds.is_empty() {
        let report = bootstrap::seed(&state.metastore, &seeds)
            .await
            .map_err(|e| format!("cannot make the users of auth.users: {e}"))?;
        info!(
            users = seeds.len(),
            created = report.created,
            granted = report.granted,
            "users of the settings checked"
        );
    }

    // The Cedar engine (`auth.cedar`), with `auth.cedar_policies` on top of
    // its defaults. Only with authorization on: Cedar is a policy engine,
    // not an authorization switch.
    if authorization && cfg.auth.cedar {
        let policies: Vec<PathBuf> = cfg.auth.cedar_policies.iter().map(PathBuf::from).collect();
        state = state
            .with_cedar(&policies)
            .map_err(|e| format!("Cedar engine init failed: {e}"))?;
        info!(
            extra_policies = policies.len(),
            "Cedar policy engine enabled"
        );
    }

    // `auth.admin_principals`: metastore admins besides its owner.
    let admins = &cfg.auth.admin_principals;
    if !admins.is_empty() {
        info!(count = admins.len(), "admin principals configured");
        state = state.with_admins(admins.clone());
    }

    state = state
        .with_delta_api_only(cfg.delta_api_only)
        .with_iceberg_writes_enabled(cfg.iceberg_writes_enabled)
        .with_external_local_roots(cfg.external_local_roots.clone())
        .with_http(cfg.http.clone());

    // `aws.master_role_arn`, shown on AWS IAM role credentials.
    if let Some(arn) = cfg
        .aws_master_role_arn
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
    {
        state = state.with_aws_master_role_arn(arn);
    }

    // The web UI's sign-in: `auth.mode` and its settings.
    match &sign_in {
        Some(SignIn::Oidc(oidc)) => {
            info!(issuer = %oidc.issuer, public_url = %oidc.public_url, "web sign-in with OpenID Connect")
        }
        Some(SignIn::Forward(forward)) => {
            info!(proxies = ?forward.trusted_proxies, "sign-in through an auth proxy")
        }
        None => {}
    }
    let auth = auth.with_sign_in(sign_in);
    // The key that signs the server's tokens: `auth.signing_key_file`, else
    // a new key in memory. The server never writes it.
    let security = match &cfg.auth.signing_key_file {
        Some(path) => {
            let security = SecurityContext::from_pem_file(path)
                .map_err(|e| format!("cannot load the signing key: {}", e.message()))?;
            info!(file = %path.display(), "signing key loaded");
            security
        }
        None => {
            let security = SecurityContext::generate()
                .map_err(|e| format!("cannot make a signing key: {}", e.message()))?;
            match authorization || cfg.auth.mode != SignInMode::None {
                true => warn!(
                    "auth.signing_key_file is not set: the signing key is in memory only, so \
                     sign-ins end when the server stops, and replicas cannot check each \
                     other's tokens. Set it for production."
                ),
                false => info!("signing key in memory (auth.signing_key_file is not set)"),
            }
            security
        }
    };
    state = state
        .with_security(Arc::new(security))
        .with_auth_settings(auth);

    // Upstream `CloudCredentialVendor`, with the credentials of `[aws]`,
    // `[s3.*]`, `[adls.*]` and `[gcs.*]`; the regions of `[s3.*]` also go to
    // Iceberg clients.
    let storage = cfg.credentials.clone();
    info!(
        s3 = storage.s3.len(),
        adls = storage.adls.len(),
        gcs = storage.gcs.len(),
        s3_endpoint = storage.s3_store.as_ref().map(|s| s.endpoint.as_str()),
        "credential vending configured"
    );
    for (bucket, s3) in &storage.s3 {
        if let Some(region) = &s3.region {
            state = state.with_s3_region(bucket, region);
        }
    }
    state = state.with_vendor(CloudVendor::new(storage));

    // Upstream `StorageCleanupWorker`: reclaims the files of dropped
    // managed tables, volumes and models.
    let cleanup_storage = Arc::new(uc_server::storage::cleanup::DefaultCleanupStorage::new(
        state.metastore.clone(),
        state.vendor.clone(),
    ));
    let cleanup_worker = uc_server::storage::cleanup::StorageCleanupWorker::new(
        state.metastore.clone(),
        cleanup_storage,
        cfg.cleanup.clone(),
    )
    .spawn();

    let observability = match obs_port {
        None => None,
        Some(port) => {
            // Upstream: one check before serving, then in the background.
            state
                .readiness
                .check(&state.metastore, cfg.readiness_timeout)
                .await;
            state.readiness.spawn(
                state.metastore.clone(),
                cfg.readiness_interval,
                cfg.readiness_timeout,
            );
            let address = std::net::SocketAddr::new([0, 0, 0, 0].into(), port);
            let listener = TcpListener::bind(address)
                .await
                .map_err(|e| format!("failed to bind the observability port {port}: {e}"))?;
            Some((listener, build_observability_app(state.clone())))
        }
    };

    if let Some(dir) = &cfg.http.ui_dir {
        info!(dir = %dir.display(), "serving the web UI");
    }
    // A router for each worker, each with its own copy of the state.
    let state = state.with_api_port(bind.port());
    let app = move || build_app(state.for_worker());

    let listener = TcpListener::bind(bind)
        .await
        .map_err(|e| format!("failed to bind {bind}: {e}"))?;
    let observability = observability.map(|(listener, app)| {
        info!(port = obs_port, "observability listening");
        tokio::spawn(async move { axum::serve(listener, app).await })
    });

    info!(%bind, workers = cfg.workers, "listening");
    let served = serve::serve(listener, app, cfg.workers, shutdown_signal()).await;
    cleanup_worker.abort();
    if let Some(observability) = observability {
        observability.abort();
    }
    served.map_err(|e| format!("server error: {e}"))?;
    info!("shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Ctrl+C received, draining in-flight requests"),
        _ = terminate => info!("SIGTERM received, draining in-flight requests"),
    }
}
