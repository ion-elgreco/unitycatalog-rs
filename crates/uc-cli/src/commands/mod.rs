//! `uc <entity> <operation> [options]`, as upstream's `bin/uc`
//! (`UnityCatalogCli`): its entities, operations, options, requests and
//! output, which stay as upstream's so that its commands still work. This
//! CLI's own entities (token, group, admin, iceberg) take the same form.
//!
//! Each entity's module has its operations, as clap parses them, and runs
//! them: as upstream's `*Cli` classes, it calls the API ([`Client`]) and
//! returns the text to print ([`crate::output`]). The operations on data
//! (`table read`, `volume write`, `function call`, ...) run on this machine
//! ([`crate::data`]).

pub mod admin;
pub mod auth;
pub mod catalog;
pub mod columns;
pub mod credential;
pub mod external_location;
pub mod function;
pub mod group;
pub mod iceberg;
pub mod metastore;
pub mod model_version;
pub mod options;
pub mod permission;
pub mod registered_model;
pub mod schema;
pub mod table;
pub mod token;
pub mod user;
pub mod volume;

use std::cell::RefCell;

use clap::Subcommand;
use serde_json::Value;
use uc_client::{API_BASE, Client};
use uc_common::settings::Settings;

use crate::args::Global;
use crate::config;
use crate::output::models::{Model, shape, shape_list};
use crate::output::{self, Format};
use crate::sign_in::{self, SignIns};

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum Entity {
    /// Manage catalogs
    #[command(subcommand)]
    Catalog(catalog::CatalogOp),
    /// Manage schemas
    #[command(subcommand)]
    Schema(schema::SchemaOp),
    /// Manage tables; create, read and write Delta tables
    #[command(subcommand)]
    Table(table::TableOp),
    /// Manage volumes; read and write their files
    #[command(subcommand)]
    Volume(volume::VolumeOp),
    /// Manage functions; call Python functions
    #[command(subcommand)]
    Function(function::FunctionOp),
    /// Manage registered models
    #[command(subcommand)]
    RegisteredModel(registered_model::RegisteredModelOp),
    /// Manage model versions
    #[command(subcommand)]
    ModelVersion(model_version::ModelVersionOp),
    /// Grant and revoke privileges
    #[command(subcommand)]
    Permission(permission::PermissionOp),
    /// Show the metastore
    #[command(subcommand)]
    Metastore(metastore::MetastoreOp),
    /// Manage storage credentials
    #[command(subcommand)]
    Credential(credential::CredentialOp),
    /// Manage external locations
    #[command(subcommand)]
    ExternalLocation(external_location::ExternalLocationOp),
    /// Manage users
    #[command(subcommand)]
    User(user::UserOp),
    /// Sign in and out
    #[command(subcommand)]
    Auth(auth::AuthOp),
    /// Manage personal access tokens
    #[command(subcommand)]
    Token(token::TokenOp),
    /// Manage groups
    #[command(subcommand)]
    Group(group::GroupOp),
    /// Back up, restore and audit the server
    #[command(subcommand)]
    Admin(admin::AdminOp),
    /// Read the Iceberg REST catalog
    #[command(subcommand)]
    Iceberg(iceberg::IcebergOp),
}

/// What every command needs: the client and the output format.
pub struct Context<'a> {
    pub client: &'a Client,
    pub format: Format,
    /// The S3-compatible store of `aws.endpoint`, for table and volume data.
    pub s3: Option<crate::data::S3Store>,
    /// The option that gives a list's next page, which upstream does not
    /// print: it goes to standard error, after the list.
    next_page: RefCell<Option<String>>,
}

/// Upstream `CliException`: a message, printed as is.
pub type Result<T> = anyhow::Result<T>;

/// The error of a call to the server as upstream's `ApiException`: a
/// refused call is `{operation} call failed with: {status} - {body}`; a
/// call without an answer is `{operation} call failed: {error}`.
pub fn upstream(operation: &'static str) -> impl FnOnce(uc_client::Error) -> anyhow::Error {
    move |e| {
        let message = match &e {
            uc_client::Error::Status { status, body } => {
                format!("{operation} call failed with: {status} - {body}")
            }
            e => format!("{operation} call failed: {e}"),
        };
        anyhow::Error::new(CallFailed {
            message,
            status: e.status(),
        })
    }
}

/// A failed call: upstream's message, and the HTTP status of a refused
/// call.
#[derive(Debug)]
struct CallFailed {
    message: String,
    status: Option<u16>,
}

impl std::fmt::Display for CallFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CallFailed {}

/// Whether the server refused a call of `e` for want of a sign-in (401).
fn needs_sign_in(e: &anyhow::Error) -> bool {
    e.chain().any(|cause| {
        let status = match cause.downcast_ref::<CallFailed>() {
            Some(call) => call.status,
            None => cause
                .downcast_ref::<uc_client::Error>()
                .and_then(uc_client::Error::status),
        };
        status == Some(401)
    })
}

impl Context<'_> {
    pub fn json(&self, value: &Value) -> String {
        self.format.json(value)
    }

    fn shaped(&self, model: &Model, value: &Value) -> String {
        self.json(&shape(model, value))
    }

    fn shaped_list(&self, model: &Model, value: &Value, key: &str) -> String {
        self.page_token(value["next_page_token"].as_str());
        self.json(&shape_list(model, value, key))
    }

    /// Note the token of a list's next page; none or empty on the last.
    pub fn page_token(&self, token: Option<&str>) {
        if let Some(token) = token.filter(|t| !t.is_empty()) {
            *self.next_page.borrow_mut() = Some(format!("--page_token {}", shell_word(token)));
        }
    }

    /// Note the `--start_index` of a SCIM list's next page, after a full
    /// page: `items` from `start`, of the `count` asked for. As upstream's,
    /// the server's `totalResults` is the size of the page.
    pub fn start_index(&self, start: i32, count: i32, items: usize) {
        if items > 0 && items as i64 >= i64::from(count) {
            let next = i64::from(start.max(1)) + items as i64;
            *self.next_page.borrow_mut() = Some(format!("--start_index {next}"));
        }
    }
}

/// `text` as one word of a shell command: as is when it is only letters,
/// digits and `-_.,:/=+@%`, else in single quotes.
pub(crate) fn shell_word(text: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "-_.,:/=+@%".contains(c);
    if text.chars().all(plain) {
        text.to_string()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

/// Whether the result prints as the command made it, never as a table:
/// rows, files, a function's output and backup snapshots.
fn prints_as_is(entity: &Entity) -> bool {
    matches!(
        entity,
        Entity::Table(table::TableOp::Read { .. })
            | Entity::Volume(volume::VolumeOp::Read { .. })
            | Entity::Function(function::FunctionOp::Call { .. })
            | Entity::Admin(admin::AdminOp::Backup)
    )
}

/// Whether the command prints no result: `auth logout`, whose message goes
/// to stderr. Deletes print a table without rows, as upstream.
fn prints_nothing(entity: &Entity) -> bool {
    matches!(entity, Entity::Auth(auth::AuthOp::Logout))
}

/// Run the command of `entity`; the exit code.
pub async fn run(global: &Global, settings: &Settings, entity: Entity) -> i32 {
    let origin = match config::origin(&config::server(settings, global.server.as_deref())) {
        Ok(origin) => origin,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    let client = match Client::new(format!("{origin}{API_BASE}/")) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    let token = config::token(
        settings,
        global.auth_token.as_deref(),
        &origin,
        SignIns::of_user().as_ref(),
    );
    let client = match &token {
        Some((token, _)) => client.with_bearer(token.clone()),
        None => client,
    };
    let c = Context {
        client: &client,
        format: global.output,
        s3: crate::data::S3Store::from_settings(&settings.aws),
        next_page: RefCell::new(None),
    };
    let as_is = prints_as_is(&entity);
    let quiet = prints_nothing(&entity);
    // `auth` signs in itself.
    let signs_in = matches!(entity, Entity::Auth(_));
    let result = match entity {
        Entity::Catalog(op) => catalog::run(&c, op).await,
        Entity::Schema(op) => schema::run(&c, op).await,
        Entity::Table(op) => table::run(&c, op).await,
        Entity::Volume(op) => volume::run(&c, op).await,
        Entity::Function(op) => function::run(&c, op).await,
        Entity::RegisteredModel(op) => registered_model::run(&c, op).await,
        Entity::ModelVersion(op) => model_version::run(&c, op).await,
        Entity::Permission(op) => permission::run(&c, op).await,
        Entity::Metastore(op) => metastore::run(&c, op).await,
        Entity::Credential(op) => credential::run(&c, op).await,
        Entity::ExternalLocation(op) => external_location::run(&c, op).await,
        Entity::User(op) => user::run(&c, op).await,
        Entity::Auth(op) => auth::run(&c, op, settings, &origin).await,
        Entity::Token(op) => token::run(&c, op).await,
        Entity::Group(op) => group::run(&c, op).await,
        Entity::Admin(op) => admin::run(&c, op).await,
        Entity::Iceberg(op) => iceberg::run(&c, op).await,
    };
    match result {
        Ok(text) => {
            if !quiet {
                let shown = if as_is {
                    text
                } else {
                    output::print(c.format, settings.cli.output_width, &text)
                };
                println!("{shown}");
            }
            if let Some(option) = c.next_page.take() {
                eprintln!("Next page: {option}");
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {e:#}");
            if !signs_in && needs_sign_in(&e) {
                let source = token.map(|(_, source)| source);
                let login = sign_in::login_command(global);
                eprintln!("{}", sign_in::advice(&origin, source, &login));
            }
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use clap::error::ErrorKind;

    use super::Entity;
    use super::admin::AdminOp;
    use super::catalog::CatalogOp;
    use super::function::FunctionOp;
    use super::model_version::ModelVersionOp;
    use super::permission::PermissionOp;
    use uc_api::{DataSourceFormat, Privilege, TableType};
    use uc_common::SecurableType;

    use super::table::TableOp;
    use crate::args::{Cli, Command};
    use crate::output::Format;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("uc").chain(args.iter().copied()))
    }

    fn entity(args: &[&str]) -> Entity {
        match parse(args).unwrap().command {
            Some(Command::Entity(entity)) => *entity,
            other => panic!("{other:?}"),
        }
    }

    /// Why clap stops at `args`.
    fn stop(args: &[&str]) -> ErrorKind {
        parse(args).unwrap_err().kind()
    }

    #[test]
    fn takes_upstream_syntax() {
        // Common options before or after the command; `--option=value`.
        let args = [
            "--server",
            "http://h",
            "catalog",
            "create",
            "--name",
            "c",
            "--comment=hi there",
            "--properties",
            r#"{"k": 1}"#,
            "--output",
            "json",
        ];
        let global = parse(&args).unwrap().global;
        assert_eq!(global.server.as_deref(), Some("http://h"));
        assert_eq!(global.output, Format::Json);
        let Entity::Catalog(CatalogOp::Create {
            name,
            comment,
            properties,
            storage_root: None,
        }) = entity(&args)
        else {
            panic!();
        };
        assert_eq!((name.as_str(), comment.as_deref()), ("c", Some("hi there")));
        assert_eq!(properties.unwrap()["k"], "1");
        assert_eq!(
            parse(&["schema", "list", "--catalog", "c"])
                .unwrap()
                .global
                .output,
            Format::Table
        );

        // `--force` with or without a value.
        for (args, force) in [
            (&["--force"][..], Some(true)),
            (&["--force", "true"], Some(true)),
            (&["--force", "false"], Some(false)),
            (&[], None),
        ] {
            let mut all = vec!["catalog", "delete", "--name", "c"];
            all.extend(args);
            let Entity::Catalog(CatalogOp::Delete { force: given, .. }) = entity(&all) else {
                panic!();
            };
            assert_eq!(given, force, "{args:?}");
        }

        // `--version` is a model version's number in model_version.
        let Entity::ModelVersion(ModelVersionOp::Get { version, .. }) = entity(&[
            "model_version",
            "get",
            "--full_name",
            "a.b.c",
            "--version",
            "2",
        ]) else {
            panic!();
        };
        assert_eq!(version, 2);
        assert_eq!(stop(&["--version"]), ErrorKind::DisplayVersion);
        assert_eq!(stop(&["-v"]), ErrorKind::DisplayVersion);

        // Values in any case; negative function arguments.
        let Entity::Table(TableOp::Create(create)) = entity(&[
            "table",
            "create",
            "--full_name",
            "c.s.t",
            "--columns",
            "id INT",
            "--table_type",
            "managed",
        ]) else {
            panic!();
        };
        assert_eq!(create.full_name.schema, "s");
        assert_eq!(
            (create.table_type, create.format),
            (TableType::Managed, DataSourceFormat::Delta)
        );
        let Entity::Function(FunctionOp::Call { input_params, .. }) = entity(&[
            "function",
            "call",
            "--full_name",
            "a.b.f",
            "--input_params",
            "-1,2",
        ]) else {
            panic!();
        };
        assert_eq!(input_params, "-1,2");
        let Entity::Permission(PermissionOp::Create {
            securable_type,
            privilege,
            ..
        }) = entity(&[
            "permission",
            "create",
            "--securable_type",
            "Registered_Model",
            "--name",
            "a.b.m",
            "--principal",
            "p",
            "--privilege",
            "use catalog",
        ])
        else {
            panic!();
        };
        assert_eq!(
            (securable_type, privilege),
            (SecurableType::RegisteredModel, Privilege::UseCatalog)
        );
        // `-` is standard input.
        let Entity::Admin(AdminOp::Restore { path }) = entity(&["admin", "restore", "--path", "-"])
        else {
            panic!();
        };
        assert_eq!(path, "-");
    }

    #[test]
    fn refuses_bad_usage() {
        assert_eq!(stop(&["nope"]), ErrorKind::InvalidSubcommand);
        assert_eq!(
            stop(&["catalog"]),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        assert_eq!(
            stop(&["catalog", "get"]),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            stop(&["catalog", "get", "--name", "a", "--url", "b"]),
            ErrorKind::UnknownArgument
        );
        assert_eq!(stop(&["catalog", "get", "--name"]), ErrorKind::InvalidValue);
        // An update needs a change.
        assert_eq!(
            stop(&["catalog", "update", "--name", "c"]),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(
            stop(&["volume", "create", "--full_name", "a.b"]),
            ErrorKind::ValueValidation
        );
        assert_eq!(
            stop(&["catalog", "create", "--name", "c", "--properties", "[1]"]),
            ErrorKind::ValueValidation
        );
        assert_eq!(
            stop(&[
                "table",
                "create",
                "--full_name",
                "a.b.c",
                "--columns",
                "id INT",
                "--table_type",
                "VIEW"
            ]),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            stop(&["catalog", "list", "--output", "xml"]),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn page_tokens_print_as_shell_words() {
        assert_eq!(super::shell_word("sales_2024"), "sales_2024");
        assert_eq!(super::shell_word("eyJ0IjoxfQ=="), "eyJ0IjoxfQ==");
        assert_eq!(super::shell_word("a&b"), "'a&b'");
        assert_eq!(super::shell_word("it's"), r"'it'\''s'");
    }
}
