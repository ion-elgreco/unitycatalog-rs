//! SCIM 2.0 under the control plane: upstream's Users and Me, and local
//! Groups, Bulk and discovery endpoints. Lists take full SCIM filters
//! ([`crate::routes::control::scim_filter`]).

use crate::http::extract::{Json, Path, Query};
use axum::{
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::scim::schema_uri;
use uc_api::{
    ListScimGroupsResponse, ListScimUsersResponse, ScimEmail, ScimGroup, ScimGroupMember, ScimMeta,
    ScimPatchOp, ScimRole, ScimUser,
};
use uc_common::Error;

use crate::{
    authn::principal::Principal,
    authz,
    http::error::ApiResult,
    routes::control::scim_filter::{self, Filter},
    server::state::AppState,
};

/// SCIM list parameters: `filter`, `startIndex` and `count`.
#[derive(Debug, Default, Deserialize)]
pub struct ScimListParams {
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(rename = "startIndex", default)]
    pub start_index: Option<i64>,
    #[serde(default)]
    pub count: Option<i64>,
}

impl ScimListParams {
    /// The parsed filter; a malformed one is SCIM `invalidFilter`.
    fn filter(&self) -> ScimResult<Option<Filter>> {
        match self.filter.as_deref().filter(|f| !f.is_empty()) {
            None => Ok(None),
            Some(f) => scim_filter::parse(f)
                .map(Some)
                .map_err(|e| ScimError::invalid_filter(e).into()),
        }
    }

    /// The page of resources that match the filter, as upstream: the
    /// filter is evaluated on each resource, then `count` (default 50)
    /// resources from `startIndex` (from 1).
    fn page<T: Serialize>(&self, items: Vec<T>) -> ScimResult<Vec<T>> {
        let filter = self.filter()?;
        let keep = |item: &T| {
            filter.as_ref().is_none_or(|f| {
                serde_json::to_value(item).is_ok_and(|v| scim_filter::matches(f, &v))
            })
        };
        let skip = usize::try_from(self.start_index.unwrap_or(1) - 1).unwrap_or(0);
        let take = usize::try_from(self.count.unwrap_or(50)).unwrap_or(0);
        Ok(items
            .into_iter()
            .filter(keep)
            .skip(skip)
            .take(take)
            .collect())
    }

    /// The `startIndex` a list answers.
    fn start(&self) -> i64 {
        self.start_index.unwrap_or(1)
    }
}

/// A SCIM error, upstream's `Scim2RuntimeException`: the SCIM error body
/// with HTTP 500, as upstream's `GlobalExceptionHandler` answers it. The
/// body's `status` is the SCIM status.
#[derive(Debug)]
pub struct ScimError {
    status: u16,
    scim_type: Option<&'static str>,
    detail: String,
}

impl ScimError {
    pub fn new(status: u16, detail: impl Into<String>) -> Self {
        Self {
            status,
            scim_type: None,
            detail: detail.into(),
        }
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(400, detail)
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new(409, detail)
    }

    pub fn precondition_failed(detail: impl Into<String>) -> Self {
        Self::new(412, detail)
    }

    pub fn invalid_filter(detail: impl Into<String>) -> Self {
        Self {
            scim_type: Some("invalidFilter"),
            ..Self::new(400, detail)
        }
    }

    fn body(&self) -> serde_json::Value {
        let mut body = serde_json::json!({
            "schemas": [schema_uri::ERROR],
            "status": self.status.to_string(),
            "detail": self.detail,
        });
        if let Some(scim_type) = self.scim_type {
            body["scimType"] = serde_json::json!(scim_type);
        }
        body
    }
}

/// A SCIM handler's failure: a UC error (sign-in, permission, not found) or
/// a SCIM error.
pub enum ScimFailure {
    Api(Error),
    Scim(ScimError),
}

impl From<Error> for ScimFailure {
    fn from(e: Error) -> Self {
        Self::Api(e)
    }
}

impl From<ScimError> for ScimFailure {
    fn from(e: ScimError) -> Self {
        Self::Scim(e)
    }
}

impl ScimFailure {
    /// The result a failed bulk operation reports.
    fn bulk(self) -> BulkResult {
        match self {
            Self::Api(e) => err(e),
            Self::Scim(e) => (e.status, None, Some(e.body())),
        }
    }
}

impl IntoResponse for ScimFailure {
    fn into_response(self) -> Response {
        match self {
            Self::Api(e) => crate::http::error::ApiError(e).into_response(),
            Self::Scim(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(e.body())).into_response()
            }
        }
    }
}

type ScimResult<T> = std::result::Result<T, ScimFailure>;

/// A SCIM resource, `application/scim+json` as upstream `@Produces`.
pub struct Scim<T>(StatusCode, T);

impl<T: Serialize> IntoResponse for Scim<T> {
    fn into_response(self) -> Response {
        let mut response = (self.0, axum::Json(self.1)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/scim+json"),
        );
        response
    }
}

/// Upstream `createScimUser`: the primary email names the user; new users
/// are enabled.
async fn create_scim_user(state: &AppState, user: ScimUser) -> ScimResult<ScimUser> {
    let Some(email) = user.primary_email().map(str::to_string) else {
        return Err(ScimError::precondition_failed("User does not have a primary email.").into());
    };
    let user = ScimUser {
        user_name: email.clone(),
        display_name: user.display_name,
        external_id: user.external_id,
        emails: Some(vec![ScimEmail {
            value: email,
            primary: Some(true),
            r#type: None,
        }]),
        photos: user.photos,
        active: Some(true),
        ..Default::default()
    };
    match state.metastore.scim().create_user(user).await {
        Ok(created) => Ok(created),
        Err(e) if e.code().is_already_exists() => Err(ScimError::conflict(e.message()).into()),
        Err(e) => Err(ScimError::bad_request(e.message()).into()),
    }
}

#[utoipa::path(
    post, path = "/scim2/Users",
    operation_id = "scim_create_user",
    request_body = ScimUser,
    responses(
        (status = 201, body = ScimUser),
        (status = 500, description = "SCIM error: no primary email (412), already exists (409)"),
    ),
    tag = "SCIM",
)]
pub async fn create_user(
    State(state): State<AppState>,
    principal: Principal,
    Json(user): Json<ScimUser>,
) -> ScimResult<Scim<ScimUser>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Scim(
        StatusCode::CREATED,
        create_scim_user(&state, user).await?,
    ))
}

#[utoipa::path(
    get, path = "/scim2/Users",
    operation_id = "scim_list_users",
    params(
        ("count"      = Option<i64>, Query),
        ("startIndex" = Option<i64>, Query, description = "From 1"),
        ("filter"     = Option<String>, Query, description = "SCIM filter (RFC 7644 §3.4.2.2)"),
    ),
    responses((status = 200, body = ListScimUsersResponse)),
    tag = "SCIM",
)]
pub async fn list_users(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ScimListParams>,
) -> ScimResult<Scim<ListScimUsersResponse>> {
    authz::require_admin(&state, &principal.username).await?;
    let resources = params.page(state.metastore.scim().list_users().await?)?;
    // Upstream's totals count the page; its `meta` is the time of the list.
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    Ok(Scim(
        StatusCode::OK,
        ListScimUsersResponse {
            schemas: vec![schema_uri::LIST_RESPONSE.to_string()],
            total_results: resources.len() as i64,
            start_index: params.start(),
            items_per_page: resources.len() as i64,
            resources,
            meta: Some(ScimMeta {
                resource_type: "User".to_string(),
                created: Some(now.clone()),
                last_modified: Some(now),
            }),
        },
    ))
}

#[utoipa::path(
    get, path = "/scim2/Users/{id}",
    operation_id = "scim_get_user",
    params(("id" = String, Path)),
    responses(
        (status = 200, body = ScimUser),
        (status = 404, description = "Not found"),
    ),
    tag = "SCIM",
)]
pub async fn get_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ScimResult<Scim<ScimUser>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Scim(
        StatusCode::OK,
        state.metastore.scim().get_user(&id).await?,
    ))
}

#[utoipa::path(
    delete, path = "/scim2/Users/{id}",
    operation_id = "scim_delete_user",
    params(("id" = String, Path)),
    responses(
        (status = 200, description = "Disabled, grants removed"),
        (status = 404, description = "Not found"),
    ),
    tag = "SCIM",
)]
pub async fn delete_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    disable_user(&state, &id).await?;
    Ok(StatusCode::OK)
}

/// Upstream `deleteUser`: the user's grants go and the user stays,
/// disabled. Its tokens no longer sign in.
async fn disable_user(state: &AppState, id: &str) -> Result<(), Error> {
    let mut user = state.metastore.scim().get_user(id).await?;
    state
        .metastore
        .permissions()
        .clear_principal(user.principal())
        .await?;
    user.active = Some(false);
    state.metastore.scim().update_user(id, user).await?;
    Ok(())
}

#[utoipa::path(
    patch, path = "/scim2/Users/{id}",
    operation_id = "scim_patch_user",
    params(("id" = String, Path)),
    request_body = ScimPatchOp,
    responses(
        (status = 200, description = "State changed"),
        (status = 404, description = "Not found"),
        (status = 501, description = "No supported operation"),
    ),
    tag = "SCIM",
)]
pub async fn patch_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
    Json(patch): Json<ScimPatchOp>,
) -> ScimResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    patch_user_state(&state, &id, &patch).await
}

/// Upstream `patchUser`, for Okta: the first `replace` without a path sets
/// the state from its value's first member (`{"active": false}`); 200 with
/// no body. A `replace` of path `active` (Entra ID) counts too. Anything
/// else is 501.
async fn patch_user_state(
    state: &AppState,
    id: &str,
    patch: &ScimPatchOp,
) -> ScimResult<StatusCode> {
    let replace = patch.operations.iter().find(|op| {
        op.op.eq_ignore_ascii_case("replace")
            && op
                .path
                .as_deref()
                .is_none_or(|p| p.eq_ignore_ascii_case("active"))
    });
    let Some(op) = replace else {
        return Ok(StatusCode::NOT_IMPLEMENTED);
    };
    let value = match (&op.path, &op.value) {
        (None, Some(serde_json::Value::Object(members))) => members.values().next(),
        (Some(_), value) => value.as_ref(),
        _ => None,
    };
    let active = match value {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) if s.eq_ignore_ascii_case("true") => true,
        Some(serde_json::Value::String(s)) if s.eq_ignore_ascii_case("false") => false,
        _ => return Err(ScimError::new(500, "Problem with patch operation").into()),
    };
    let mut user = state.metastore.scim().get_user(id).await?;
    user.active = Some(active);
    state.metastore.scim().update_user(id, user).await?;
    Ok(StatusCode::OK)
}

/// `PUT /scim2/Users/{id}`: upstream `updateUser`. The body's `id` must be
/// the path's; the name, state and external id change when given; the
/// email stays.
#[utoipa::path(
    put, path = "/scim2/Users/{id}",
    operation_id = "scim_put_user",
    params(("id" = String, Path)),
    request_body = ScimUser,
    responses(
        (status = 200, body = ScimUser),
        (status = 404, description = "Not found"),
        (status = 409, description = "External id of another user"),
        (status = 500, description = "SCIM error: id mismatch (409)"),
    ),
    tag = "SCIM",
)]
pub async fn put_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
    Json(user): Json<ScimUser>,
) -> ScimResult<Scim<ScimUser>> {
    authz::require_admin(&state, &principal.username).await?;
    let mut current = state.metastore.scim().get_user(&id).await?;
    if user.id.as_deref() != Some(id.as_str()) {
        return Err(ScimError::conflict("User id mismatch.").into());
    }
    if user.display_name.is_some() {
        current.display_name = user.display_name;
    }
    if user.active.is_some() {
        current.active = user.active;
    }
    if user.external_id.is_some() {
        current.external_id = user.external_id;
    }
    Ok(Scim(
        StatusCode::OK,
        state.metastore.scim().update_user(&id, current).await?,
    ))
}

/// `/scim2/Me`: upstream `getCurrentUser`, the signed-in caller's user.
/// Without a caller (authorization off, no token) it is SCIM 400 "No user
/// found.". For a caller that the checks let do what the metastore admin
/// does (`authz::is_admin`), `roles` names `admin`: the web UI then offers
/// deletes with `force`.
#[utoipa::path(
    get, path = "/scim2/Me",
    operation_id = "scim_me",
    responses(
        (status = 200, body = ScimUser),
        (status = 401, description = "Not signed in (authorization on)"),
        (status = 404, description = "The caller is no user"),
        (status = 500, description = "SCIM error: no caller (400)"),
    ),
    tag = "SCIM",
)]
pub async fn me(State(state): State<AppState>, principal: Principal) -> ScimResult<Scim<ScimUser>> {
    let Some(caller) = principal.actor() else {
        return Err(ScimError::bad_request("No user found.").into());
    };
    let mut user = state
        .metastore
        .scim()
        .find_user_by_email(caller)
        .await?
        .ok_or_else(|| Error::not_found(format!("User not found: {caller}")))?;
    if authz::is_admin(&state, caller).await? {
        user.roles = Some(vec![ScimRole {
            value: ScimRole::ADMIN.into(),
        }]);
    }
    Ok(Scim(StatusCode::OK, user))
}

#[utoipa::path(
    post, path = "/scim2/Groups",
    operation_id = "scim_create_group",
    request_body = ScimGroup,
    responses(
        (status = 201, body = ScimGroup),
        (status = 409, description = "Already exists"),
    ),
    tag = "SCIM",
)]
pub async fn create_group(
    State(state): State<AppState>,
    principal: Principal,
    Json(group): Json<ScimGroup>,
) -> ApiResult<(StatusCode, Json<ScimGroup>)> {
    authz::require_admin(&state, &principal.username).await?;
    let created = state.metastore.scim().create_group(group).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get, path = "/scim2/Groups",
    operation_id = "scim_list_groups",
    params(
        ("count"      = Option<i64>, Query),
        ("startIndex" = Option<i64>, Query, description = "From 1"),
        ("filter"     = Option<String>, Query, description = "SCIM filter (RFC 7644 §3.4.2.2)"),
    ),
    responses((status = 200, body = ListScimGroupsResponse)),
    tag = "SCIM",
)]
pub async fn list_groups(
    State(state): State<AppState>,
    principal: Principal,
    Query(params): Query<ScimListParams>,
) -> ScimResult<Json<ListScimGroupsResponse>> {
    authz::require_admin(&state, &principal.username).await?;
    let resources = params.page(state.metastore.scim().list_groups().await?)?;
    Ok(Json(ListScimGroupsResponse {
        schemas: vec![schema_uri::LIST_RESPONSE.to_string()],
        total_results: resources.len() as i64,
        start_index: params.start(),
        items_per_page: resources.len() as i64,
        resources,
    }))
}

#[utoipa::path(
    get, path = "/scim2/Groups/{id}",
    operation_id = "scim_get_group",
    params(("id" = String, Path)),
    responses(
        (status = 200, body = ScimGroup),
        (status = 404, description = "Not found"),
    ),
    tag = "SCIM",
)]
pub async fn get_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ApiResult<Json<ScimGroup>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(state.metastore.scim().get_group(&id).await?))
}

#[utoipa::path(
    delete, path = "/scim2/Groups/{id}",
    operation_id = "scim_delete_group",
    params(("id" = String, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "SCIM",
)]
pub async fn delete_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.scim().delete_group(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    patch, path = "/scim2/Groups/{id}",
    operation_id = "scim_patch_group",
    params(("id" = String, Path)),
    request_body = ScimPatchOp,
    responses(
        (status = 200, body = ScimGroup),
        (status = 404, description = "Not found"),
    ),
    tag = "SCIM",
)]
pub async fn patch_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
    Json(patch): Json<ScimPatchOp>,
) -> ApiResult<Json<ScimGroup>> {
    authz::require_admin(&state, &principal.username).await?;
    let mut group = state.metastore.scim().get_group(&id).await?;
    apply_group_patch(&mut group, &patch)?;
    Ok(Json(state.metastore.scim().update_group(&id, group).await?))
}

#[utoipa::path(
    put, path = "/scim2/Groups/{id}",
    operation_id = "scim_put_group",
    params(("id" = String, Path)),
    request_body = ScimGroup,
    responses(
        (status = 200, body = ScimGroup),
        (status = 404, description = "Not found"),
    ),
    tag = "SCIM",
)]
pub async fn put_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
    Json(group): Json<ScimGroup>,
) -> ApiResult<Json<ScimGroup>> {
    authz::require_admin(&state, &principal.username).await?;
    Ok(Json(state.metastore.scim().update_group(&id, group).await?))
}

fn apply_group_patch(group: &mut ScimGroup, patch: &ScimPatchOp) -> Result<(), Error> {
    for op in &patch.operations {
        let op_kind = op.op.to_lowercase();
        match (op_kind.as_str(), op.path.as_deref()) {
            ("add", Some("members")) => {
                if let Some(val) = op.value.as_ref() {
                    let new_members: Vec<ScimGroupMember> = serde_json::from_value(val.clone())
                        .map_err(|e| {
                            Error::invalid_argument(format!("invalid members value: {e}"))
                        })?;
                    let members = group.members.get_or_insert_with(Vec::new);
                    for nm in new_members {
                        if !members.iter().any(|m| m.value == nm.value) {
                            members.push(nm);
                        }
                    }
                }
            }
            ("remove", Some(path)) if path.starts_with("members") => {
                if let Some(filter_id) = parse_member_filter(path) {
                    if let Some(members) = group.members.as_mut() {
                        members.retain(|m| m.value != filter_id);
                    }
                } else if let Some(members) = group.members.as_mut() {
                    members.clear();
                }
            }
            ("replace", Some("displayName")) => {
                if let Some(s) = op.value.as_ref().and_then(|v| v.as_str()) {
                    group.display_name = s.to_string();
                }
            }
            ("replace", None) => {
                if let Some(val) = op.value.as_ref() {
                    let replacement: ScimGroup =
                        serde_json::from_value(val.clone()).map_err(|e| {
                            Error::invalid_argument(format!("invalid replace body: {e}"))
                        })?;
                    *group = replacement;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// `members[value eq "<id>"]` → `Some("<id>")`, with whitespace around the
/// operator and quotes around the value tolerated. `None` for any other path.
fn parse_member_filter(path: &str) -> Option<String> {
    let inner = path.strip_prefix("members[")?.strip_suffix(']')?;
    let (_attr, value) = inner.trim().split_once(" eq ")?;
    Some(value.trim().trim_matches('"').to_string())
}

// SCIM bulk (RFC 7644 §3.7): operations on Users and Groups in one request.
// `failOnErrors` stops the batch after that many failures. Each operation
// has its own status; an unsupported one is 400.

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScimBulkRequest {
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(default, rename = "failOnErrors")]
    pub fail_on_errors: Option<usize>,
    #[serde(default, rename = "Operations")]
    pub operations: Vec<ScimBulkOp>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScimBulkOp {
    pub method: String,
    pub path: String,
    #[serde(default, rename = "bulkId")]
    pub bulk_id: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[utoipa::path(
    post, path = "/scim2/Bulk",
    operation_id = "scim_bulk",
    responses((status = 200, description = "BulkResponse with per-op status")),
    tag = "SCIM",
)]
pub async fn bulk(
    State(state): State<AppState>,
    principal: Principal,
    Json(req): Json<ScimBulkRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    authz::require_admin(&state, &principal.username).await?;
    if req.operations.len() > 100 {
        return Err(Error::invalid_argument("maxOperations is 100").into());
    }
    let mut results: Vec<serde_json::Value> = Vec::with_capacity(req.operations.len());
    let mut failures = 0usize;
    for op in req.operations {
        if let Some(limit) = req.fail_on_errors
            && failures >= limit
        {
            break;
        }
        let (status, location, response) = apply_bulk_op(&state, &op).await;
        if !(200..300).contains(&status) {
            failures += 1;
        }
        let mut entry = serde_json::json!({
            "method": op.method,
            "status": status.to_string(),
        });
        if let Some(bid) = op.bulk_id {
            entry["bulkId"] = serde_json::json!(bid);
        }
        if let Some(loc) = location {
            entry["location"] = serde_json::json!(loc);
        }
        if let Some(resp) = response {
            entry["response"] = resp;
        }
        results.push(entry);
    }
    Ok(Json(serde_json::json!({
        "schemas": [schema_uri::BULK_RESPONSE],
        "Operations": results,
    })))
}

#[derive(Clone, Copy)]
enum BulkTarget {
    Users,
    Groups,
}
#[derive(Clone, Copy)]
enum BulkVerb {
    Post,
    Patch,
    Delete,
}

impl BulkTarget {
    fn location(self, id: &str) -> String {
        let resource = match self {
            BulkTarget::Users => "Users",
            BulkTarget::Groups => "Groups",
        };
        format!("/scim2/{resource}/{id}")
    }
}

/// Parse `op.method` and `op.path` into `(verb, target, optional id)`.
/// `None` for an unsupported shape, which the caller answers with 400.
fn parse_bulk_route<'a>(
    method: &str,
    path: &'a str,
) -> Option<(BulkVerb, BulkTarget, Option<&'a str>)> {
    let verb = match method.to_uppercase().as_str() {
        "POST" => BulkVerb::Post,
        "PATCH" => BulkVerb::Patch,
        "DELETE" => BulkVerb::Delete,
        _ => return None,
    };
    let (target, id) = if let Some(id) = path.strip_prefix("/Users/") {
        (BulkTarget::Users, Some(id))
    } else if let Some(id) = path.strip_prefix("/Groups/") {
        (BulkTarget::Groups, Some(id))
    } else if path == "/Users" {
        (BulkTarget::Users, None)
    } else if path == "/Groups" {
        (BulkTarget::Groups, None)
    } else {
        return None;
    };
    Some((verb, target, id))
}

/// Returns `(status_code, optional location, optional response body)`.
async fn apply_bulk_op(
    state: &AppState,
    op: &ScimBulkOp,
) -> (u16, Option<String>, Option<serde_json::Value>) {
    let unsupported = || {
        let message = format!("unsupported bulk op: {} {}", op.method, op.path);
        (400, None, Some(err_body(400, message)))
    };
    let Some((verb, target, id)) = parse_bulk_route(&op.method, &op.path) else {
        return unsupported();
    };
    let scim = state.metastore.scim();
    match (verb, target, id) {
        (BulkVerb::Post, BulkTarget::Users, None) => {
            let user: ScimUser = match bulk_decode(op) {
                Ok(u) => u,
                Err(r) => return r,
            };
            match create_scim_user(state, user).await {
                Ok(u) => (
                    201,
                    Some(target.location(u.id.as_deref().unwrap_or(""))),
                    serde_json::to_value(&u).ok(),
                ),
                Err(e) => e.bulk(),
            }
        }
        (BulkVerb::Post, BulkTarget::Groups, None) => {
            let group: ScimGroup = match bulk_decode(op) {
                Ok(g) => g,
                Err(r) => return r,
            };
            into_status(scim.create_group(group).await, 201, |g| {
                target.location(g.id.as_deref().unwrap_or(""))
            })
        }
        (BulkVerb::Delete, BulkTarget::Users, Some(id)) => match disable_user(state, id).await {
            Ok(()) => (200, None, None),
            Err(e) => err(e),
        },
        (BulkVerb::Delete, BulkTarget::Groups, Some(id)) => no_content(scim.delete_group(id).await),
        (BulkVerb::Patch, BulkTarget::Users, Some(id)) => {
            let patch: ScimPatchOp = match bulk_decode(op) {
                Ok(p) => p,
                Err(r) => return r,
            };
            match patch_user_state(state, id, &patch).await {
                Ok(status) => (status.as_u16(), Some(target.location(id)), None),
                Err(e) => e.bulk(),
            }
        }
        (BulkVerb::Patch, BulkTarget::Groups, Some(id)) => {
            let patch: ScimPatchOp = match bulk_decode(op) {
                Ok(p) => p,
                Err(r) => return r,
            };
            match scim.get_group(id).await {
                Err(e) => err(e),
                Ok(mut group) => {
                    if let Err(e) = apply_group_patch(&mut group, &patch) {
                        return (400, None, Some(err_body(400, e.to_string())));
                    }
                    into_status(scim.update_group(id, group).await, 200, |_| {
                        target.location(id)
                    })
                }
            }
        }
        _ => unsupported(),
    }
}

type BulkResult = (u16, Option<String>, Option<serde_json::Value>);

fn bulk_decode<T: serde::de::DeserializeOwned>(op: &ScimBulkOp) -> Result<T, BulkResult> {
    let Some(data) = op.data.clone() else {
        return Err((400, None, Some(err_body(400, "missing data"))));
    };
    serde_json::from_value(data).map_err(|e| {
        (
            400,
            None,
            Some(err_body(400, format!("invalid payload: {e}"))),
        )
    })
}

fn into_status<T: serde::Serialize>(
    res: Result<T, Error>,
    ok_status: u16,
    locate: impl FnOnce(&T) -> String,
) -> BulkResult {
    match res {
        Ok(v) => (
            ok_status,
            Some(locate(&v)),
            Some(serde_json::to_value(&v).unwrap_or_default()),
        ),
        Err(e) => err(e),
    }
}

fn no_content(res: Result<(), Error>) -> BulkResult {
    match res {
        Ok(()) => (204, None, None),
        Err(e) => err(e),
    }
}

fn err(e: Error) -> BulkResult {
    let status = e.code().status().as_u16();
    (status, None, Some(err_body(status, e.to_string())))
}

fn err_body(status: u16, msg: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "status": status.to_string(), "detail": msg.into() })
}

// SCIM discovery (RFC 7644 §4): Azure AD, Okta and OneLogin need it to
// learn what the server supports.

fn schemas_doc() -> serde_json::Value {
    serde_json::json!([
        {
            "schemas": [schema_uri::SCHEMA],
            "id": "urn:ietf:params:scim:schemas:core:2.0:User",
            "name": "User",
            "description": "SCIM core User schema",
            "attributes": [
                { "name": "userName",    "type": "string",  "required": true,  "uniqueness": "server" },
                { "name": "displayName", "type": "string",  "required": false },
                { "name": "active",      "type": "boolean", "required": false },
                { "name": "emails",      "type": "complex", "multiValued": true, "required": false,
                  "subAttributes": [
                    { "name": "value",   "type": "string",  "required": true },
                    { "name": "primary", "type": "boolean", "required": false },
                ]},
            ],
            "meta": { "resourceType": "Schema", "location": "/scim2/Schemas/urn:ietf:params:scim:schemas:core:2.0:User" }
        },
        {
            "schemas": [schema_uri::SCHEMA],
            "id": "urn:ietf:params:scim:schemas:core:2.0:Group",
            "name": "Group",
            "description": "SCIM core Group schema",
            "attributes": [
                { "name": "displayName", "type": "string",  "required": true,  "uniqueness": "server" },
                { "name": "members",     "type": "complex", "multiValued": true, "required": false,
                  "subAttributes": [
                    { "name": "value",   "type": "string",  "required": true },
                    { "name": "display", "type": "string",  "required": false },
                ]},
            ],
            "meta": { "resourceType": "Schema", "location": "/scim2/Schemas/urn:ietf:params:scim:schemas:core:2.0:Group" }
        }
    ])
}

fn resource_types_doc() -> serde_json::Value {
    serde_json::json!([
        {
            "schemas": [schema_uri::RESOURCE_TYPE],
            "id": "User",
            "name": "User",
            "endpoint": "/Users",
            "description": "SCIM 2.0 User resource",
            "schema": "urn:ietf:params:scim:schemas:core:2.0:User",
            "meta": { "resourceType": "ResourceType", "location": "/scim2/ResourceTypes/User" }
        },
        {
            "schemas": [schema_uri::RESOURCE_TYPE],
            "id": "Group",
            "name": "Group",
            "endpoint": "/Groups",
            "description": "SCIM 2.0 Group resource",
            "schema": "urn:ietf:params:scim:schemas:core:2.0:Group",
            "meta": { "resourceType": "ResourceType", "location": "/scim2/ResourceTypes/Group" }
        }
    ])
}

fn service_provider_config_doc() -> serde_json::Value {
    serde_json::json!({
        "schemas": [schema_uri::SERVICE_PROVIDER_CONFIG],
        "patch":          { "supported": true },
        "bulk":           { "supported": true, "maxOperations": 100, "maxPayloadSize": 1_048_576 },
        "filter":         { "supported": true, "maxResults": 1000 },
        "changePassword": { "supported": false },
        "sort":           { "supported": false },
        "etag":           { "supported": false },
        "authenticationSchemes": [
            { "type": "oauthbearertoken", "name": "OAuth Bearer Token",
              "description": "Authentication scheme using the OAuth Bearer Token Standard",
              "primary": true }
        ],
        "meta": { "resourceType": "ServiceProviderConfig", "location": "/scim2/ServiceProviderConfig" }
    })
}

/// `resources` as a SCIM ListResponse, the format of the discovery lists (RFC 7644, section 4).
fn list_response(resources: serde_json::Value) -> serde_json::Value {
    let count = resources.as_array().map_or(0, Vec::len);
    serde_json::json!({
        "schemas": [schema_uri::LIST_RESPONSE],
        "totalResults": count,
        "startIndex": 1,
        "itemsPerPage": count,
        "Resources": resources,
    })
}

#[utoipa::path(get, path = "/scim2/Schemas",
    operation_id = "scim_list_schemas",
    responses((status = 200, description = "SCIM Schemas catalog, as a ListResponse")),
    tag = "SCIM",
)]
pub async fn list_schemas() -> axum::Json<serde_json::Value> {
    axum::Json(list_response(schemas_doc()))
}

#[utoipa::path(get, path = "/scim2/ResourceTypes",
    operation_id = "scim_list_resource_types",
    responses((status = 200, description = "SCIM ResourceTypes catalog, as a ListResponse")),
    tag = "SCIM",
)]
pub async fn list_resource_types() -> axum::Json<serde_json::Value> {
    axum::Json(list_response(resource_types_doc()))
}

#[utoipa::path(get, path = "/scim2/ServiceProviderConfig",
    operation_id = "scim_service_provider_config",
    responses((status = 200, description = "SCIM ServiceProviderConfig")),
    tag = "SCIM",
)]
pub async fn service_provider_config() -> axum::Json<serde_json::Value> {
    axum::Json(service_provider_config_doc())
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_user, list_users))
        .routes(routes!(get_user, patch_user, put_user, delete_user))
        .routes(routes!(me))
        .routes(routes!(create_group, list_groups))
        .routes(routes!(get_group, patch_group, put_group, delete_group))
        .routes(routes!(list_schemas))
        .routes(routes!(list_resource_types))
        .routes(routes!(service_provider_config))
        .routes(routes!(bulk))
}
