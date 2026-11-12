//! The admin side of sharing: shares, their permissions and recipients,
//! under the UC API. The routes and the models are those of
//! `opensharing-admin-api`, in the shape of the Databricks sharing API; the
//! state is in the metastore.
//!
//! A recipient gets its bearer token from an activation link, once
//! ([`ACTIVATION_PATH`]). The admin sees the link, not the token.

use std::convert::Infallible;

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, Request, State},
    http::{HeaderMap, StatusCode, request::Parts},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use opensharing_admin_api::{
    RecipientHandler, ShareHandler,
    recipients::v1::{
        AuthenticationType, CreateRecipientRequest, DeleteRecipientRequest, GetRecipientRequest,
        ListRecipientsRequest, ListRecipientsResponse, Recipient, RecipientToken,
        RotateRecipientTokenRequest, SecurablePropertiesKvPairs, UpdateRecipientRequest,
    },
    shares::v1::{
        Action, CreateShareRequest, DataObject, DataObjectStatus, DataObjectType,
        DeleteShareRequest, GetPermissionsRequest, GetPermissionsResponse, GetShareRequest,
        HistoryStatus, ListSharesRequest, ListSharesResponse, PrivilegeAssignment, Share,
        UpdatePermissionsRequest, UpdatePermissionsResponse, UpdateShareRequest,
    },
};
use serde::Serialize;
use uc_common::{Error, ErrorResponse, Pagination, Result};
use uc_metastore::{
    NewRecipient, ObjectChange, RecipientUpdate, ShareObject, ShareObjectType, ShareUpdate,
};

use super::DELTA_SHARING_PREFIX;
use crate::{
    authn::principal::Principal,
    authz,
    http::error::{ApiError, ApiResult, Stray, format_errors},
    server::state::AppState,
};

/// Where the activation links are, under the server's URL.
pub const ACTIVATION_PATH: &str = "/api/1.0/opensharing/activations";

/// The privilege of a recipient on a share.
const SELECT: &str = "SELECT";

/// The handlers of the admin routes.
#[derive(Clone)]
pub struct SharingAdmin {
    state: AppState,
}

impl SharingAdmin {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// The name that changes are recorded with, if the caller is an admin.
    async fn admin<'a>(&self, caller: &'a Caller) -> Result<Option<&'a str>> {
        authz::require_admin(&self.state, &caller.principal.username).await?;
        Ok(caller.principal.actor())
    }
}

/// Who sent a request, and the URL that the request came to.
pub struct Caller {
    principal: Principal,
    server: String,
}

impl FromRequestParts<SharingAdmin> for Caller {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        admin: &SharingAdmin,
    ) -> Result<Self, Infallible> {
        let Ok(principal) = Principal::from_request_parts(parts, admin).await;
        let public_url = admin.state.auth.public_url.as_deref();
        Ok(Self {
            principal,
            server: crate::http::public_url(public_url, &parts.headers),
        })
    }
}

fn page(max_results: Option<i32>, page_token: Option<String>) -> Pagination {
    Pagination {
        max_results: max_results.map(i64::from),
        page_token,
    }
}

fn not_supported(what: &str) -> Error {
    Error::invalid_argument(format!("{what} is not supported"))
}

/// An empty text of a request is a value that was not given.
fn given(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

fn object_type(object: &DataObject) -> Result<ShareObjectType> {
    match object.data_object_type.as_known() {
        Some(DataObjectType::TABLE) => Ok(ShareObjectType::Table),
        Some(DataObjectType::VOLUME) => Ok(ShareObjectType::Volume),
        Some(DataObjectType::AGENT_SKILL) => Ok(ShareObjectType::AgentSkill),
        _ => Err(Error::invalid_argument(format!(
            "'{}': data_object_type must be TABLE, VOLUME or AGENT_SKILL",
            object.name
        ))),
    }
}

/// The object of a request. Without `shared_as`, a recipient sees the
/// schema and the name that the object has in the catalog.
fn share_object(object: DataObject) -> Result<ShareObject> {
    let object_type = object_type(&object)?;
    if !object.partitions.is_empty() {
        return Err(not_supported("partitions"));
    }
    if object.cdf_enabled == Some(true) {
        return Err(not_supported("cdf_enabled"));
    }
    let alias = (object.shared_as.as_deref())
        .or(object.string_shared_as.as_deref())
        .filter(|alias| !alias.is_empty());
    let parts: Vec<&str> = alias.unwrap_or(&object.name).split('.').collect();
    let [.., schema, name] = parts.as_slice() else {
        return Err(Error::invalid_argument(match alias {
            Some(alias) => format!("shared_as '{alias}' must be <schema>.<name>"),
            None => format!("'{}' must be <catalog>.<schema>.<name>", object.name),
        }));
    };
    if alias.is_some() && parts.len() != 2 || schema.is_empty() || name.is_empty() {
        return Err(Error::invalid_argument(format!(
            "shared_as '{}' must be <schema>.<name>",
            alias.unwrap_or(&object.name)
        )));
    }
    let history = object
        .history_data_sharing_status
        .and_then(|s| s.as_known());
    Ok(ShareObject {
        object_type,
        shared_as_schema: schema.to_string(),
        shared_as_name: name.to_string(),
        history_shared: history == Some(HistoryStatus::ENABLED),
        start_version: object.start_version,
        comment: object.comment,
        added_at: 0,
        added_by: None,
        source_full_name: object.name,
    })
}

fn data_object(object: ShareObject) -> DataObject {
    let alias = format!("{}.{}", object.shared_as_schema, object.shared_as_name);
    let table = object.object_type == ShareObjectType::Table;
    let history = match object.history_shared {
        true => HistoryStatus::ENABLED,
        false => HistoryStatus::DISABLED,
    };
    DataObject {
        name: object.source_full_name,
        data_object_type: match object.object_type {
            ShareObjectType::Table => DataObjectType::TABLE,
            ShareObjectType::Volume => DataObjectType::VOLUME,
            ShareObjectType::AgentSkill => DataObjectType::AGENT_SKILL,
        }
        .into(),
        added_at: Some(object.added_at),
        added_by: object.added_by,
        comment: object.comment,
        shared_as: table.then(|| alias.clone()),
        string_shared_as: (!table).then_some(alias),
        history_data_sharing_status: table.then(|| history.into()),
        start_version: object.start_version,
        status: Some(DataObjectStatus::ACTIVE.into()),
        ..Default::default()
    }
}

/// The share as an answer, with its objects if `objects`.
fn share(share: uc_metastore::Share, objects: bool) -> Share {
    Share {
        id: Some(share.id),
        name: share.name,
        objects: match objects {
            true => share.objects.into_iter().map(data_object).collect(),
            false => Vec::new(),
        },
        owner: share.owner,
        comment: share.comment,
        created_at: Some(share.created_at),
        created_by: share.created_by,
        updated_at: Some(share.updated_at),
        updated_by: share.updated_by,
        ..Default::default()
    }
}

fn properties(
    pairs: Option<&SecurablePropertiesKvPairs>,
) -> Option<std::collections::BTreeMap<String, String>> {
    pairs.map(|pairs| {
        (pairs.properties.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    })
}

/// The recipient as an answer. A token that waits for its activation has
/// its link, at the URL `server`.
fn recipient(recipient: uc_metastore::Recipient, server: &str) -> Recipient {
    let tokens = (recipient.tokens.into_iter())
        .map(|token| RecipientToken {
            id: token.id,
            created_at: token.created_at,
            created_by: token.created_by.unwrap_or_default(),
            activation_url: (token.activation_code)
                .map(|code| format!("{server}{ACTIVATION_PATH}/{code}"))
                .unwrap_or_default(),
            expiration_time: token.expiration_time.unwrap_or_default(),
            updated_at: token.updated_at,
            updated_by: token.updated_by.unwrap_or_default(),
            ..Default::default()
        })
        .collect();
    let pairs = (!recipient.properties.is_empty()).then(|| SecurablePropertiesKvPairs {
        properties: recipient.properties.into_iter().collect(),
        ..Default::default()
    });
    Recipient {
        id: Some(recipient.id),
        name: recipient.name,
        authentication_type: AuthenticationType::TOKEN.into(),
        owner: recipient.owner,
        comment: recipient.comment,
        properties_kvpairs: pairs.into(),
        created_at: Some(recipient.created_at),
        created_by: recipient.created_by,
        tokens,
        updated_at: Some(recipient.updated_at),
        updated_by: recipient.updated_by,
        ..Default::default()
    }
}

impl SharingAdmin {
    /// The recipients that read the share, each with the one privilege.
    async fn assignments(&self, share: &str) -> Result<Vec<PrivilegeAssignment>> {
        let recipients = self
            .state
            .metastore
            .sharing()
            .share_recipients(share)
            .await?;
        Ok((recipients.into_iter())
            .map(|principal| PrivilegeAssignment {
                principal,
                privileges: vec![SELECT.to_string()],
                ..Default::default()
            })
            .collect())
    }
}

#[async_trait]
impl ShareHandler<Caller> for SharingAdmin {
    type Error = ApiError;

    async fn list_shares(
        &self,
        request: ListSharesRequest,
        caller: Caller,
    ) -> ApiResult<ListSharesResponse> {
        self.admin(&caller).await?;
        let page = page(request.max_results, request.page_token);
        let (shares, next_page_token) = self.state.metastore.sharing().list_shares(&page).await?;
        Ok(ListSharesResponse {
            shares: shares.into_iter().map(|s| share(s, false)).collect(),
            next_page_token,
            ..Default::default()
        })
    }

    async fn create_share(&self, request: CreateShareRequest, caller: Caller) -> ApiResult<Share> {
        let actor = self.admin(&caller).await?;
        if request.storage_root.is_some() {
            return Err(not_supported("storage_root").into());
        }
        let sharing = self.state.metastore.sharing();
        let created = (sharing.create_share(&request.name, request.comment, actor)).await?;
        Ok(share(created, true))
    }

    async fn get_share(&self, request: GetShareRequest, caller: Caller) -> ApiResult<Share> {
        self.admin(&caller).await?;
        let found = self
            .state
            .metastore
            .sharing()
            .get_share(&request.name)
            .await?;
        Ok(share(found, request.include_shared_data == Some(true)))
    }

    async fn update_share(&self, request: UpdateShareRequest, caller: Caller) -> ApiResult<Share> {
        let actor = self.admin(&caller).await?;
        let mut objects = Vec::with_capacity(request.updates.len());
        for update in request.updates {
            let object = (update.data_object.into_option())
                .ok_or_else(|| Error::invalid_argument("an update needs a data_object"))?;
            objects.push(match update.action.as_known() {
                Some(Action::ADD) => ObjectChange::Add(share_object(object)?),
                Some(Action::UPDATE) => ObjectChange::Update(share_object(object)?),
                Some(Action::REMOVE) => ObjectChange::Remove(object_type(&object)?, object.name),
                _ => {
                    let message = "the action of an update must be ADD, REMOVE or UPDATE";
                    return Err(Error::invalid_argument(message).into());
                }
            });
        }
        let update = ShareUpdate {
            new_name: request.new_name,
            owner: request.owner,
            comment: request.comment,
            objects,
        };
        let sharing = self.state.metastore.sharing();
        let updated = sharing.update_share(&request.name, update, actor).await?;
        Ok(share(updated, true))
    }

    async fn delete_share(&self, request: DeleteShareRequest, caller: Caller) -> ApiResult<()> {
        self.admin(&caller).await?;
        Ok(self
            .state
            .metastore
            .sharing()
            .delete_share(&request.name)
            .await?)
    }

    async fn get_permissions(
        &self,
        request: GetPermissionsRequest,
        caller: Caller,
    ) -> ApiResult<GetPermissionsResponse> {
        self.admin(&caller).await?;
        Ok(GetPermissionsResponse {
            privilege_assignments: self.assignments(&request.name).await?,
            ..Default::default()
        })
    }

    async fn update_permissions(
        &self,
        request: UpdatePermissionsRequest,
        caller: Caller,
    ) -> ApiResult<UpdatePermissionsResponse> {
        let actor = self.admin(&caller).await?;
        let sharing = self.state.metastore.sharing();
        for change in &request.changes {
            if let Some(other) = (change.add.iter().chain(&change.remove)).find(|p| *p != SELECT) {
                let message = format!("a recipient has {SELECT} on a share, not {other}");
                return Err(Error::invalid_argument(message).into());
            }
        }
        for change in &request.changes {
            if !change.add.is_empty() {
                (sharing.grant_share(&request.name, &change.principal, actor)).await?;
            }
            if !change.remove.is_empty() {
                (sharing.revoke_share(&request.name, &change.principal)).await?;
            }
        }
        let privilege_assignments = match request.omit_permissions_list {
            Some(true) => Vec::new(),
            _ => self.assignments(&request.name).await?,
        };
        Ok(UpdatePermissionsResponse {
            privilege_assignments,
            ..Default::default()
        })
    }
}

#[async_trait]
impl RecipientHandler<Caller> for SharingAdmin {
    type Error = ApiError;

    async fn list_recipients(
        &self,
        request: ListRecipientsRequest,
        caller: Caller,
    ) -> ApiResult<ListRecipientsResponse> {
        self.admin(&caller).await?;
        let page = page(request.max_results, request.page_token);
        let sharing = self.state.metastore.sharing();
        let (recipients, next_page_token) = sharing.list_recipients(&page).await?;
        Ok(ListRecipientsResponse {
            recipients: (recipients.into_iter())
                .map(|r| recipient(r, &caller.server))
                .collect(),
            next_page_token,
            ..Default::default()
        })
    }

    async fn create_recipient(
        &self,
        request: CreateRecipientRequest,
        caller: Caller,
    ) -> ApiResult<Recipient> {
        let actor = self.admin(&caller).await?;
        match request.authentication_type.as_known() {
            Some(
                AuthenticationType::TOKEN | AuthenticationType::AUTHENTICATION_TYPE_UNSPECIFIED,
            ) => {}
            _ => return Err(Error::invalid_argument("authentication_type must be TOKEN").into()),
        }
        let new = NewRecipient {
            name: request.name,
            comment: request.comment,
            owner: given(request.owner),
            properties: properties(request.properties_kvpairs.as_option()).unwrap_or_default(),
            expiration_time: request.expiration_time,
        };
        let created = self
            .state
            .metastore
            .sharing()
            .create_recipient(new, actor)
            .await?;
        Ok(recipient(created, &caller.server))
    }

    async fn get_recipient(
        &self,
        request: GetRecipientRequest,
        caller: Caller,
    ) -> ApiResult<Recipient> {
        self.admin(&caller).await?;
        let found = self
            .state
            .metastore
            .sharing()
            .get_recipient(&request.name)
            .await?;
        Ok(recipient(found, &caller.server))
    }

    async fn update_recipient(
        &self,
        request: UpdateRecipientRequest,
        caller: Caller,
    ) -> ApiResult<Recipient> {
        let actor = self.admin(&caller).await?;
        let update = RecipientUpdate {
            new_name: request.new_name,
            owner: request.owner,
            comment: request.comment,
            properties: properties(request.properties_kvpairs.as_option()),
            expiration_time: request.expiration_time,
        };
        let sharing = self.state.metastore.sharing();
        let updated = sharing
            .update_recipient(&request.name, update, actor)
            .await?;
        Ok(recipient(updated, &caller.server))
    }

    async fn delete_recipient(
        &self,
        request: DeleteRecipientRequest,
        caller: Caller,
    ) -> ApiResult<()> {
        self.admin(&caller).await?;
        Ok(self
            .state
            .metastore
            .sharing()
            .delete_recipient(&request.name)
            .await?)
    }

    async fn rotate_recipient_token(
        &self,
        request: RotateRecipientTokenRequest,
        caller: Caller,
    ) -> ApiResult<Recipient> {
        let actor = self.admin(&caller).await?;
        let grace = request.existing_token_expire_in_seconds;
        let sharing = self.state.metastore.sharing();
        let rotated = (sharing.rotate_recipient_token(&request.name, grace, actor)).await?;
        Ok(recipient(rotated, &caller.server))
    }
}

/// The profile file of a Delta Sharing client.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    share_credentials_version: u32,
    endpoint: String,
    bearer_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiration_time: Option<String>,
}

/// `GET /api/1.0/opensharing/activations/{code}`: the profile of the
/// recipient, once. The request has no token: the code is the secret.
async fn activate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> ApiResult<Json<Profile>> {
    let activated = state.metastore.sharing().activate(&code).await?;
    let server = crate::http::public_url(state.auth.public_url.as_deref(), &headers);
    let end = (activated.expiration_time)
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    Ok(Json(Profile {
        share_credentials_version: 1,
        endpoint: format!("{server}{DELTA_SHARING_PREFIX}"),
        bearer_token: activated.bearer_token,
        expiration_time: end,
    }))
}

/// A request that the models refuse, such as a body that is not JSON, gets
/// the error body of the UC API. The extractors of the models answer with
/// plain text.
async fn request_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let refused = matches!(
        response.status(),
        StatusCode::BAD_REQUEST
            | StatusCode::UNPROCESSABLE_ENTITY
            | StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    if !refused || response.extensions().get::<ErrorResponse>().is_some() {
        return response;
    }
    let invalid = |stray: Stray| ApiError(Error::invalid_argument(stray.message)).into_response();
    format_errors(response, |_| false, invalid).await
}

/// The admin routes under the UC API at `api_base`, and the activation
/// route.
pub fn router(state: &AppState, api_base: &str) -> Router<AppState> {
    let admin = opensharing_admin_api::admin_router::<_, Caller>(SharingAdmin::new(state.clone()))
        .layer(middleware::from_fn(request_errors));
    Router::new()
        .nest(api_base, admin.with_state(()))
        .route(&format!("{ACTIVATION_PATH}/{{code}}"), get(activate))
}
