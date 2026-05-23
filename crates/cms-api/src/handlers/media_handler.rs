use axum::{Json, extract::State};

use crate::{
    bootstrap::AppState,
    dto::media_dto::{CompleteReq, CompleteResp, PresignReq, PresignResp},
    error::AppResult,
    services::media_service,
};

pub async fn presign(
    State(state): State<AppState>,
    Json(req): Json<PresignReq>,
) -> AppResult<Json<PresignResp>> {
    Ok(Json(media_service::presign_upload(&state, req).await?))
}

pub async fn complete(
    State(state): State<AppState>,
    Json(req): Json<CompleteReq>,
) -> AppResult<Json<CompleteResp>> {
    Ok(Json(media_service::complete_upload(&state, req).await?))
}
