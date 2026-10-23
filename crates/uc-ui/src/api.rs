//! The UI's calls to the server, through `uc-client`, on the UI's own
//! origin: uc-server serves the UI and the API, and `uc web` sends the API
//! calls on to a server.
//!
//! The functions here return whole lists: the server answers at most 100
//! items a page, with a token for the next page.

use std::cmp::Reverse;
use std::future::Future;

use uc_api::{
    CatalogInfo, FunctionInfo, ModelVersionInfo, PatInfo, PermissionsChange, PermissionsList,
    Privilege, PrivilegeAssignment, RegisteredModelInfo, SchemaInfo, TableInfo, UpdatePermissions,
    VolumeInfo,
};
use uc_client::{API_BASE, Client, Page};
use uc_common::{PageToken, SecurableType};

pub use uc_client::{Error, Result};

/// A client of the API on this page's origin.
pub fn client() -> Client {
    let origin = web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default();
    Client::new(format!("{origin}{API_BASE}/")).expect("the page's origin is a URL")
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

/// The `message` of the server's JSON answer: none without an answer, or
/// for a proxy's error page.
fn server_message(e: &Error) -> Option<String> {
    let Error::Status { body, .. } = e else {
        return None;
    };
    let body: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = body.get("message")?.as_str()?;
    (!message.is_empty()).then(|| message.to_string())
}

/// Every item of a list: `page` gets one page, and the token of the next.
async fn all<T, F>(mut page: impl FnMut(Page) -> F) -> Result<Vec<T>>
where
    F: Future<Output = Result<(Vec<T>, Option<PageToken>)>>,
{
    let mut items = Vec::new();
    let mut next = Page::default();
    loop {
        let (batch, token) = page(next.clone()).await?;
        items.extend(batch);
        match token {
            Some(PageToken(token)) if !token.is_empty() => next.page_token = Some(token),
            _ => return Ok(items),
        }
    }
}

pub async fn catalogs() -> Result<Vec<CatalogInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client.list_catalogs(&page).await?;
        Ok((list.catalogs, list.next_page_token))
    })
    .await
}

pub async fn schemas(catalog: &str) -> Result<Vec<SchemaInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client.list_schemas(catalog, &page).await?;
        Ok((list.schemas, list.next_page_token))
    })
    .await
}

pub async fn tables(catalog: &str, schema: &str) -> Result<Vec<TableInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client.list_tables(catalog, schema, &page).await?;
        Ok((list.tables, list.next_page_token))
    })
    .await
}

pub async fn volumes(catalog: &str, schema: &str) -> Result<Vec<VolumeInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client.list_volumes(catalog, schema, &page).await?;
        Ok((list.volumes, list.next_page_token))
    })
    .await
}

pub async fn functions(catalog: &str, schema: &str) -> Result<Vec<FunctionInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client.list_functions(catalog, schema, &page).await?;
        Ok((list.functions, list.next_page_token))
    })
    .await
}

pub async fn models(catalog: &str, schema: &str) -> Result<Vec<RegisteredModelInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client
            .list_registered_models(Some(catalog), Some(schema), &page)
            .await?;
        Ok((list.registered_models, list.next_page_token))
    })
    .await
}

/// The versions of the registered model `full_name`.
pub async fn model_versions(full_name: &str) -> Result<Vec<ModelVersionInfo>> {
    let client = &client();
    all(move |page| async move {
        let list = client.list_model_versions(full_name, &page).await?;
        Ok((list.model_versions, list.next_page_token))
    })
    .await
}

/// The grants on a securable, by principal, as the server lets the caller
/// see them: all of them for the admin and the owners, else the caller's
/// own.
pub async fn permissions(kind: SecurableType, full_name: &str) -> Result<Vec<PrivilegeAssignment>> {
    let list = client().get_permissions(kind, full_name, None).await?;
    Ok(list.privilege_assignments)
}

/// Grant and revoke privileges of one principal: the grants afterwards.
pub async fn change_permissions(
    kind: SecurableType,
    full_name: &str,
    principal: String,
    add: Vec<Privilege>,
    remove: Vec<Privilege>,
) -> Result<PermissionsList> {
    let request = UpdatePermissions {
        changes: vec![PermissionsChange {
            principal,
            add,
            remove,
        }],
    };
    client().update_permissions(kind, full_name, &request).await
}

/// The signed-in user's access tokens, the newest first.
pub async fn tokens() -> Result<Vec<PatInfo>> {
    let client = &client();
    let mut tokens = all(move |page| async move {
        let list = client.list_pats(&page).await?;
        Ok((list.tokens, list.next_page_token))
    })
    .await?;
    tokens.sort_by_key(|token| Reverse(token.created_at));
    Ok(tokens)
}
