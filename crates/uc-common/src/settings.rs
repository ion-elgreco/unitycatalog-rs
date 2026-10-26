//! The settings of uc-server and uc.
//!
//! Each setting is a `key` in a `[group]` table of `uc.toml` and the
//! environment variable `UC_<GROUP>_<KEY>`: `oidc_issuer` in `[auth]` is
//! `UC_AUTH_OIDC_ISSUER`. A setting takes its value from the first of:
//!
//! 1. a command-line flag,
//! 2. the environment variable,
//! 3. `uc.toml`,
//! 4. the default.
//!
//! The server's identity in a cloud is a table per cloud, such as
//! `[identity.aws]`, and its variables include the cloud:
//! `UC_IDENTITY_AWS_ACCESS_KEY`. Locations are named tables, such as
//! `[locations.analytics]`, and their variables include the name:
//! `UC_LOCATIONS_ANALYTICS_URL` is `url` in `[locations.analytics]`. A name
//! has only lowercase letters and digits, and at least one letter.
//!
//! # `uc.toml`
//!
//! The file is the first of: `--config`, `UC_CONFIG`, or the first `uc.toml`
//! in the working directory or a directory above it. Without a file, the
//! variables and the defaults apply.
//!
//! # Values
//!
//! - A key that is not a setting is an error, in the file and in a variable
//!   of a group. Other `UC_` variables, such as `UC_CONFIG`, are not settings.
//! - Variables are text. Numbers and booleans are parsed from the text, and
//!   an empty variable is unset.
//! - A duration is text such as `30s`, `1m` or `5d`.
//! - A list is a TOML array, or text with commas (`a,b`).

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use figment2::Figment;
use figment2::providers::{Env, Format, Toml};
use serde::{Deserialize, Deserializer};

/// The groups of `uc.toml`; a variable of another group (`UC_CONFIG`,
/// `UC_TEST_POSTGRES_URL`) is not a setting.
const GROUPS: &[&str] = &[
    "server",
    "db",
    "auth",
    "storage",
    "secrets",
    "tables",
    "identity",
    "locations",
    "ui",
    "log",
    "cli",
];

/// The groups of tables in tables: `UC_IDENTITY_<CLOUD>_<KEY>` and
/// `UC_LOCATIONS_<NAME>_<KEY>`.
const NESTED: &[&str] = &["identity", "locations"];

/// The settings of uc-server and uc: every group of `uc.toml`. uc-server
/// reads all groups but `[cli]`; uc reads `[cli]`, `[ui]`,
/// `identity.aws.endpoint` and `identity.aws.region`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub server: Server,
    pub db: Db,
    pub auth: Auth,
    pub storage: Storage,
    pub secrets: Secrets,
    pub tables: Tables,
    pub identity: Identity,
    /// Locations and the identities that reach them, by name.
    pub locations: BTreeMap<String, Location>,
    pub ui: Ui,
    pub log: Log,
    pub cli: Cli,
    /// The `uc.toml` that was read.
    #[serde(skip)]
    pub file: Option<PathBuf>,
}

/// `[server]`: where uc-server keeps its state, its listeners and limits.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    /// The folder of the embedded database, unless `db.url` names another
    /// database (default `.uc`).
    pub data_dir: PathBuf,
    /// The API's address (default `0.0.0.0:8080`).
    pub bind: SocketAddr,
    /// `/livez`, `/readyz` and `/metrics` on their own port; none without
    /// it.
    #[serde(deserialize_with = "some")]
    pub obs_port: Option<u16>,
    /// The largest request body, in bytes (default 10 MiB, as Armeria).
    #[serde(deserialize_with = "scalar")]
    pub max_request_length: usize,
    /// The time a request may take (default `10s`, as Armeria; `0s`
    /// for none).
    #[serde(deserialize_with = "duration")]
    pub request_timeout: Duration,
    /// Requests per second and client address; 0 for no limit.
    #[serde(deserialize_with = "scalar")]
    pub rate_limit_per_sec: u32,
    /// The origins that browsers may call the API from; none for every
    /// origin.
    #[serde(deserialize_with = "list")]
    pub cors_allow_origins: Vec<String>,
    /// How often `/readyz` checks the database (default `5s`).
    #[serde(deserialize_with = "duration")]
    pub readiness_interval: Duration,
    /// How long that check may take (default `2s`).
    #[serde(deserialize_with = "duration")]
    pub readiness_db_timeout: Duration,
    /// Threads that serve requests, each with a runtime of its own and its
    /// share of the PostgreSQL connections (default: one per CPU).
    #[serde(deserialize_with = "some")]
    pub workers: Option<usize>,
    /// Worker threads of the runtime for outbound IO: the connections to
    /// object stores and HTTP services (default 1).
    #[serde(deserialize_with = "scalar")]
    pub io_threads: usize,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".uc"),
            bind: SocketAddr::from(([0, 0, 0, 0], 8080)),
            obs_port: None,
            max_request_length: 10 * 1024 * 1024,
            request_timeout: Duration::from_secs(10),
            rate_limit_per_sec: 0,
            cors_allow_origins: Vec::new(),
            readiness_interval: Duration::from_secs(5),
            readiness_db_timeout: Duration::from_secs(2),
            workers: None,
            io_threads: 1,
        }
    }
}

/// `[db]`: the catalog database.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Db {
    /// `memory`, `file:<path>`, `postgresql://…` or `jdbc:postgresql://…`
    /// (default: the Turso file `uc.db` in `server.data_dir`).
    pub url: Option<String>,
    /// The PostgreSQL user and password, instead of the URL's.
    pub user: Option<String>,
    pub password: Option<String>,
    /// The PostgreSQL connections (default 20).
    #[serde(deserialize_with = "some")]
    pub pool_size: Option<usize>,
}

/// How the server signs users in to the web UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignInMode {
    /// No sign-in to the web UI.
    #[default]
    None,
    /// With an OpenID Connect provider.
    Oidc,
    /// Through an auth proxy in front of the server.
    Forward,
}

/// `[auth]`: sign-in and authorization.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Auth {
    /// Sign-in and privilege checks (default false).
    #[serde(deserialize_with = "scalar")]
    pub enabled: bool,
    /// The PEM file of the RSA keys of the server's tokens, newest first:
    /// the first signs, each checks the tokens it signed. Without it, the
    /// server makes a key in memory at each start: its tokens end when it
    /// stops, and replicas cannot check each other's tokens.
    pub signing_key_file: Option<PathBuf>,
    /// The users that the server makes at each start when they are missing
    /// (`[[auth.users]]`, only in `uc.toml`).
    pub users: Vec<AuthUser>,
    /// The issuers of identity tokens that the server exchanges (exact, or
    /// `*` for one DNS label).
    #[serde(deserialize_with = "list")]
    pub allowed_issuers: Vec<String>,
    /// The audiences that those tokens must have; `*` alone for any.
    #[serde(deserialize_with = "list")]
    pub audiences: Vec<String>,
    /// The life of the server's access tokens (default `24h`).
    #[serde(deserialize_with = "duration")]
    pub access_token_timeout: Duration,
    /// The life of the `UC_TOKEN` cookie (default `5d`).
    #[serde(deserialize_with = "duration")]
    pub cookie_timeout: Duration,
    /// Users with every privilege.
    #[serde(deserialize_with = "list")]
    pub admin_principals: Vec<String>,
    /// The Cedar policy engine, with `cedar_policies` on top of its
    /// defaults (default false).
    #[serde(deserialize_with = "scalar")]
    pub cedar: bool,
    #[serde(deserialize_with = "list")]
    pub cedar_policies: Vec<String>,
    /// The web UI's sign-in: `none`, `oidc` or `forward`.
    pub mode: SignInMode,
    /// The server's external URL: the provider returns the browser to
    /// `<public_url>/auth/callback`, and with `oidc_end_session` to
    /// `<public_url>/` after "Log out".
    pub public_url: Option<String>,
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    /// None for a public client (PKCE only).
    pub oidc_client_secret: Option<String>,
    /// Default `openid email profile`.
    pub oidc_scopes: Option<String>,
    /// "Log out" also ends the provider's session (default false). Off, a
    /// provider that other apps share keeps their sign-in.
    #[serde(deserialize_with = "scalar")]
    pub oidc_end_session: bool,
    /// The auth proxies whose headers count: CIDRs or addresses.
    #[serde(deserialize_with = "list")]
    pub forward_trusted_proxies: Vec<String>,
    /// The header with the user's email (default `Remote-Email`).
    pub forward_email_header: Option<String>,
    /// The header with the user, without the email header (default
    /// `Remote-User`).
    pub forward_user_header: Option<String>,
    /// The proxy's sign-out, where the UI's "Log out" goes.
    pub forward_logout_url: Option<String>,
}

impl Default for Auth {
    fn default() -> Self {
        Self {
            enabled: false,
            signing_key_file: None,
            users: Vec::new(),
            allowed_issuers: Vec::new(),
            audiences: Vec::new(),
            access_token_timeout: Duration::from_secs(24 * 3600),
            cookie_timeout: Duration::from_secs(5 * 24 * 3600),
            admin_principals: Vec::new(),
            cedar: false,
            cedar_policies: Vec::new(),
            mode: SignInMode::None,
            public_url: None,
            oidc_issuer: None,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_scopes: None,
            oidc_end_session: false,
            forward_trusted_proxies: Vec::new(),
            forward_email_header: None,
            forward_user_header: None,
            forward_logout_url: None,
        }
    }
}

/// `[[auth.users]]`: a user that the server makes at each start when it is
/// missing. A user that exists stays as it is; its missing `privileges`
/// are granted again.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthUser {
    /// The user's email: its name in grants and owners.
    pub email: String,
    /// The display name.
    pub name: Option<String>,
    /// The id of the user's OAuth client (`azp` or `client_id`), for a
    /// machine client that signs in with its own access tokens.
    pub external_id: Option<String>,
    /// Privileges on the metastore, such as `CREATE CATALOG`.
    #[serde(default)]
    pub privileges: Vec<String>,
}

/// `[storage]`: managed storage and its cleanup.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Storage {
    /// Where managed tables go when their catalog and schema have no
    /// storage root.
    pub tables_root: Option<String>,
    /// Where models go when their catalog and schema have no storage root.
    pub models_root: Option<String>,
    /// The local folders under which external Iceberg tables may be.
    #[serde(deserialize_with = "list")]
    pub external_local_roots: Vec<String>,
    /// The cleanup of dropped managed tables, volumes and models (defaults:
    /// `1m`, `30m`, `2h`, `7d` and `1h`).
    #[serde(deserialize_with = "some_duration")]
    pub cleanup_poll_interval: Option<Duration>,
    #[serde(deserialize_with = "some_duration")]
    pub cleanup_attempt_timeout: Option<Duration>,
    #[serde(deserialize_with = "some_duration")]
    pub cleanup_lease_duration: Option<Duration>,
    #[serde(deserialize_with = "some_duration")]
    pub cleanup_initial_delay: Option<Duration>,
    #[serde(deserialize_with = "some_duration")]
    pub cleanup_retry_backoff: Option<Duration>,
}

/// `[tables]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tables {
    /// MANAGED tables (default true).
    #[serde(deserialize_with = "scalar")]
    pub managed: bool,
    /// Managed Delta tables only through the Delta REST API (default false).
    #[serde(deserialize_with = "scalar")]
    pub delta_api_only: bool,
    /// UniForm Iceberg v2 tables without deletion vectors (default false).
    #[serde(deserialize_with = "scalar")]
    pub allow_missing_deletion_vectors: bool,
    /// Writes through the Iceberg REST catalog (default true).
    #[serde(deserialize_with = "scalar")]
    pub iceberg_writes: bool,
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            managed: true,
            delta_api_only: false,
            allow_missing_deletion_vectors: false,
            iceberg_writes: true,
        }
    }
}

/// `[identity]`: the identity that the server signs in to each cloud with,
/// for the storage credentials of external locations and of
/// `[locations.*]`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Identity {
    pub aws: AwsIdentity,
    pub azure: AzureIdentity,
    pub gcp: GcpIdentity,
}

/// `[identity.aws]`: the identity that the server signs in to AWS with; it
/// assumes the roles of storage credentials. Without keys, the credentials
/// of the machine or pod where the server runs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AwsIdentity {
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    /// The region of STS calls.
    pub region: Option<String>,
    /// The ARN of this identity, shown as `unity_catalog_iam_arn` on storage
    /// credentials, for the trust policies of their roles. A role here is
    /// also the one of `aws_iam_role_arn = true`.
    pub arn: Option<String>,
    /// The S3 and STS API of an S3-compatible store, such as MinIO or
    /// RustFS, in place of AWS (http or https). uc's table and volume
    /// commands use it too.
    pub endpoint: Option<String>,
}

/// `[identity.azure]`: the identity that the server signs in to Azure with:
/// a service principal, or a user-assigned managed identity. Without one,
/// the credentials of the machine or pod where the server runs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AzureIdentity {
    /// The service principal's tenant.
    pub directory_id: Option<String>,
    /// The service principal's client ID.
    pub application_id: Option<String>,
    pub client_secret: Option<String>,
    /// The resource ID of a user-assigned managed identity.
    pub managed_identity_id: Option<String>,
}

/// `[identity.gcp]`: the identity that the server signs in to GCP with.
/// Without a key, the credentials of the machine or pod where the server
/// runs (Application Default Credentials).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GcpIdentity {
    /// A service account's JSON key file.
    pub json_key_file: Option<PathBuf>,
    /// The STS and IAM Credentials API of a GCP emulator, such as Floci, in
    /// place of Google's (http or https).
    pub endpoint: Option<String>,
}

/// `[locations.<name>]`: a URL and the identity that reaches it, for the
/// paths under it that no external location covers. The identity has the
/// fields of `uc credential create`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Location {
    /// `s3://bucket`, `abfss://container@account.dfs.core.windows.net` or
    /// `gs://bucket`, with a path or without.
    pub url: Option<String>,
    /// For `s3://`: the bucket's region, which Iceberg clients get; else
    /// the server asks S3.
    pub region: Option<String>,
    /// `true` for the server's own role, `identity.aws.arn`, or the ARN of
    /// a role that the server's identity assumes.
    #[serde(deserialize_with = "own_or")]
    pub aws_iam_role_arn: Option<Option<String>>,
    pub azure_directory_id: Option<String>,
    pub azure_application_id: Option<String>,
    pub azure_client_secret: Option<String>,
    /// `true` for the server's Azure identity, or the resource ID of a
    /// user-assigned managed identity.
    #[serde(deserialize_with = "own_or")]
    pub azure_managed_identity: Option<Option<String>>,
    /// `true` for the server's GCP identity, or the email of a service
    /// account that it impersonates.
    #[serde(deserialize_with = "own_or")]
    pub gcp_service_account: Option<Option<String>>,
    /// A service account's JSON key file.
    pub gcp_service_account_key: Option<PathBuf>,
}

/// `[ui]`: the web UI.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    /// The web UI at `/` (default true).
    #[serde(deserialize_with = "scalar")]
    pub enabled: bool,
    /// Its folder (default `share/unitycatalog-rs/ui` next to the binary's
    /// `bin/` folder).
    pub dir: Option<PathBuf>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: None,
        }
    }
}

/// Where the server keeps the secrets of storage credentials.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretStoreKind {
    /// In the database, each secret sealed with `secrets.key_file`.
    #[default]
    Database,
}

/// `[secrets]`: the store of the secrets of storage credentials, and how
/// long the server keeps them in memory.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Secrets {
    /// The store (default `database`).
    pub store: SecretStoreKind,
    /// The PEM file of the P-256 keys that seal secrets, newest first: the
    /// first seals, each opens what it sealed. Without it, the server makes
    /// a new key in memory at each start, so the secrets cannot be opened
    /// after it stops.
    pub key_file: Option<PathBuf>,
    /// The most secrets, and Azure sign-ins, kept in memory (default 500;
    /// 0 for none).
    #[serde(deserialize_with = "scalar")]
    pub cache_capacity: u64,
    /// How long a secret stays in memory after the store gives it (default
    /// `10m`).
    #[serde(deserialize_with = "duration")]
    pub cache_ttl: Duration,
}

impl Default for Secrets {
    fn default() -> Self {
        Self {
            store: SecretStoreKind::Database,
            key_file: None,
            cache_capacity: 500,
            cache_ttl: Duration::from_secs(10 * 60),
        }
    }
}

/// `[log]`: the log file, as upstream's: rolled at 10 MB, the newest 5
/// kept. Logs also go to stdout.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Log {
    /// The folder of `server.log`; none without it.
    pub dir: Option<PathBuf>,
}

/// `[cli]`: the uc CLI.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Cli {
    /// The server (default `http://localhost:8080`).
    pub server: Option<String>,
    /// The token of requests (default: the sign-in of `uc auth login`).
    pub token: Option<String>,
    /// The uc-server that `uc serve` and `uc dev` run (default: the one
    /// next to uc, else on `PATH`).
    pub server_bin: Option<PathBuf>,
    /// The width of tables (default: `COLUMNS`, else the terminal's width;
    /// at least upstream's minimum, 120).
    #[serde(deserialize_with = "some")]
    pub output_width: Option<usize>,
    /// `uc auth login` in the browser: the provider's OAuth endpoints and
    /// client.
    pub oauth_authorization_url: Option<String>,
    pub oauth_token_url: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    #[serde(deserialize_with = "some")]
    pub oauth_redirect_port: Option<u16>,
}

impl Settings {
    /// The settings of the environment and of `uc.toml`: the file `config`
    /// (`--config`), else `UC_CONFIG`'s, else the first `uc.toml` in the
    /// working directory or a folder above it.
    pub fn load(config: Option<&Path>) -> Result<Self, String> {
        let file = match config {
            Some(path) => Some(path.to_path_buf()),
            None => match std::env::var_os("UC_CONFIG").filter(|p| !p.is_empty()) {
                Some(path) => Some(PathBuf::from(path)),
                None => std::env::current_dir()
                    .ok()
                    .and_then(|dir| find_upwards(&dir, "uc.toml")),
            },
        };
        let mut figment = Figment::new();
        if let Some(path) = &file {
            if !path.is_file() {
                return Err(format!("{}: no such file", path.display()));
            }
            figment = figment.merge(Toml::file(path).search(false));
        }
        let mut settings: Self = figment.merge(variables()).extract().map_err(message)?;
        settings.file = file;
        Ok(settings)
    }

    /// The settings of the TOML text `text` alone, for tests.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        Figment::from(Toml::string(text)).extract().map_err(message)
    }
}

/// The `UC_<GROUP>_<KEY>` variables, as text, without empty ones.
fn variables() -> Env {
    Env::prefixed("UC_")
        .ignore_empty(true)
        .parser(|value: &str| figment2::value::Value::from(value.to_string()))
        .filter_map(|key| {
            let key = key.as_str().to_ascii_lowercase();
            let (group, rest) = key.split_once('_')?;
            if !GROUPS.contains(&group) {
                return None;
            }
            let path = match NESTED.contains(&group) {
                true => format!("{group}.{}", rest.replacen('_', ".", 1)),
                false => format!("{group}.{rest}"),
            };
            Some(path.into())
        })
}

/// The errors of figment2, one per line, each with its setting and where
/// its value came from: `auth.enabled (UC_AUTH_ENABLED): …`, or
/// `/etc/uc/uc.toml: auth.enabld: …`.
fn message(error: figment2::Error) -> String {
    error
        .into_iter()
        .map(|e| {
            let key = e.path.join(".");
            let file = e.metadata.as_ref().and_then(|m| match &m.source {
                Some(figment2::Source::File(path)) => Some(path.display().to_string()),
                _ => None,
            });
            let variable = e
                .metadata
                .as_ref()
                .is_some_and(|m| m.name.contains("environment variable"));
            match (file, key.is_empty()) {
                (Some(file), true) => format!("{file}: {}", e.kind),
                (Some(file), false) => format!("{file}: {key}: {}", e.kind),
                (None, false) if variable => format!("{}: {}", name(&key), e.kind),
                (None, false) => format!("{key}: {}", e.kind),
                (None, true) => e.kind.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A setting's names, for messages: `auth.public_url (UC_AUTH_PUBLIC_URL)`.
pub fn name(key: &str) -> String {
    format!("{key} (UC_{})", key.replace('.', "_").to_ascii_uppercase())
}

/// The message for the invalid value `value` of the setting `key`.
pub fn invalid(key: &str, value: impl std::fmt::Display, reason: impl std::fmt::Display) -> String {
    format!("Invalid value for {}: '{value}'. {reason}", name(key))
}

/// The first `name` in `dir` or a folder above it.
fn find_upwards(dir: &Path, name: &str) -> Option<PathBuf> {
    dir.ancestors()
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// A value of TOML (`8081`, `true`) or of a variable (`"8081"`, `"true"`).
#[derive(Deserialize)]
#[serde(untagged)]
enum Scalar<T> {
    Value(T),
    Text(String),
}

fn scalar<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match Scalar::<T>::deserialize(deserializer)? {
        Scalar::Value(value) => Ok(value),
        Scalar::Text(text) => text
            .trim()
            .parse()
            .map_err(|e| serde::de::Error::custom(format!("'{text}': {e}"))),
    }
}

fn some<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + std::str::FromStr,
    T::Err: std::fmt::Display,
{
    scalar(deserializer).map(Some)
}

/// A duration as humantime has it: `30s`, `1m`, `5d`, `1h 30m`.
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    humantime::parse_duration(text)
        .map_err(|e| format!("'{text}' is not a duration, such as 1m or 5d: {e}"))
}

fn duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    let text = String::deserialize(deserializer)?;
    parse_duration(&text).map_err(serde::de::Error::custom)
}

fn some_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Duration>, D::Error> {
    duration(deserializer).map(Some)
}

/// A list: a TOML array, or text with commas (`a,b`), as a variable has it.
fn list<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum List {
        Items(Vec<String>),
        Text(String),
    }
    Ok(match List::deserialize(deserializer)? {
        List::Items(items) => items,
        List::Text(text) => text
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
    })
}

/// `true` for the server's own identity, or the name of another one, as
/// the optional value of `uc credential create --azure_managed_identity`
/// and `--gcp_service_account`. `false` is none.
#[allow(clippy::option_option)]
fn own_or<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Option<String>>, D::Error> {
    Ok(match Scalar::<bool>::deserialize(deserializer)? {
        Scalar::Value(own) => own.then_some(None),
        Scalar::Text(text) => match text.trim() {
            "true" => Some(None),
            "false" | "" => None,
            name => Some(Some(name.to_string())),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
        [server]
        bind = "127.0.0.1:9000"
        obs_port = 9001

        [auth]
        enabled = true
        allowed_issuers = ["https://a.example", "https://b.example"]
        cookie_timeout = "1d"
        mode = "oidc"

        [locations.analytics]
        url = "s3://analytics"
        region = "us-east-1"

        [locations.models]
        url = "gs://models"
        gcp_service_account = true
    "#;

    /// `uc.toml` in a folder above the working directory, and the
    /// variables `vars`. `Jail`'s closure returns figment2's large error.
    #[allow(clippy::result_large_err)]
    fn load(file: &str, vars: &[(&str, &str)]) -> Result<Settings, String> {
        let mut result = None;
        figment2::Jail::expect_with(|jail| {
            jail.create_file("uc.toml", file)?;
            std::fs::create_dir("work").map_err(|e| e.to_string())?;
            jail.change_dir("work")?;
            for (name, value) in vars {
                jail.set_env(name, value);
            }
            result = Some(Settings::load(None));
            Ok(())
        });
        result.unwrap()
    }

    #[test]
    fn variables_win_over_the_file() {
        let settings = load(
            FILE,
            &[
                ("UC_SERVER_BIND", "0.0.0.0:8080"),
                ("UC_SERVER_OBS_PORT", "9100"),
                ("UC_AUTH_ENABLED", "false"),
                ("UC_AUTH_OIDC_CLIENT_ID", "12345"),
                ("UC_IDENTITY_AWS_SECRET_KEY", "secret"),
                (
                    "UC_IDENTITY_AZURE_MANAGED_IDENTITY_ID",
                    "/subscriptions/s/uc",
                ),
                (
                    "UC_LOCATIONS_ANALYTICS_AWS_IAM_ROLE_ARN",
                    "arn:aws:iam::1:role/uc",
                ),
                (
                    "UC_LOCATIONS_LAKE_URL",
                    "abfss://data@lake.dfs.core.windows.net",
                ),
                ("UC_LOCATIONS_LAKE_AZURE_MANAGED_IDENTITY", "true"),
                (
                    "UC_LOCATIONS_ARCHIVE_GCP_SERVICE_ACCOUNT",
                    "uc@p.iam.gserviceaccount.com",
                ),
                ("UC_DB_PASSWORD", ""),
                ("UC_TEST_POSTGRES_URL", "postgresql://not/a/setting"),
            ],
        )
        .unwrap();
        assert!(settings.file.unwrap().ends_with("uc.toml"));
        assert_eq!(settings.server.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
        assert_eq!(settings.server.obs_port, Some(9100));
        assert!(!settings.auth.enabled);
        assert_eq!(settings.auth.oidc_client_id.as_deref(), Some("12345"));
        assert_eq!(settings.auth.mode, SignInMode::Oidc);
        assert_eq!(settings.auth.cookie_timeout, Duration::from_secs(86_400));
        let identity = &settings.identity;
        assert_eq!(identity.aws.secret_key.as_deref(), Some("secret"));
        assert_eq!(
            identity.azure.managed_identity_id.as_deref(),
            Some("/subscriptions/s/uc")
        );
        let locations = &settings.locations;
        let analytics = &locations["analytics"];
        assert_eq!(analytics.url.as_deref(), Some("s3://analytics"));
        assert_eq!(
            analytics.aws_iam_role_arn,
            Some(Some("arn:aws:iam::1:role/uc".into()))
        );
        // `true` is the server's own identity, as in the file; other text
        // names an identity.
        assert_eq!(locations["lake"].azure_managed_identity, Some(None));
        assert_eq!(locations["models"].gcp_service_account, Some(None));
        assert_eq!(
            locations["archive"].gcp_service_account,
            Some(Some("uc@p.iam.gserviceaccount.com".into()))
        );
        // An empty variable counts as unset.
        assert_eq!(settings.db.password, None);
    }

    #[test]
    fn lists_come_from_arrays_or_commas() {
        let settings = load(FILE, &[]).unwrap();
        assert_eq!(
            settings.auth.allowed_issuers,
            ["https://a.example", "https://b.example"]
        );
        let settings = load(FILE, &[("UC_AUTH_ALLOWED_ISSUERS", " x, y ,")]).unwrap();
        assert_eq!(settings.auth.allowed_issuers, ["x", "y"]);
    }

    #[test]
    fn defaults_without_a_file() {
        let settings = Settings::from_toml("").unwrap();
        assert_eq!(settings.server.data_dir, PathBuf::from(".uc"));
        assert_eq!(settings.server.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
        assert_eq!(settings.server.request_timeout, Duration::from_secs(10));
        assert!(settings.tables.managed && settings.tables.iceberg_writes);
        assert!(settings.ui.enabled);
        assert_eq!(settings.auth.mode, SignInMode::None);
        assert_eq!(settings.log.dir, None);
    }

    #[test]
    fn errors_name_the_setting_and_its_source() {
        let error = load(FILE, &[("UC_AUTH_ENABLED", "maybe")]).unwrap_err();
        assert!(
            error.starts_with("auth.enabled (UC_AUTH_ENABLED): 'maybe'"),
            "{error}"
        );
        let error = load(FILE, &[("UC_AUTH_ENABLD", "true")]).unwrap_err();
        assert!(
            error.starts_with("auth.enabld (UC_AUTH_ENABLD): unknown field"),
            "{error}"
        );
        let error = load("[auth]\nenabld = true", &[]).unwrap_err();
        assert!(
            error.contains("uc.toml: auth.enabld: unknown field"),
            "{error}"
        );
        let error = Settings::from_toml("[nope]\nx = 1").unwrap_err();
        assert!(error.contains("nope: unknown field"), "{error}");
        let error = Settings::from_toml("[auth]\ncookie_timeout = \"P5D\"").unwrap_err();
        assert!(error.contains("is not a duration"), "{error}");
        let error = Settings::from_toml("[auth]\nmode = \"saml\"").unwrap_err();
        assert!(
            error.starts_with("auth.mode: unknown variant: found `saml`"),
            "{error}"
        );
    }

    #[test]
    fn users_come_from_the_file() {
        let settings = Settings::from_toml(
            r#"
            [secrets]
            key_file = "/etc/uc/keys/secrets_key.pem"
            cache_ttl = "5m"

            [auth]
            signing_key_file = "/etc/uc/keys/private_key.pem"

            [[auth.users]]
            email = "alice@example.com"
            name = "Alice"
            privileges = ["CREATE CATALOG"]

            [[auth.users]]
            email = "etl@example.com"
            external_id = "etl-client"
            "#,
        )
        .unwrap();
        assert_eq!(
            settings.auth.signing_key_file,
            Some(PathBuf::from("/etc/uc/keys/private_key.pem"))
        );
        assert_eq!(
            settings.secrets.key_file,
            Some(PathBuf::from("/etc/uc/keys/secrets_key.pem"))
        );
        assert_eq!(settings.secrets.store, SecretStoreKind::Database);
        assert_eq!(settings.secrets.cache_capacity, 500);
        assert_eq!(settings.secrets.cache_ttl, Duration::from_secs(300));
        assert_eq!(
            settings.auth.users,
            [
                AuthUser {
                    email: "alice@example.com".into(),
                    name: Some("Alice".into()),
                    external_id: None,
                    privileges: vec!["CREATE CATALOG".into()],
                },
                AuthUser {
                    email: "etl@example.com".into(),
                    name: None,
                    external_id: Some("etl-client".into()),
                    privileges: Vec::new(),
                },
            ]
        );
        let error = Settings::from_toml("[[auth.users]]\nname = \"Bob\"").unwrap_err();
        assert!(error.contains("missing field `email`"), "{error}");
    }

    #[test]
    fn names_say_where_a_setting_lives() {
        assert_eq!(
            name("auth.public_url"),
            "auth.public_url (UC_AUTH_PUBLIC_URL)"
        );
        assert_eq!(
            invalid("locations.analytics.region", "x", "Expected a region."),
            "Invalid value for locations.analytics.region (UC_LOCATIONS_ANALYTICS_REGION): 'x'. \
             Expected a region."
        );
    }

    #[test]
    fn a_given_file_must_exist() {
        let missing = std::path::Path::new("/nonexistent/uc.toml");
        assert_eq!(
            Settings::load(Some(missing)).unwrap_err(),
            "/nonexistent/uc.toml: no such file"
        );
    }
}
