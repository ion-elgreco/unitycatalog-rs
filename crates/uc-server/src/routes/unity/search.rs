//! Search: the catalogs, schemas, tables, volumes, functions and models
//! whose name or comment holds some words. Not in upstream.

use crate::http::extract::{Json, Query};
use axum::extract::State;
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::SearchObjectsResponse;
use uc_common::{Error, PageToken, Pagination, SecurableType};
use uc_metastore::SearchHit;

use crate::{
    authn::principal::Principal,
    authz::{Access, rules},
    http::error::ApiResult,
    server::state::AppState,
};

/// Reads of the index for one page at most: the caller may not see all
/// the objects that a read returns.
const MAX_READS: usize = 10;

#[derive(Debug, Deserialize)]
pub struct SearchObjectsParams {
    pub query: String,
    pub securable_type: Option<SecurableType>,
    #[serde(flatten)]
    pub page: Pagination,
}

#[utoipa::path(
    get, path = "/search",
    params(
        ("query"          = String, Query, description = "The words to find in names and comments"),
        ("securable_type" = Option<String>, Query, description = "Only objects of this type"),
        ("max_results"    = Option<u32>, Query),
        ("page_token"     = Option<String>, Query),
    ),
    responses((status = 200, body = SearchObjectsResponse)),
    tag = "Search",
)]
pub async fn search_objects(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<SearchObjectsParams>,
) -> ApiResult<Json<SearchObjectsResponse>> {
    let size = params.page.page_size()?;
    let start = match params.page.page_token.as_deref() {
        None | Some("") => 0,
        Some(token) => token
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)
            .ok_or_else(|| Error::invalid_argument(format!("Invalid page token: {token}")))?,
    };
    let access = Access::load(&state, &principal).await?;
    let mut results = Vec::new();
    // The offset of the next result in the index's order, if any.
    let mut next = Some(start);
    for _ in 0..MAX_READS {
        let Some(offset) = next.filter(|_| (results.len() as i64) < size) else {
            break;
        };
        // One more than a page: when it comes, there are more.
        let found = state
            .metastore
            .search()
            .search(&params.query, params.securable_type, offset, size + 1)
            .await?;
        next = None;
        for (i, hit) in found.into_iter().enumerate() {
            if results.len() as i64 == size || i as i64 == size {
                next = Some(offset + i as i64);
                break;
            }
            if visible(&access, &hit).await {
                results.push(hit.result);
            }
        }
    }
    Ok(Json(SearchObjectsResponse {
        results,
        next_page_token: next.map(|n| PageToken::new(n.to_string())),
    }))
}

/// Whether the caller may see a result: the rule of its type's `get`.
async fn visible(access: &Access, hit: &SearchHit) -> bool {
    if !access.enabled() {
        return true;
    }
    let l = &hit.lineage;
    match hit.result.securable_type {
        SecurableType::Catalog => rules::get_catalog(access, l).await,
        SecurableType::Schema => rules::get_schema(access, l).await,
        SecurableType::Table => rules::get_table(access, l).await,
        SecurableType::Volume => rules::get_volume(access, l).await,
        SecurableType::Function => rules::get_function(access, l).await,
        _ => rules::get_model(access, l).await,
    }
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(search_objects))
}
