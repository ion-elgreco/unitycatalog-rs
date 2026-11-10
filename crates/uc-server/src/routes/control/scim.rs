//! SCIM 2.0 (RFC 7643 and RFC 7644) under the control plane: users,
//! groups, the signed-in user, searches, bulk requests and the discovery
//! endpoints. Only the metastore admin manages users and groups.
//!
//! Requests are read by the schemas of [`scim_schema`]: attribute names in
//! any case, unknown attributes left alone. Every error under `/scim2` is a
//! SCIM error with its HTTP status ([`error_layer`]).
//!
//! The metastore keeps the rules of the server's own user `admin`
//! ([`uc_metastore::ADMIN`]).

use std::collections::HashMap;
use std::convert::Infallible;

use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Request, State},
    http::{HeaderValue, StatusCode, header, request::Parts},
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
use uc_common::{CONTROL_BASE, Error};
use uc_metastore::{Page, ScimKey};

use crate::{
    authn::principal::Principal,
    authz,
    http::error::{Stray, format_errors, mounted},
    http::extract::{Path, Query},
    routes::control::scim_filter::{self, Filter},
    routes::control::scim_patch,
    routes::control::scim_schema::{
        self, GROUPS, MAX_BULK_OPERATIONS, MAX_RESULTS, RESOURCE_TYPES, ResourceType, USERS,
    },
    server::state::AppState,
};

/// A SCIM error (RFC 7644, section 3.12): its HTTP status, `scimType` and
/// detail. The body has only the attributes of the RFC: a strict client
/// refuses any other. The UC clients show `detail`.
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
            StatusCode::CONFLICT if e.code().is_already_exists() => Some("uniqueness"),
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
    location: Option<String>,
}

impl Scim {
    fn ok(body: Value) -> Self {
        Self {
            status: StatusCode::OK,
            body,
            location: None,
        }
    }

    /// 201 with `resource`, as [`located`] makes it, and its URL as
    /// `Location`, whatever attributes `selection` picks.
    fn created(resource_type: &ResourceType, resource: Value, selection: &Selection) -> Self {
        Self {
            status: StatusCode::CREATED,
            location: resource["meta"]["location"].as_str().map(str::to_string),
            body: selection.apply(resource_type, resource),
        }
    }
}

impl IntoResponse for Scim {
    fn into_response(self) -> Response {
        let location = (self.location).and_then(|l| HeaderValue::from_str(&l).ok());
        let mut response = scim_json(self.status, self.body);
        if let Some(location) = location {
            response.headers_mut().insert(header::LOCATION, location);
        }
        response
    }
}

/// Every error under `/scim2` is a SCIM error: those of the handlers, and
/// those of sign-in, request parsing, unknown paths and methods, the
/// request timeout and the rate limit, which come as UC errors or bare
/// statuses.
pub async fn error_layer(req: Request, next: Next) -> Response {
    let scim = mounted(req.uri().path(), CONTROL_BASE, "/scim2");
    let resp = next.run(req).await;
    let is_scim = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|t| t.as_bytes().starts_with(b"application/scim+json"));
    if !scim || is_scim {
        return resp;
    }
    format_errors(
        resp,
        |_| false,
        |stray: Stray| {
            match stray.status {
                StatusCode::BAD_REQUEST => ScimError::bad_request("invalidSyntax", stray.message),
                status => ScimError::new(status, stray.message),
            }
            .into_response()
        },
    )
    .await
}

/// A path under `/scim2` that names no endpoint.
async fn unknown_endpoint() -> ScimError {
    ScimError::new(StatusCode::NOT_FOUND, "No SCIM endpoint has this path")
}

/// The URL of the SCIM endpoints, under the server's public URL
/// ([`crate::http::public_url`]).
pub struct BaseUrl(String);

impl FromRequestParts<AppState> for BaseUrl {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let base = crate::http::public_url(state.auth.public_url.as_deref(), &parts.headers);
        Ok(Self(format!("{base}{CONTROL_BASE}/scim2")))
    }
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

/// `value`, a JSON object, read by `resource_type`, as a `T`.
fn decode<T: DeserializeOwned>(resource_type: &ResourceType, value: Value) -> ScimResult<T> {
    // serde would read a list as the fields of `T` in their order.
    if !value.is_object() {
        return Err(ScimError::bad_request(
            "invalidSyntax",
            format!("A {} is a JSON object", resource_type.id),
        ));
    }
    serde_json::from_value(resource_type.read(value)).map_err(|e| {
        ScimError::bad_request("invalidValue", format!("Invalid {}: {e}", resource_type.id))
    })
}

/// A new resource that a client sent, read by `resource_type`, as a `T`.
/// The server ignores `id`, `meta` and read-only attributes such as
/// `roles` (RFC 7643, section 2.2), so they may hold anything.
fn decode_new<T: DeserializeOwned>(resource_type: &ResourceType, body: Value) -> ScimResult<T> {
    let body = match resource_type.read(body) {
        Value::Object(mut object) => {
            object.retain(|name, _| {
                name != "id"
                    && name != "meta"
                    && scim_schema::find(resource_type.attributes, name)
                        .is_none_or(|a| a.mutability != scim_schema::Mutability::ReadOnly)
            });
            Value::Object(object)
        }
        body => body,
    };
    decode(resource_type, body)
}

/// The member `name` of `object`, in any case.
fn member(object: &Map<String, Value>, name: &str) -> Option<Value> {
    scim_filter::member(object, name).cloned()
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

    /// The page of a list without a filter.
    fn page(&self) -> Page {
        Page {
            offset: self.start - 1,
            limit: self.count,
        }
    }

    /// The ListResponse of `resources`, each as JSON with its type:
    /// `totalResults` counts the matches, the page holds `count` of them
    /// from `start`. Only the page gets its URLs under `base`
    /// ([`locate`]), unless the filter names one.
    fn answer(&self, resources: Vec<(&'static ResourceType, Value)>, base: &str) -> Value {
        let locate_all =
            (self.filter.as_ref()).is_some_and(|f| f.names("$ref") || f.names("location"));
        let located = |(resource_type, mut resource): (&'static ResourceType, Value)| {
            locate(resource_type, &mut resource, base);
            (resource_type, resource)
        };
        let matching: Vec<(&'static ResourceType, Value)> = resources
            .into_iter()
            .map(|r| if locate_all { located(r) } else { r })
            .filter(|(_, r)| {
                self.filter
                    .as_ref()
                    .is_none_or(|f| scim_filter::matches(f, r))
            })
            .collect();
        let total = matching.len();
        let page = matching
            .into_iter()
            .skip(self.start - 1)
            .take(self.count)
            .map(|r| if locate_all { r } else { located(r) })
            .collect();
        self.list_response(total, page)
    }

    /// The ListResponse of `page`, of `total` matches.
    fn list_response(&self, total: usize, page: Vec<(&ResourceType, Value)>) -> Value {
        let page: Vec<Value> = page
            .into_iter()
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

/// `resource` as JSON.
fn json_of<T: Serialize>(resource: &T) -> Value {
    serde_json::to_value(resource).unwrap_or_default()
}

/// Give `resource`, a resource of `resource_type` as JSON, its URL under
/// `base`, and each member its URL as `$ref`.
fn locate(resource_type: &ResourceType, resource: &mut Value, base: &str) {
    if let Some(members) = resource.get_mut("members").and_then(Value::as_array_mut) {
        for member in members.iter_mut().filter_map(Value::as_object_mut) {
            if let Some(id) = member.get("value").and_then(Value::as_str) {
                let url = format!("{base}{}/{id}", USERS.endpoint);
                member.insert("$ref".into(), json!(url));
            }
        }
    }
    if let Some(id) = resource["id"].as_str().map(str::to_string) {
        let location = format!("{base}{}/{id}", resource_type.endpoint);
        match resource.get_mut("meta").and_then(Value::as_object_mut) {
            Some(meta) => {
                meta.insert("location".into(), json!(location));
            }
            None => {
                resource["meta"] =
                    json!({ "resourceType": resource_type.id, "location": location });
            }
        }
    }
}

/// `resource` as JSON with its URLs under `base` ([`locate`]).
fn located<T: Serialize>(resource_type: &ResourceType, resource: &T, base: &str) -> Value {
    let mut value = json_of(resource);
    locate(resource_type, &mut value, base);
    value
}

/// `resource` with the attributes of `body` that a client may write: a
/// value replaces, `null` removes, an absent attribute stays (RFC 7644,
/// section 3.5.1, lets a server keep it). An `active` of `null` stays too:
/// without it a user is enabled, and a client that writes each attribute
/// that it has no value for as `null` would enable a disabled user. The body needs the required
/// attributes, `userName` or `displayName` (RFC 7643, section 2.2). An
/// `id` other than `id` is an error.
fn replaced(
    resource_type: &ResourceType,
    mut resource: Map<String, Value>,
    body: Value,
    id: &str,
) -> ScimResult<Map<String, Value>> {
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
    for (name, value) in body {
        match name.as_str() {
            "id" => {
                if value.as_str().is_some_and(|v| !v.eq_ignore_ascii_case(id)) {
                    return Err(ScimError::bad_request(
                        "mutability",
                        "The id of the body is not the id of the path",
                    ));
                }
            }
            "schemas" | "meta" => {}
            _ => {
                if let Some(attribute) = resource_type.writable(&name)
                    && !(attribute.name == "active" && value.is_null())
                {
                    set(&mut resource, attribute.name.to_string(), value);
                }
            }
        }
    }
    Ok(resource)
}

/// A patch may not remove a required attribute (RFC 7644, section
/// 3.5.2.2).
fn has_required(resource_type: &ResourceType, resource: &Map<String, Value>) -> ScimResult<()> {
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

/// `resource` as a JSON object.
fn object<T: Serialize>(resource: &T) -> ScimResult<Map<String, Value>> {
    match serde_json::to_value(resource) {
        Ok(Value::Object(object)) => Ok(object),
        _ => Err(Error::internal("a stored resource is not an object").into()),
    }
}

async fn create_user_from(state: &AppState, body: Value) -> ScimResult<ScimUser> {
    let user = new_user(decode_new(&USERS, body)?);
    Ok(state.metastore.scim().create_user(user).await?)
}

/// Change the user `id` with `change`, which makes the new user from the
/// stored one while the metastore holds it locked. When the principal
/// changes, the server logs what still names the old one.
async fn modify_user(
    state: &AppState,
    id: &str,
    change: impl FnOnce(&ScimUser) -> ScimResult<ScimUser> + Send,
) -> ScimResult<ScimUser> {
    let edit = state.metastore.scim().edit_user(id).await?;
    let old = edit.current().principal().to_string();
    let user = new_user(change(edit.current())?);
    let updated = edit.save(user).await?;
    if updated.principal() != old {
        principal_changed(state, &old, updated.principal());
    }
    Ok(updated)
}

/// The grants, owned objects and tokens of `old` moved to `new`. Settings
/// and Cedar policies that name `old` stay as they are: the server names
/// them in warnings.
fn principal_changed(state: &AppState, old: &str, new: &str) {
    tracing::info!(%old, %new, "a user's principal changed: its grants, objects and tokens moved");
    if state.names_admin(old) && !state.names_admin(new) {
        tracing::warn!(%old, %new, "auth.admin_principals names the old principal of a user");
    }
    for (file, names) in state.cedar_names.iter() {
        if names.contains(old) {
            tracing::warn!(
                file = %file.display(), %old, %new,
                "a Cedar policy file names the old principal of a user"
            );
        }
    }
}

async fn put_user_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimUser> {
    modify_user(state, id, |current| {
        let merged = replaced(&USERS, object(current)?, body, id)?;
        decode(&USERS, Value::Object(merged))
    })
    .await
}

async fn patch_user_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimUser> {
    let operations = patch_operations(body)?;
    modify_user(state, id, |current| {
        let mut resource = object(current)?;
        scim_patch::apply(&mut resource, &USERS, &operations)?;
        has_required(&USERS, &resource)?;
        decode(&USERS, Value::Object(resource))
    })
    .await
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
        &USERS,
        located(&USERS, &user, &base),
        &selection,
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

/// The term of `filter` that the database finds its matches by: `id`, the
/// name attribute of `resource_type`, `externalId` or a user's
/// `displayName` equal to a text. Of several, the one that finds the
/// fewest.
fn key_term<'f>(
    filter: Option<&'f Filter>,
    resource_type: &ResourceType,
) -> Option<(ScimKey, &'f str)> {
    let name = match resource_type.id {
        "User" => "userName",
        _ => "displayName",
    };
    let keys = [
        ("id", ScimKey::Id),
        (name, ScimKey::Name),
        ("externalId", ScimKey::ExternalId),
        ("displayName", ScimKey::DisplayName),
    ];
    let terms = filter?.required_texts();
    keys.into_iter().find_map(|(attribute, key)| {
        terms
            .iter()
            .find(|(a, sub, _)| sub.is_none() && a.eq_ignore_ascii_case(attribute))
            .map(|(_, _, text)| (key, *text))
    })
}

/// Whether no resource of `resource_type` matches `filter`: a term that
/// every match has names an attribute that the type lacks, or another
/// `meta.resourceType`.
fn rules_out(filter: Option<&Filter>, resource_type: &ResourceType) -> bool {
    let Some(filter) = filter else {
        return false;
    };
    (filter.required_texts())
        .into_iter()
        .any(|(attribute, sub, text)| {
            let other_type = attribute.eq_ignore_ascii_case("meta")
                && sub.is_some_and(|sub| sub.eq_ignore_ascii_case("resourceType"))
                && !text.eq_ignore_ascii_case(resource_type.id);
            other_type || !resource_type.has(attribute)
        })
}

/// The users that can match `filter`, as JSON: those of its [`key_term`],
/// else every user.
async fn users(
    state: &AppState,
    filter: Option<&Filter>,
) -> ScimResult<Vec<(&'static ResourceType, Value)>> {
    if rules_out(filter, &USERS) {
        return Ok(Vec::new());
    }
    let scim = state.metastore.scim();
    let users = match key_term(filter, &USERS) {
        Some((key, text)) => scim.users_with(key, text).await?,
        None => scim.list_users(None).await?,
    };
    Ok(users.iter().map(|u| (&USERS, json_of(u))).collect())
}

/// A list of users. Without a filter, the metastore reads only the page.
async fn list_users_by(state: &AppState, base: &str, query: ListQuery) -> ScimResult<Scim> {
    if query.filter.is_none() {
        let scim = state.metastore.scim();
        let total = scim.count_users().await?;
        let users = scim.list_users(Some(query.page())).await?;
        let page = (users.iter())
            .map(|u| (&USERS, located(&USERS, u, base)))
            .collect();
        return Ok(Scim::ok(query.list_response(total, page)));
    }
    let users = users(state, query.filter.as_ref()).await?;
    Ok(Scim::ok(query.answer(users, base)))
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
    if query.filter.is_none() {
        // The users, then the groups: the metastore reads only the page.
        let scim = state.metastore.scim();
        let (users, groups) = (scim.count_users().await?, scim.count_groups().await?);
        let Page { offset, limit } = query.page();
        let mut page: Vec<(&ResourceType, Value)> = (scim.list_users(Some(query.page())).await?)
            .iter()
            .map(|u| (&USERS, located(&USERS, u, &base)))
            .collect();
        let rest = Page {
            offset: offset.saturating_sub(users),
            limit: limit - page.len(),
        };
        if rest.limit > 0 {
            let groups = scim.list_groups(Some(rest)).await?;
            page.extend(groups.iter().map(|g| (&GROUPS, located(&GROUPS, g, &base))));
        }
        return Ok(Scim::ok(query.list_response(users + groups, page)));
    }
    let mut resources = users(&state, query.filter.as_ref()).await?;
    resources.extend(groups(&state, query.filter.as_ref()).await?);
    Ok(Scim::ok(query.answer(resources, &base)))
}

/// The group that a client sent, as the server keeps it: no id or times.
fn new_group(mut group: ScimGroup) -> ScimGroup {
    group.id = None;
    group.meta = None;
    group
}

async fn create_group_from(state: &AppState, body: Value) -> ScimResult<ScimGroup> {
    let group: ScimGroup = decode_new(&GROUPS, body)?;
    Ok(state
        .metastore
        .scim()
        .create_group(new_group(group))
        .await?)
}

/// Change the group `id` with `change`, as [`modify_user`] does a user.
async fn modify_group(
    state: &AppState,
    id: &str,
    change: impl FnOnce(&ScimGroup) -> ScimResult<ScimGroup> + Send,
) -> ScimResult<ScimGroup> {
    let edit = state.metastore.scim().edit_group(id).await?;
    let group = new_group(change(edit.current())?);
    Ok(edit.save(group).await?)
}

async fn put_group_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimGroup> {
    modify_group(state, id, |current| {
        let merged = replaced(&GROUPS, object(current)?, body, id)?;
        decode(&GROUPS, Value::Object(merged))
    })
    .await
}

async fn patch_group_from(state: &AppState, id: &str, body: Value) -> ScimResult<ScimGroup> {
    let operations = patch_operations(body)?;
    modify_group(state, id, |current| {
        let mut resource = object(current)?;
        scim_patch::apply(&mut resource, &GROUPS, &operations)?;
        has_required(&GROUPS, &resource)?;
        decode(&GROUPS, Value::Object(resource))
    })
    .await
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
        &GROUPS,
        located(&GROUPS, &group, &base),
        &selection,
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

/// The groups that can match `filter`, as [`users`] finds users.
async fn groups(
    state: &AppState,
    filter: Option<&Filter>,
) -> ScimResult<Vec<(&'static ResourceType, Value)>> {
    if rules_out(filter, &GROUPS) {
        return Ok(Vec::new());
    }
    let scim = state.metastore.scim();
    let groups = match key_term(filter, &GROUPS) {
        Some((key, text)) => scim.groups_with(key, text).await?,
        None => scim.list_groups(None).await?,
    };
    Ok(groups.iter().map(|g| (&GROUPS, json_of(g))).collect())
}

/// A list of groups, as [`list_users_by`] lists users.
async fn list_groups_by(state: &AppState, base: &str, query: ListQuery) -> ScimResult<Scim> {
    if query.filter.is_none() {
        let scim = state.metastore.scim();
        let total = scim.count_groups().await?;
        let groups = scim.list_groups(Some(query.page())).await?;
        let page = (groups.iter())
            .map(|g| (&GROUPS, located(&GROUPS, g, base)))
            .collect();
        return Ok(Scim::ok(query.list_response(total, page)));
    }
    let groups = groups(state, query.filter.as_ref()).await?;
    Ok(Scim::ok(query.answer(groups, base)))
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
// request, in order. `failOnErrors` stops the batch after that many
// failures. Each operation has its own status, and a failed one a SCIM
// error. `bulkId:<id>` in a path or in data names the resource that an
// earlier operation with that `bulkId` made.

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
    let req: ScimBulkRequest = serde_json::from_value(bulk_names(body)).map_err(|e| {
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
    let mut made = HashMap::new();
    // The batch stops at the first failure for `failOnErrors` 0 too.
    let limit = req.fail_on_errors.map(|limit| limit.max(1));
    for mut op in req.operations {
        if limit.is_some_and(|limit| failures >= limit) {
            break;
        }
        let mut entry = json!({ "method": op.method });
        if let Some(bulk_id) = &op.bulk_id {
            entry["bulkId"] = json!(bulk_id);
        }
        let applied = match resolve_bulk_ids(&mut op, &made) {
            Ok(()) => apply_bulk_op(&state, &op).await,
            Err(e) => Err(e),
        };
        match applied {
            Ok((status, resource)) => {
                entry["status"] = json!(status.as_u16().to_string());
                if let Some((resource_type, id)) = resource {
                    entry["location"] = json!(format!("{base}{}/{id}", resource_type.endpoint));
                    if status == StatusCode::CREATED
                        && let Some(bulk_id) = &op.bulk_id
                    {
                        made.insert(bulk_id.clone(), id);
                    }
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

/// `body`, a BulkRequest, with its names as [`ScimBulkRequest`] and
/// [`ScimBulkOp`] write them: a client writes them in any case.
fn bulk_names(body: Value) -> Value {
    fn named(object: Map<String, Value>, names: &[&str]) -> Map<String, Value> {
        let name_of = |key: String| match names.iter().find(|n| n.eq_ignore_ascii_case(&key)) {
            Some(name) => name.to_string(),
            None => key,
        };
        (object.into_iter())
            .map(|(key, value)| (name_of(key), value))
            .collect()
    }
    let Value::Object(body) = body else {
        return body;
    };
    let mut body = named(body, &["schemas", "failOnErrors", "Operations"]);
    if let Some(Value::Array(operations)) = body.get_mut("Operations") {
        for operation in operations.iter_mut().filter_map(Value::as_object_mut) {
            *operation = named(
                std::mem::take(operation),
                &["method", "path", "bulkId", "data"],
            );
        }
    }
    Value::Object(body)
}

/// The prefix of a reference to the resource of an earlier operation.
const BULK_ID: &str = "bulkId:";

/// `op` with each `bulkId:<id>` replaced by the id of the resource that
/// the operation `<id>` made, of `made`: in its path, and in its data where
/// it is a `value`, as that of a group member (RFC 7644, section 3.7.2).
/// Any other text of the data stays as it is.
fn resolve_bulk_ids(op: &mut ScimBulkOp, made: &HashMap<String, String>) -> ScimResult<()> {
    let resolve = |reference: &str| {
        made.get(reference).cloned().ok_or_else(|| {
            ScimError::new(
                StatusCode::CONFLICT,
                format!("No earlier operation made the resource of {BULK_ID}{reference}"),
            )
        })
    };
    if let Some((before, reference)) = op.path.split_once(BULK_ID) {
        let (reference, after) = reference.split_at(reference.find('/').unwrap_or(reference.len()));
        op.path = format!("{before}{}{after}", resolve(reference)?);
    }
    fn walk(value: &mut Value, resolve: &dyn Fn(&str) -> ScimResult<String>) -> ScimResult<()> {
        match value {
            Value::Array(items) => {
                for item in items {
                    walk(item, resolve)?;
                }
            }
            Value::Object(object) => {
                for (name, item) in object.iter_mut() {
                    match item {
                        Value::String(text) if name.eq_ignore_ascii_case("value") => {
                            if let Some(reference) = text.strip_prefix(BULK_ID) {
                                *text = resolve(reference)?;
                            }
                        }
                        item => walk(item, resolve)?,
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    match op.data.as_mut() {
        Some(data) => walk(data, &resolve),
        None => Ok(()),
    }
}

/// Run one bulk operation: its status, and the type and id of the
/// resource it created or changed.
async fn apply_bulk_op(
    state: &AppState,
    op: &ScimBulkOp,
) -> ScimResult<(StatusCode, Option<(&'static ResourceType, String)>)> {
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
    let resource = |rt: &'static ResourceType, id: &str| Some((rt, id.to_string()));
    match (op.method.to_ascii_uppercase().as_str(), id) {
        ("POST", None) if users => {
            let user = create_user_from(state, data()?).await?;
            let id = user.id.unwrap_or_default();
            Ok((StatusCode::CREATED, resource(&USERS, &id)))
        }
        ("POST", None) => {
            let group = create_group_from(state, data()?).await?;
            let id = group.id.unwrap_or_default();
            Ok((StatusCode::CREATED, resource(&GROUPS, &id)))
        }
        ("PUT", Some(id)) if users => {
            put_user_from(state, id, data()?).await?;
            Ok((StatusCode::OK, resource(&USERS, id)))
        }
        ("PUT", Some(id)) => {
            put_group_from(state, id, data()?).await?;
            Ok((StatusCode::OK, resource(&GROUPS, id)))
        }
        ("PATCH", Some(id)) if users => {
            patch_user_from(state, id, data()?).await?;
            Ok((StatusCode::OK, resource(&USERS, id)))
        }
        ("PATCH", Some(id)) => {
            patch_group_from(state, id, data()?).await?;
            Ok((StatusCode::OK, resource(&GROUPS, id)))
        }
        ("DELETE", Some(id)) if users => {
            state.metastore.scim().delete_user(id).await?;
            Ok((StatusCode::NO_CONTENT, None))
        }
        ("DELETE", Some(id)) => {
            state.metastore.scim().delete_group(id).await?;
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
pub async fn service_provider_config(
    State(state): State<AppState>,
    BaseUrl(base): BaseUrl,
) -> Scim {
    Scim::ok(scim_schema::service_provider_config(
        &base,
        state.http.max_request_length,
    ))
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
