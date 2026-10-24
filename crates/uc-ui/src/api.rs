//! The UI's calls to the server, through `uc-client`, on the UI's own
//! origin: uc-server serves the UI and the API, and `uc web` sends the API
//! calls on to a server.
//!
//! The functions here return whole lists: the server answers at most 100
//! items a page, with a token for the next page.

use std::cmp::Reverse;

use futures::TryStreamExt;
use uc_client::Client;
use uc_client::models::catalogs::v1::CatalogInfo;
use uc_client::models::functions::v1::FunctionInfo;
use uc_client::models::models::v1::{ModelVersionInfo, RegisteredModelInfo};
use uc_client::models::schemas::v1::SchemaInfo;
use uc_client::models::tables::v1::TableInfo;
use uc_client::models::tokens::v1::PatInfo;
use uc_client::models::volumes::v1::VolumeInfo;

pub use uc_client::error::{Error, Result};

/// A client of the API on this page's origin.
pub fn client() -> Client {
    let origin = web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default();
    let root = format!("{origin}/")
        .parse()
        .expect("the page's origin is a URL");
    Client::new_unauthenticated(root)
}

/// What the UI shows for a failed call: the server's message, else
/// `action`'s failure, such as "Failed to create catalog", as upstream. A
/// 401 or 403 can also mean that the session ended, so the UI asks the
/// server again who is signed in (see [`crate::auth`]).
pub fn failure(e: &Error, action: &str) -> String {
    if matches!(e.status(), Some(401 | 403)) {
        crate::auth::check_again();
    }
    server_message(e).unwrap_or_else(|| action.to_string())
}

/// The message of the server's JSON error: none without an answer, or for
/// a proxy's error page.
pub fn server_message(e: &Error) -> Option<String> {
    let Error::Api(answer) = e else {
        return None;
    };
    // The server's errors name their code. The client takes the text of
    // other answers, such as a proxy's error page, as their message.
    if answer.code.is_none() || answer.message.is_empty() {
        return None;
    }
    Some(answer.message.clone())
}

pub async fn catalogs() -> Result<Vec<CatalogInfo>> {
    client().list_catalogs().into_stream().try_collect().await
}

pub async fn schemas(catalog: &str) -> Result<Vec<SchemaInfo>> {
    client()
        .list_schemas(catalog)
        .into_stream()
        .try_collect()
        .await
}

pub async fn tables(catalog: &str, schema: &str) -> Result<Vec<TableInfo>> {
    client()
        .list_tables(catalog, schema)
        .into_stream()
        .try_collect()
        .await
}

pub async fn volumes(catalog: &str, schema: &str) -> Result<Vec<VolumeInfo>> {
    client()
        .list_volumes(catalog, schema)
        .into_stream()
        .try_collect()
        .await
}

pub async fn functions(catalog: &str, schema: &str) -> Result<Vec<FunctionInfo>> {
    client()
        .list_functions(catalog, schema)
        .into_stream()
        .try_collect()
        .await
}

pub async fn models(catalog: &str, schema: &str) -> Result<Vec<RegisteredModelInfo>> {
    client()
        .list_registered_models()
        .with_catalog_name(catalog.to_string())
        .with_schema_name(schema.to_string())
        .into_stream()
        .try_collect()
        .await
}

/// The versions of the registered model `full_name`.
pub async fn model_versions(full_name: &str) -> Result<Vec<ModelVersionInfo>> {
    client()
        .list_model_versions(full_name)
        .into_stream()
        .try_collect()
        .await
}

/// The signed-in user's access tokens, the newest first.
pub async fn tokens() -> Result<Vec<PatInfo>> {
    let mut tokens: Vec<PatInfo> = client().list_tokens().into_stream().try_collect().await?;
    tokens.sort_by_key(|token| Reverse(token.created_at));
    Ok(tokens)
}
