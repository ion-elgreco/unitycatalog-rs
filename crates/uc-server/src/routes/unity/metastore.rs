use crate::http::extract::Json;
use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use uc_api::GetMetastoreSummaryResponse;

use crate::{http::error::ApiResult, server::state::AppState};

#[utoipa::path(
    get, path = "/metastore_summary",
    responses((status = 200, body = GetMetastoreSummaryResponse)),
    tag = "Metastore",
)]
pub async fn metastore_summary(
    State(state): State<AppState>,
) -> ApiResult<Json<GetMetastoreSummaryResponse>> {
    let id = state.metastore.permissions().metastore_id().await?;
    Ok(Json(GetMetastoreSummaryResponse {
        metastore_id: Some(id),
    }))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(metastore_summary))
}
