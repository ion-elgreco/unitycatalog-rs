//! SCIM 2.0 (RFC 7643 and RFC 7644) under the control plane: users,
//! groups, the signed-in user, searches, bulk requests and the discovery
//! endpoints. Only the metastore admin manages users and groups.
//!
//! Requests are read by the schemas of [`scim_schema`]: attribute names in
//! any case, unknown attributes left alone. Every error under `/scim2` is a
//! SCIM error with its HTTP status ([`error_layer`]).

use std::convert::Infallible;

use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::any,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::scim::schema_uri;
use uc_api::{
    ListScimGroupsResponse, ListScimUsersResponse, ScimGroup, ScimPatchOp, ScimPatchOperation,
    ScimRole, ScimUser,
};
use uc_common::{CONTROL_BASE, Error, ErrorResponse};

use crate::{
    authn::principal::Principal,
    authz,
    http::extract::{Path, Query},
    routes::control::scim_filter::{self, Filter},
    routes::control::scim_patch,
    routes::control::scim_schema::{
        self, GROUPS, MAX_RESULTS, RESOURCE_TYPES, ResourceType, USERS,
    },
    server::state::AppState,
};

/// A SCIM error (RFC 7644, section 3.12): its HTTP status, `scimType` and
/// detail.
#[derive(Debug)]
pub struct ScimError {
    status: StatusCode,
    scim_type: Option<&'static str>,
    detail: String,
}

impl ScimError {
    pub fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            scim_type: None,
            detail: detail.into(),
        }
    }

    /// 400 with the `scimType` `scim_type`.
    pub fn bad_request(scim_type: &'static str, detail: impl Into<String>) -> Self {
        Self {
            scim_type: Some(scim_type),
            ..Self::new(StatusCode::BAD_REQUEST, detail)
        }
    }

    fn body(&self) -> Value {
        let mut body = json!({
            "schemas": [schema_uri::ERROR],
            "status": self.status.as_u16().to_string(),
            "detail": self.detail,
        });
        if let Some(scim_type) = self.scim_type {
            body["scimType"] = json!(scim_type);
        }
        body
    }
}

/// A UC error as a SCIM error: an existing user or group is `uniqueness`,
/// a bad request `invalidValue`.
impl From<Error> for ScimError {
    fn from(e: Error) -> Self {
        let status = e.code().status();
        let scim_type = match status {
            StatusCode::CONFLICT => Some("uniqueness"),
            StatusCode::BAD_REQUEST => Some("invalidValue"),
            _ => None,
        };
        if status.is_server_error() {
            tracing::error!(error = %e, "server error");
        }
        Self {
            status,
            scim_type,
            detail: e.message().to_string(),
        }
    }
}

impl From<scim_patch::PatchError> for ScimError {
    fn from(e: scim_patch::PatchError) -> Self {
        Self::bad_request(e.scim_type, e.detail)
    }
}

impl IntoResponse for ScimError {
    fn into_response(self) -> Response {
        scim_json(self.status, self.body())
    }
}

type ScimResult<T> = std::result::Result<T, ScimError>;

/// `body` as `application/scim+json`.
fn scim_json(status: StatusCode, body: Value) -> Response {
    let mut response = (status, axum::Json(body)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/scim+json"),
    );
    response
}

/// A SCIM answer, with `Location` for a created resource.
pub struct Scim {
    status: StatusCode,
    body: Value,
}

impl Scim {
    fn ok(body: Value) -> Self {
        Self {
            status: StatusCode::OK,
            body,
        }
    }

    fn created(body: Value) -> Self {
        Self {
            status: StatusCode::CREATED,
            body,
        }
    }
}

impl IntoResponse for Scim {
    fn into_response(self) -> Response {
        let location = (self.status == StatusCode::CREATED)
            .then(|| self.body["meta"]["location"].as_str().map(str::to_string))
            .flatten();
        let mut response = scim_json(self.status, self.body);
        if let Some(location) = location.and_then(|l| HeaderValue::from_str(&l).ok()) {
            response.headers_mut().insert(header::LOCATION, location);
        }
        response
    }
}

/// Every error under `/scim2` is a SCIM error: those of the handlers, and
/// those of sign-in, request parsing and unknown paths and methods, which
/// come as UC errors or bare statuses.
pub async fn error_layer(req: Request, next: Next) -> Response {
    let scim = req
        .uri()
        .path()
        .strip_prefix(CONTROL_BASE)
        .and_then(|rest| rest.strip_prefix("/scim2"))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
    let resp = next.run(req).await;
    let status = resp.status();
    if !scim || !(status.is_client_error() || status.is_server_error()) {
        return resp;
    }
    let is_scim = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|t| t.as_bytes().starts_with(b"application/scim+json"));
    if is_scim {
        return resp;
    }
    let allow = resp.headers().get(header::ALLOW).cloned();
    let detail = match resp.extensions().get::<ErrorResponse>() {
        Some(uc) => uc.message.clone(),
        None => {
            let (_, body) = resp.into_parts();
            let bytes = axum::body::to_bytes(body, 1 << 20)
                .await
                .unwrap_or_default();
            let text = String::from_utf8_lossy(&bytes).trim().to_string();
            match text.is_empty() {
                true => status.canonical_reason().unwrap_or("Error").to_string(),
                false => text,
            }
        }
    };
    let error = match status {
        StatusCode::BAD_REQUEST => ScimError::bad_request("invalidSyntax", detail),
        status => ScimError::new(status, detail),
    };
    let mut resp = error.into_response();
    if let Some(allow) = allow {
        resp.headers_mut().insert(header::ALLOW, allow);
    }
    resp
}

/// A path under `/scim2` that names no endpoint.
async fn unknown_endpoint() -> ScimError {
    ScimError::new(StatusCode::NOT_FOUND, "No SCIM endpoint has this path")
}

/// The URL of the SCIM endpoints as the client called them:
/// `X-Forwarded-Proto` and `X-Forwarded-Host` behind a proxy, else `Host`.
pub struct BaseUrl(String);

impl<S: Send + Sync> FromRequestParts<S> for BaseUrl {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(base_url(&parts.headers)))
    }
}

fn base_url(headers: &HeaderMap) -> String {
    let first = |name: &str| {
        headers
            .get(name)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(',').next())
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .map(str::to_string)
    };
    let host = first("x-forwarded-host")
        .or_else(|| first(header::HOST.as_str()))
        .unwrap_or_else(|| "localhost".to_string());
    let scheme = first("x-forwarded-proto").unwrap_or_else(|| "http".to_string());
    format!("{scheme}://{host}{CONTROL_BASE}/scim2")
}

/// A JSON request body in any content type; malformed JSON is SCIM
/// `invalidSyntax`.
pub struct ScimBody(Value);

impl<S: Send + Sync> FromRequest<S> for ScimBody {
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;
        serde_json::from_slice(&bytes).map(ScimBody).map_err(|e| {
            ScimError::bad_request("invalidSyntax", format!("Malformed request body: {e}"))
                .into_response()
        })
    }
}

/// `value`, read by `resource_type`, as a `T`.
fn decode<T: DeserializeOwned>(resource_type: &ResourceType, value: Value) -> ScimResult<T> {
    serde_json::from_value(resource_type.read(value)).map_err(|e| {
        ScimError::bad_request("invalidValue", format!("Invalid {}: {e}", resource_type.id))
    })
}

/// The member `name` of `object`, in any case.
fn member(object: &Map<String, Value>, name: &str) -> Option<Value> {
    object
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

/// The operations of a PatchOp body, with names in any case.
fn patch_operations(body: Value) -> ScimResult<Vec<ScimPatchOperation>> {
    let invalid = |detail: &str| ScimError::bad_request("invalidSyntax", detail);
    let Value::Object(body) = body else {
        return Err(invalid("A PatchOp is an object"));
    };
    let Some(Value::Array(operations)) = member(&body, "Operations") else {
        return Err(invalid("A PatchOp needs a list of Operations"));
    };
    operations
        .into_iter()
        .map(|operation| {
            let Value::Object(operation) = operation else {
                return Err(invalid("Each operation is an object"));
            };
            let Some(Value::String(op)) = member(&operation, "op") else {
                return Err(invalid("Each operation needs an op"));
            };
            let path = match member(&operation, "path") {
                Some(Value::String(path)) => Some(path),
                None | Some(Value::Null) => None,
                Some(_) => return Err(invalid("The path of an operation is text")),
            };
            Ok(ScimPatchOperation {
                op,
                path,
                value: member(&operation, "value"),
            })
        })
        .collect()
}

/// The `attributes` and `excludedAttributes` parameters, which pick the
/// attributes of the resources in an answer.
#[derive(Debug, Default, Deserialize)]
pub struct Selection {
    #[serde(default)]
    pub attributes: Option<String>,
    #[serde(rename = "excludedAttributes", default)]
    pub excluded_attributes: Option<String>,
}

impl Selection {
    fn names(text: &Option<String>) -> Vec<String> {
        text.iter().map(String::to_string).collect()
    }

    fn apply(&self, resource_type: &ResourceType, resource: Value) -> Value {
        resource_type.select(
            resource,
            &Self::names(&self.attributes),
            &Self::names(&self.excluded_attributes),
        )
    }
}

/// SCIM list parameters: `filter`, `startIndex`, `count`, `attributes` and
/// `excludedAttributes`.
#[derive(Debug, Default, Deserialize)]
pub struct ScimListParams {
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(rename = "startIndex", default)]
    pub start_index: Option<i64>,
    #[serde(default)]
    pub count: Option<i64>,
    #[serde(flatten)]
    pub selection: Selection,
}

/// A list request, from the query (GET) or a SearchRequest (POST
/// `.search`, RFC 7644, section 3.4.3).
struct ListQuery {
    filter: Option<Filter>,
    /// From 1.
    start: usize,
    count: usize,
    attributes: Vec<String>,
    excluded: Vec<String>,
}

/// The default `count`.
const DEFAULT_COUNT: i64 = 50;

impl ListQuery {
    fn new(
        filter: Option<&str>,
        start_index: Option<i64>,
        count: Option<i64>,
        attributes: Vec<String>,
        excluded: Vec<String>,
    ) -> ScimResult<Self> {
        let filter = match filter.map(str::trim).filter(|f| !f.is_empty()) {
            None => None,
            Some(f) => Some(
                scim_filter::parse(f).map_err(|e| ScimError::bad_request("invalidFilter", e))?,
            ),
        };
        Ok(Self {
            filter,
            start: usize::try_from(start_index.unwrap_or(1).max(1)).unwrap_or(1),
            count: usize::try_from(count.unwrap_or(DEFAULT_COUNT).max(0))
                .unwrap_or(0)
                .min(MAX_RESULTS),
            attributes,
            excluded,
        })
    }

    fn from_params(params: &ScimListParams) -> ScimResult<Self> {
        Self::new(
            params.filter.as_deref(),
            params.start_index,
            params.count,
            Selection::names(&params.selection.attributes),
            Selection::names(&params.selection.excluded_attributes),
        )
    }

    fn from_search(body: Value) -> ScimResult<Self> {
        let Value::Object(body) = body else {
            return Err(ScimError::bad_request(
                "invalidSyntax",
                "A SearchRequest is an object",
            ));
        };
        let text = |name: &str| match member(&body, name) {
            Some(Value::String(s)) => Some(s),
            _ => None,
        };
        let number = |name: &str| member(&body, name).and_then(|v| v.as_i64());
        let names = |name: &str| match member(&body, name) {
            Some(Value::Array(items)) => items
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            Some(Value::String(s)) => vec![s],
            _ => Vec::new(),
        };
        Self::new(
            text("filter").as_deref(),
            number("startIndex"),
            number("count"),
            names("attributes"),
            names("excludedAttributes"),
        )
    }

    /// The ListResponse of `resources`, each as JSON with its type:
    /// `totalResults` counts the matches, the page holds `count` of them
    /// from `start`.
    fn answer(&self, resources: Vec<(&ResourceType, Value)>) -> Value {
        let matching: Vec<(&ResourceType, Value)> = resources
            .into_iter()
            .filter(|(_, r)| {
                self.filter
                    .as_ref()
                    .is_none_or(|f| scim_filter::matches(f, r))
            })
            .collect();
        let total = matching.len();
        let page: Vec<Value> = matching
            .into_iter()
            .skip(self.start - 1)
            .take(self.count)
            .map(|(rt, r)| rt.select(r, &self.attributes, &self.excluded))
            .collect();
        json!({
            "schemas": [schema_uri::LIST_RESPONSE],
            "totalResults": total,
            "startIndex": self.start,
            "itemsPerPage": page.len(),
            "Resources": page,
        })
    }
}

/// `resource` as JSON, with its URL under `base`, and the URL of each
/// member as its `$ref`.
fn located<T: Serialize>(resource_type: &ResourceType, resource: &T, base: &str) -> Value {
    let mut value = serde_json::to_value(resource).unwrap_or_default();
    if let Some(members) = value.get_mut("members").and_then(Value::as_array_mut) {
        for member in members.iter_mut().filter_map(Value::as_object_mut) {
            if let Some(id) = member.get("value").and_then(Value::as_str) {
                let url = format!("{base}{}/{id}", USERS.endpoint);
                member.insert("$ref".into(), json!(url));
            }
        }
    }
    if let Some(id) = value["id"].as_str().map(str::to_string) {
        let location = format!("{base}{}/{id}", resource_type.endpoint);
        match value.get_mut("meta").and_then(Value::as_object_mut) {
            Some(meta) => {
                meta.insert("location".into(), json!(location));
            }
            None => {
                value["meta"] = json!({ "resourceType": resource_type.id, "location": location });
            }
        }
    }
    value
}

/// `current` with the attributes of `body` that a client may write: a
/// value replaces, `null` removes, an absent attribute stays (RFC 7644,
/// section 3.5.1, lets a server keep it). An `id` other than `id` is an
/// error.
fn replaced(
    resource_type: &ResourceType,
    current: Value,
    body: Value,
    id: &str,
) -> ScimResult<Value> {
    let Value::Object(body) = resource_type.read(body) else {
        return Err(ScimError::bad_request(
            "invalidSyntax",
            format!("A {} is an object", resource_type.id),
        ));
    };
    if let Some(name) = resource_type.missing_required(&body) {
        return Err(ScimError::bad_request(
            "invalidValue",
            format!("A replace request needs {name}"),
        ));
    }
    let Value::Object(mut resource) = current else {
        return Err(Error::internal("a stored resource is not an object").into());
    };
    for (name, value) in body {
        match name.as_str() {
            "id" => {
                if value.as_str().is_some_and(|v| v != id) {
                    return Err(ScimError::bad_request(
                        "mutability",
                        "The id of the body is not the id of the path",
                    ));
                }
            }
            "schemas" | "meta" => {}
            "externalId" => set(&mut resource, name, value),
            _ => {
                if let Some(attribute) = scim_schema::find(resource_type.attributes, &name)
                    && attribute.mutability != scim_schema::Mutability::ReadOnly
                {
                    set(&mut resource, attribute.name.to_string(), value);
                }
            }
        }
    }
    Ok(Value::Object(resource))
}

/// A patch may not remove a required attribute.
fn still_has_required(
    resource_type: &ResourceType,
    resource: &Map<String, Value>,
) -> ScimResult<()> {
    match resource_type.missing_required(resource) {
        Some(name) => Err(ScimError::bad_request(
            "mutability",
            format!("{name} is required"),
        )),
        None => Ok(()),
    }
}

fn set(resource: &mut Map<String, Value>, name: String, value: Value) {
    match value {
        Value::Null => {
            resource.remove(&name);
        }
        value => {
            resource.insert(name, value);
        }
    }
}

/// The user that a client sent, as the server keeps it: no id, times or
/// roles.
fn new_user(mut user: ScimUser) -> ScimUser {
    user.id = None;
    user.meta = None;
    user.roles = None;
    user
}

async fn create_user_from(state: &AppState, body: Value) -> ScimResult<ScimUser> {
    let user: ScimUser = decode(&USERS, body)?;
    Ok(state.metastore.scim().create_user(new_user(user)).await?)
}

/// Replace the user `id` with `user`. When its principal changes, the
/// server logs what still names the old one.
async fn update_user(
    state: &AppState,
    id: &str,
    current: &ScimUser,
    user: ScimUser,
) -> ScimResult<ScimUser> {
    let updated = state
        .metastore
        .scim()
        .update_user(id, new_user(user))
        .await?;
    if updated.principal() != current.principal() {
        principal_changed(state, current.principal(), updated.principal());
    }
    Ok(updated)
}

/// The grants, owned objects and tokens of `old` moved to `new`. Settings
/// and Cedar policies that name `old` stay as they are: the server names
/// them in warnings.
fn principal_changed(state: &AppState, old: &str, new: &str) {
    tracing::info!(%old, %new, "a user's principal changed: its grants, objects and tokens moved");
    if state.admin_principals.contains(old) {
        tracing::warn!(%old, %new, "auth.admin_principals names the old principal of a user");
    }
    let quoted = format!("\"{old}\"");
    for file in state.cedar_files.iter() {
        if std::fs::read_to_string(file).is_ok_and(|text| text.contains(&quoted)) {
            tracing::warn!(
                file = %file.display(), %old, %new,
                "a Cedar policy file names the old principal of a user"
            );
        }
    }
}

async fn put_user_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimUser> {
    let current = state.metastore.scim().get_user(id).await?;
    let merged = replaced(
        &USERS,
        serde_json::to_value(&current).unwrap_or_default(),
        body,
        id,
    )?;
    let user: ScimUser = decode(&USERS, merged)?;
    update_user(state, id, &current, user).await
}

async fn patch_user_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimUser> {
    let operations = patch_operations(body)?;
    let current = state.metastore.scim().get_user(id).await?;
    let Value::Object(mut resource) = serde_json::to_value(&current).unwrap_or_default() else {
        return Err(Error::internal("a stored user is not an object").into());
    };
    scim_patch::apply(&mut resource, &USERS, &operations)?;
    still_has_required(&USERS, &resource)?;
    let user: ScimUser = decode(&USERS, Value::Object(resource))?;
    update_user(state, id, &current, user).await
}

#[utoipa::path(
    post, path = "/scim2/Users",
    operation_id = "scim_create_user",
    request_body = ScimUser,
    responses(
        (status = 201, body = ScimUser),
        (status = 400, description = "SCIM error: no userName or email (invalidValue)"),
        (status = 409, description = "SCIM error: the userName, principal or externalId of another user (uniqueness)"),
    ),
    tag = "SCIM",
)]
pub async fn create_user(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Query(selection): Query<Selection>,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let user = create_user_from(&state, body).await?;
    Ok(Scim::created(
        selection.apply(&USERS, located(&USERS, &user, &base)),
    ))
}

#[utoipa::path(
    get, path = "/scim2/Users",
    operation_id = "scim_list_users",
    params(
        ("filter"             = Option<String>, Query, description = "SCIM filter (RFC 7644, section 3.4.2.2)"),
        ("startIndex"         = Option<i64>, Query, description = "From 1"),
        ("count"              = Option<i64>, Query),
        ("attributes"         = Option<String>, Query),
        ("excludedAttributes" = Option<String>, Query),
    ),
    responses((status = 200, body = ListScimUsersResponse)),
    tag = "SCIM",
)]
pub async fn list_users(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Query(params): Query<ScimListParams>,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    list_users_by(&state, &base, ListQuery::from_params(&params)?).await
}

/// Every user, as JSON with its URL under `base`.
async fn users(state: &AppState, base: &str) -> ScimResult<Vec<(&'static ResourceType, Value)>> {
    let users = state.metastore.scim().list_users().await?;
    Ok(users
        .iter()
        .map(|u| (&USERS, located(&USERS, u, base)))
        .collect())
}

async fn list_users_by(state: &AppState, base: &str, query: ListQuery) -> ScimResult<Scim> {
    Ok(Scim::ok(query.answer(users(state, base).await?)))
}

#[utoipa::path(
    post, path = "/scim2/Users/.search",
    operation_id = "scim_search_users",
    responses((status = 200, body = ListScimUsersResponse)),
    tag = "SCIM",
)]
pub async fn search_users(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    list_users_by(&state, &base, ListQuery::from_search(body)?).await
}

#[utoipa::path(
    get, path = "/scim2/Users/{id}",
    operation_id = "scim_get_user",
    params(
        ("id" = String, Path),
        ("attributes"         = Option<String>, Query),
        ("excludedAttributes" = Option<String>, Query),
    ),
    responses(
        (status = 200, body = ScimUser),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn get_user(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Path(id): Path<String>,
    Query(selection): Query<Selection>,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let user = state.metastore.scim().get_user(&id).await?;
    Ok(Scim::ok(
        selection.apply(&USERS, located(&USERS, &user, &base)),
    ))
}

#[utoipa::path(
    put, path = "/scim2/Users/{id}",
    operation_id = "scim_put_user",
    params(("id" = String, Path)),
    request_body = ScimUser,
    responses(
        (status = 200, body = ScimUser),
        (status = 400, description = "SCIM error: an id other than the path's (mutability)"),
        (status = 404, description = "SCIM error: not found"),
        (status = 409, description = "SCIM error: the userName, principal or externalId of another user (uniqueness)"),
    ),
    tag = "SCIM",
)]
pub async fn put_user(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Path(id): Path<String>,
    Query(selection): Query<Selection>,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let user = put_user_from(&state, &id, body).await?;
    Ok(Scim::ok(
        selection.apply(&USERS, located(&USERS, &user, &base)),
    ))
}

#[utoipa::path(
    patch, path = "/scim2/Users/{id}",
    operation_id = "scim_patch_user",
    params(("id" = String, Path)),
    request_body = ScimPatchOp,
    responses(
        (status = 200, body = ScimUser),
        (status = 400, description = "SCIM error: a malformed operation"),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn patch_user(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Path(id): Path<String>,
    Query(selection): Query<Selection>,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let user = patch_user_from(&state, &id, body).await?;
    Ok(Scim::ok(
        selection.apply(&USERS, located(&USERS, &user, &base)),
    ))
}

#[utoipa::path(
    delete, path = "/scim2/Users/{id}",
    operation_id = "scim_delete_user",
    params(("id" = String, Path)),
    responses(
        (status = 204, description = "Deleted, with its grants, tokens and group memberships"),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn delete_user(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ScimResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.scim().delete_user(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `/scim2/Me` (RFC 7644, section 3.11): the signed-in caller's user.
/// Without a caller (authorization off, no token) it is upstream's 400 "No
/// user found.". For a caller that the checks let do what the metastore
/// admin does (`authz::is_admin`), `roles` names `admin`: the web UI then
/// offers deletes with `force`.
#[utoipa::path(
    get, path = "/scim2/Me",
    operation_id = "scim_me",
    responses(
        (status = 200, body = ScimUser),
        (status = 400, description = "SCIM error: no caller"),
        (status = 401, description = "SCIM error: not signed in"),
        (status = 404, description = "SCIM error: the caller is no user"),
    ),
    tag = "SCIM",
)]
pub async fn me(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
) -> ScimResult<Scim> {
    let Some(caller) = principal.actor() else {
        return Err(ScimError::new(StatusCode::BAD_REQUEST, "No user found."));
    };
    let mut user = state
        .metastore
        .scim()
        .find_user_by_principal(caller)
        .await?
        .ok_or_else(|| Error::not_found(format!("User not found: {caller}")))?;
    if authz::is_admin(&state, caller).await? {
        user.roles = Some(vec![ScimRole {
            value: ScimRole::ADMIN.into(),
        }]);
    }
    Ok(Scim::ok(located(&USERS, &user, &base)))
}

/// `POST /scim2/.search` (RFC 7644, section 3.4.3): users and groups at
/// once.
#[utoipa::path(
    post, path = "/scim2/.search",
    operation_id = "scim_search",
    responses((status = 200, description = "A ListResponse of users and groups")),
    tag = "SCIM",
)]
pub async fn search(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let query = ListQuery::from_search(body)?;
    let mut resources = users(&state, &base).await?;
    resources.extend(groups(&state, &base).await?);
    Ok(Scim::ok(query.answer(resources)))
}

/// The group that a client sent, as the server keeps it: no id or times.
fn new_group(mut group: ScimGroup) -> ScimGroup {
    group.id = None;
    group.meta = None;
    group
}

async fn create_group_from(state: &AppState, body: Value) -> ScimResult<ScimGroup> {
    let group: ScimGroup = decode(&GROUPS, body)?;
    Ok(state
        .metastore
        .scim()
        .create_group(new_group(group))
        .await?)
}

async fn put_group_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimGroup> {
    let current = state.metastore.scim().get_group(id).await?;
    let merged = replaced(
        &GROUPS,
        serde_json::to_value(&current).unwrap_or_default(),
        body,
        id,
    )?;
    let group: ScimGroup = decode(&GROUPS, merged)?;
    Ok(state
        .metastore
        .scim()
        .update_group(id, new_group(group))
        .await?)
}

async fn patch_group_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimGroup> {
    let operations = patch_operations(body)?;
    let current = state.metastore.scim().get_group(id).await?;
    let Value::Object(mut resource) = serde_json::to_value(&current).unwrap_or_default() else {
        return Err(Error::internal("a stored group is not an object").into());
    };
    scim_patch::apply(&mut resource, &GROUPS, &operations)?;
    still_has_required(&GROUPS, &resource)?;
    let group: ScimGroup = decode(&GROUPS, Value::Object(resource))?;
    Ok(state
        .metastore
        .scim()
        .update_group(id, new_group(group))
        .await?)
}

#[utoipa::path(
    post, path = "/scim2/Groups",
    operation_id = "scim_create_group",
    request_body = ScimGroup,
    responses(
        (status = 201, body = ScimGroup),
        (status = 400, description = "SCIM error: no displayName, or a member that is no user (invalidValue)"),
        (status = 409, description = "SCIM error: the name of another group or user (uniqueness)"),
    ),
    tag = "SCIM",
)]
pub async fn create_group(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Query(selection): Query<Selection>,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let group = create_group_from(&state, body).await?;
    Ok(Scim::created(
        selection.apply(&GROUPS, located(&GROUPS, &group, &base)),
    ))
}

#[utoipa::path(
    get, path = "/scim2/Groups",
    operation_id = "scim_list_groups",
    params(
        ("filter"             = Option<String>, Query, description = "SCIM filter (RFC 7644, section 3.4.2.2)"),
        ("startIndex"         = Option<i64>, Query, description = "From 1"),
        ("count"              = Option<i64>, Query),
        ("attributes"         = Option<String>, Query),
        ("excludedAttributes" = Option<String>, Query),
    ),
    responses((status = 200, body = ListScimGroupsResponse)),
    tag = "SCIM",
)]
pub async fn list_groups(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Query(params): Query<ScimListParams>,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    list_groups_by(&state, &base, ListQuery::from_params(&params)?).await
}

/// Every group, as JSON with its URL under `base`.
async fn groups(state: &AppState, base: &str) -> ScimResult<Vec<(&'static ResourceType, Value)>> {
    let groups = state.metastore.scim().list_groups().await?;
    Ok(groups
        .iter()
        .map(|g| (&GROUPS, located(&GROUPS, g, base)))
        .collect())
}

async fn list_groups_by(state: &AppState, base: &str, query: ListQuery) -> ScimResult<Scim> {
    Ok(Scim::ok(query.answer(groups(state, base).await?)))
}

#[utoipa::path(
    post, path = "/scim2/Groups/.search",
    operation_id = "scim_search_groups",
    responses((status = 200, body = ListScimGroupsResponse)),
    tag = "SCIM",
)]
pub async fn search_groups(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    list_groups_by(&state, &base, ListQuery::from_search(body)?).await
}

#[utoipa::path(
    get, path = "/scim2/Groups/{id}",
    operation_id = "scim_get_group",
    params(
        ("id" = String, Path),
        ("attributes"         = Option<String>, Query),
        ("excludedAttributes" = Option<String>, Query),
    ),
    responses(
        (status = 200, body = ScimGroup),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn get_group(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Path(id): Path<String>,
    Query(selection): Query<Selection>,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let group = state.metastore.scim().get_group(&id).await?;
    Ok(Scim::ok(
        selection.apply(&GROUPS, located(&GROUPS, &group, &base)),
    ))
}

#[utoipa::path(
    put, path = "/scim2/Groups/{id}",
    operation_id = "scim_put_group",
    params(("id" = String, Path)),
    request_body = ScimGroup,
    responses(
        (status = 200, body = ScimGroup),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn put_group(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Path(id): Path<String>,
    Query(selection): Query<Selection>,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let group = put_group_from(&state, &id, body).await?;
    Ok(Scim::ok(
        selection.apply(&GROUPS, located(&GROUPS, &group, &base)),
    ))
}

#[utoipa::path(
    patch, path = "/scim2/Groups/{id}",
    operation_id = "scim_patch_group",
    params(("id" = String, Path)),
    request_body = ScimPatchOp,
    responses(
        (status = 200, body = ScimGroup),
        (status = 400, description = "SCIM error: a malformed operation, or a member that is no user"),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn patch_group(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    Path(id): Path<String>,
    Query(selection): Query<Selection>,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let group = patch_group_from(&state, &id, body).await?;
    Ok(Scim::ok(
        selection.apply(&GROUPS, located(&GROUPS, &group, &base)),
    ))
}

#[utoipa::path(
    delete, path = "/scim2/Groups/{id}",
    operation_id = "scim_delete_group",
    params(("id" = String, Path)),
    responses(
        (status = 204, description = "Deleted, with its grants"),
        (status = 404, description = "SCIM error: not found"),
    ),
    tag = "SCIM",
)]
pub async fn delete_group(
    State(state): State<AppState>,
    principal: Principal,
    Path(id): Path<String>,
) -> ScimResult<StatusCode> {
    authz::require_admin(&state, &principal.username).await?;
    state.metastore.scim().delete_group(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// SCIM bulk (RFC 7644, section 3.7): operations on users and groups in one
// request. `failOnErrors` stops the batch after that many failures. Each
// operation has its own status, and a failed one a SCIM error.

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
    pub data: Option<Value>,
}

/// The most operations in a bulk request.
const MAX_BULK_OPERATIONS: usize = 100;

#[utoipa::path(
    post, path = "/scim2/Bulk",
    operation_id = "scim_bulk",
    responses((status = 200, description = "BulkResponse with per-op status")),
    tag = "SCIM",
)]
pub async fn bulk(
    State(state): State<AppState>,
    principal: Principal,
    BaseUrl(base): BaseUrl,
    ScimBody(body): ScimBody,
) -> ScimResult<Scim> {
    authz::require_admin(&state, &principal.username).await?;
    let req: ScimBulkRequest = serde_json::from_value(body).map_err(|e| {
        ScimError::bad_request("invalidSyntax", format!("Invalid BulkRequest: {e}"))
    })?;
    if req.operations.len() > MAX_BULK_OPERATIONS {
        return Err(ScimError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("A bulk request has at most {MAX_BULK_OPERATIONS} operations"),
        ));
    }
    let mut results = Vec::with_capacity(req.operations.len());
    let mut failures = 0usize;
    for op in req.operations {
        if req.fail_on_errors.is_some_and(|limit| failures >= limit) {
            break;
        }
        let mut entry = json!({ "method": op.method });
        if let Some(bulk_id) = &op.bulk_id {
            entry["bulkId"] = json!(bulk_id);
        }
        match apply_bulk_op(&state, &base, &op).await {
            Ok((status, location)) => {
                entry["status"] = json!(status.as_u16().to_string());
                if let Some(location) = location {
                    entry["location"] = json!(location);
                }
            }
            Err(e) => {
                failures += 1;
                entry["status"] = json!(e.status.as_u16().to_string());
                entry["response"] = e.body();
            }
        }
        results.push(entry);
    }
    Ok(Scim::ok(json!({
        "schemas": [schema_uri::BULK_RESPONSE],
        "Operations": results,
    })))
}

/// Run one bulk operation: its status, and the URL of the resource it
/// created or changed.
async fn apply_bulk_op(
    state: &AppState,
    base: &str,
    op: &ScimBulkOp,
) -> ScimResult<(StatusCode, Option<String>)> {
    let unsupported = || {
        ScimError::bad_request(
            "invalidPath",
            format!("No bulk operation is {} {}", op.method, op.path),
        )
    };
    let path = op.path.trim_start_matches('/');
    let (resource_type, id) = match path.split_once('/') {
        Some((endpoint, id)) => (endpoint, Some(id)),
        None => (path, None),
    };
    let users = resource_type.eq_ignore_ascii_case("Users");
    if !users && !resource_type.eq_ignore_ascii_case("Groups") {
        return Err(unsupported());
    }
    let data = || {
        op.data
            .clone()
            .ok_or_else(|| ScimError::bad_request("invalidSyntax", "The operation needs data"))
    };
    let location = |rt: &ResourceType, id: &str| Some(format!("{base}{}/{id}", rt.endpoint));
    let scim = state.metastore.scim();
    match (op.method.to_ascii_uppercase().as_str(), id) {
        ("POST", None) if users => {
            let user = create_user_from(state, data()?).await?;
            let id = user.id.unwrap_or_default();
            Ok((StatusCode::CREATED, location(&USERS, &id)))
        }
        ("POST", None) => {
            let group = create_group_from(state, data()?).await?;
            let id = group.id.unwrap_or_default();
            Ok((StatusCode::CREATED, location(&GROUPS, &id)))
        }
        ("PUT", Some(id)) if users => {
            put_user_from(state, id, data()?).await?;
            Ok((StatusCode::OK, location(&USERS, id)))
        }
        ("PUT", Some(id)) => {
            put_group_from(state, id, data()?).await?;
            Ok((StatusCode::OK, location(&GROUPS, id)))
        }
        ("PATCH", Some(id)) if users => {
            patch_user_from(state, id, data()?).await?;
            Ok((StatusCode::OK, location(&USERS, id)))
        }
        ("PATCH", Some(id)) => {
            patch_group_from(state, id, data()?).await?;
            Ok((StatusCode::OK, location(&GROUPS, id)))
        }
        ("DELETE", Some(id)) if users => {
            scim.delete_user(id).await?;
            Ok((StatusCode::NO_CONTENT, None))
        }
        ("DELETE", Some(id)) => {
            scim.delete_group(id).await?;
            Ok((StatusCode::NO_CONTENT, None))
        }
        _ => Err(unsupported()),
    }
}

// SCIM discovery (RFC 7644, section 4): Entra ID, Okta and authentik read
// it to learn what the server supports.

/// `resources` as a ListResponse.
fn discovery_list(resources: Vec<Value>) -> Value {
    json!({
        "schemas": [schema_uri::LIST_RESPONSE],
        "totalResults": resources.len(),
        "startIndex": 1,
        "itemsPerPage": resources.len(),
        "Resources": resources,
    })
}

#[utoipa::path(get, path = "/scim2/Schemas",
    operation_id = "scim_list_schemas",
    responses((status = 200, description = "The User and Group schemas, as a ListResponse")),
    tag = "SCIM",
)]
pub async fn list_schemas(BaseUrl(base): BaseUrl) -> Scim {
    Scim::ok(discovery_list(
        RESOURCE_TYPES
            .iter()
            .map(|rt| rt.schema_doc(&base))
            .collect(),
    ))
}

#[utoipa::path(get, path = "/scim2/Schemas/{id}",
    operation_id = "scim_get_schema",
    params(("id" = String, Path, description = "The schema URI")),
    responses(
        (status = 200, description = "The schema"),
        (status = 404, description = "SCIM error: no such schema"),
    ),
    tag = "SCIM",
)]
pub async fn get_schema(BaseUrl(base): BaseUrl, Path(id): Path<String>) -> ScimResult<Scim> {
    RESOURCE_TYPES
        .iter()
        .find(|rt| rt.schema.eq_ignore_ascii_case(&id))
        .map(|rt| Scim::ok(rt.schema_doc(&base)))
        .ok_or_else(|| ScimError::new(StatusCode::NOT_FOUND, format!("No schema is {id}")))
}

#[utoipa::path(get, path = "/scim2/ResourceTypes",
    operation_id = "scim_list_resource_types",
    responses((status = 200, description = "The User and Group resource types, as a ListResponse")),
    tag = "SCIM",
)]
pub async fn list_resource_types(BaseUrl(base): BaseUrl) -> Scim {
    Scim::ok(discovery_list(
        RESOURCE_TYPES
            .iter()
            .map(|rt| rt.resource_type_doc(&base))
            .collect(),
    ))
}

#[utoipa::path(get, path = "/scim2/ResourceTypes/{id}",
    operation_id = "scim_get_resource_type",
    params(("id" = String, Path, description = "User or Group")),
    responses(
        (status = 200, description = "The resource type"),
        (status = 404, description = "SCIM error: no such resource type"),
    ),
    tag = "SCIM",
)]
pub async fn get_resource_type(BaseUrl(base): BaseUrl, Path(id): Path<String>) -> ScimResult<Scim> {
    RESOURCE_TYPES
        .iter()
        .find(|rt| rt.id.eq_ignore_ascii_case(&id))
        .map(|rt| Scim::ok(rt.resource_type_doc(&base)))
        .ok_or_else(|| ScimError::new(StatusCode::NOT_FOUND, format!("No resource type is {id}")))
}

#[utoipa::path(get, path = "/scim2/ServiceProviderConfig",
    operation_id = "scim_service_provider_config",
    responses((status = 200, description = "SCIM ServiceProviderConfig")),
    tag = "SCIM",
)]
pub async fn service_provider_config(BaseUrl(base): BaseUrl) -> Scim {
    Scim::ok(scim_schema::service_provider_config(&base))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_user, list_users))
        .routes(routes!(search_users))
        .routes(routes!(get_user, patch_user, put_user, delete_user))
        .routes(routes!(me))
        .routes(routes!(create_group, list_groups))
        .routes(routes!(search_groups))
        .routes(routes!(search))
        .routes(routes!(get_group, patch_group, put_group, delete_group))
        .routes(routes!(list_schemas))
        .routes(routes!(get_schema))
        .routes(routes!(list_resource_types))
        .routes(routes!(get_resource_type))
        .routes(routes!(service_provider_config))
        .routes(routes!(bulk))
        .route("/scim2/{*rest}", any(unknown_endpoint))
}
