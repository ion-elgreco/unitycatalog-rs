//! Search: the catalogs, schemas, tables, volumes, functions and models
//! whose name or comment holds some words. Not in upstream.

use std::collections::HashMap;

use crate::http::extract::{Json, Query};
use axum::extract::State;
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::{SearchObjectsResponse, SearchResult};
use uc_common::{Error, PageToken, Pagination, SecurableType, schema_full_name};
use uc_metastore::Lineage;

use crate::{
    authn::principal::Principal,
    authz::{self, Access, rules},
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
    let access = Access::load(&state, &principal.username).await?;
    let mut visible = Visible::new(&state, &access);
    let mut results = Vec::new();
    // The offset of the next result in the index's order, if any.
    let mut next = Some(start);
    let mut reads = 0;
    while let Some(offset) = next.filter(|_| (results.len() as i64) < size && reads < MAX_READS) {
        reads += 1;
        let found = state
            .metastore
            .search()
            .search(&params.query, params.securable_type, offset, size)
            .await?;
        next = (found.len() as i64 == size).then_some(offset + size);
        for (i, result) in found.into_iter().enumerate() {
            if results.len() as i64 == size {
                next = Some(offset + i as i64);
                break;
            }
            if visible.allows(&result).await? {
                results.push(result);
            }
        }
    }
    Ok(Json(SearchObjectsResponse {
        results,
        next_page_token: next.map(|n| PageToken::new(n.to_string())),
    }))
}

/// Whether the caller may see a result: the rule of its type's `get`. Each
/// catalog's and schema's lineage is looked up once.
struct Visible<'a> {
    state: &'a AppState,
    access: &'a Access,
    parents: HashMap<(SecurableType, String), Lineage>,
}

impl<'a> Visible<'a> {
    fn new(state: &'a AppState, access: &'a Access) -> Self {
        Self {
            state,
            access,
            parents: HashMap::new(),
        }
    }

    async fn parent(&mut self, kind: SecurableType, name: String) -> ApiResult<&Lineage> {
        let key = (kind, name);
        if !self.parents.contains_key(&key) {
            let lineage = authz::lineage(self.state, kind, &key.1).await?;
            self.parents.insert(key.clone(), lineage);
        }
        Ok(&self.parents[&key])
    }

    async fn allows(&mut self, result: &SearchResult) -> ApiResult<bool> {
        if !self.access.enabled() {
            return Ok(true);
        }
        let kind = result.securable_type;
        let object = authz::node(kind, Some(&result.id), result.owner.as_deref());
        let (lineage, rule): (Lineage, fn(&Access, &Lineage) -> bool) = match kind {
            SecurableType::Catalog => (
                Lineage {
                    catalog: object.clone(),
                    schema: None,
                    object,
                },
                rules::get_catalog,
            ),
            SecurableType::Schema => {
                let catalog = self
                    .parent(SecurableType::Catalog, result.catalog_name.clone())
                    .await?;
                (
                    Lineage {
                        catalog: catalog.catalog.clone(),
                        schema: object.clone(),
                        object,
                    },
                    rules::get_schema,
                )
            }
            _ => {
                let schema = schema_full_name(
                    &result.catalog_name,
                    result.schema_name.as_deref().unwrap_or_default(),
                );
                let schema = self.parent(SecurableType::Schema, schema).await?;
                let rule = match kind {
                    SecurableType::Table => rules::get_table,
                    SecurableType::Volume => rules::get_volume,
                    SecurableType::Function => rules::get_function,
                    _ => rules::get_model,
                };
                (
                    Lineage {
                        object,
                        ..schema.clone()
                    },
                    rule,
                )
            }
        };
        Ok(rule(self.access, &lineage))
    }
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(search_objects))
}
