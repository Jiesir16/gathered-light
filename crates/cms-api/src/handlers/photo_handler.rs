use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::{
    bootstrap::AppState,
    dto::media_dto::RepairResp,
    dto::photo_dto::{
        BulkDeleteReq, BulkPrivacyReq, BulkResp, BulkTagsReq, PhotoDto, PhotoListQuery,
        PhotoListResp, PhotoQuery, PhotoReq, PrivacyReq, UnlockReq, UnlockResp,
    },
    error::AppResult,
    services::photo_service,
};

pub async fn list_public(
    State(state): State<AppState>,
    Query(query): Query<PhotoQuery>,
) -> AppResult<Json<Vec<PhotoDto>>> {
    Ok(Json(photo_service::list_public(&state, query).await?))
}

pub async fn list_admin(
    State(state): State<AppState>,
    Query(query): Query<PhotoListQuery>,
) -> AppResult<Json<PhotoListResp>> {
    Ok(Json(photo_service::list_admin(&state, query).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<PhotoReq>,
) -> AppResult<(StatusCode, Json<PhotoDto>)> {
    let photo = photo_service::create(&state, req).await?;
    Ok((StatusCode::CREATED, Json(photo)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<PhotoReq>,
) -> AppResult<Json<PhotoDto>> {
    Ok(Json(photo_service::update(&state, id, req).await?))
}

pub async fn update_privacy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<PrivacyReq>,
) -> AppResult<Json<PhotoDto>> {
    Ok(Json(photo_service::update_privacy(&state, id, req).await?))
}

pub async fn recover_urls(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<PhotoDto>> {
    Ok(Json(photo_service::recover_urls(&state, id).await?))
}

/// 修复历史「桶名双写」数据（admin「修复历史数据」按钮）。
pub async fn repair_legacy(State(state): State<AppState>) -> AppResult<Json<RepairResp>> {
    Ok(Json(photo_service::repair_legacy_keys(&state).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<StatusCode> {
    photo_service::delete(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn bulk_delete(
    State(state): State<AppState>,
    Json(req): Json<BulkDeleteReq>,
) -> AppResult<Json<BulkResp>> {
    Ok(Json(photo_service::bulk_delete(&state, req).await?))
}

pub async fn bulk_update_privacy(
    State(state): State<AppState>,
    Json(req): Json<BulkPrivacyReq>,
) -> AppResult<Json<BulkResp>> {
    Ok(Json(photo_service::bulk_update_privacy(&state, req).await?))
}

pub async fn bulk_set_tags(
    State(state): State<AppState>,
    Json(req): Json<BulkTagsReq>,
) -> AppResult<Json<BulkResp>> {
    Ok(Json(photo_service::bulk_set_tags(&state, req).await?))
}

pub async fn reset(State(state): State<AppState>) -> AppResult<Json<Vec<PhotoDto>>> {
    Ok(Json(photo_service::reset(&state).await?))
}

pub async fn unlock(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UnlockReq>,
) -> AppResult<Json<UnlockResp>> {
    Ok(Json(photo_service::unlock(&state, id, req).await?))
}
