use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::{
    bootstrap::AppState,
    dto::tag_dto::{NewTagReq, TagDto, TagListQuery, TagListResp, UpdateTagReq},
    error::AppResult,
    services::tag_service,
};

pub async fn list_public(State(state): State<AppState>) -> AppResult<Json<Vec<TagDto>>> {
    Ok(Json(tag_service::list_public_tags(&state).await?))
}

pub async fn list_admin(
    State(state): State<AppState>,
    Query(query): Query<TagListQuery>,
) -> AppResult<Json<TagListResp>> {
    Ok(Json(tag_service::list_admin_tags(&state, query).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewTagReq>,
) -> AppResult<(StatusCode, Json<TagDto>)> {
    let tag = tag_service::create_tag(&state, req).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTagReq>,
) -> AppResult<Json<TagDto>> {
    Ok(Json(tag_service::update_tag(&state, id, req).await?))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<StatusCode> {
    tag_service::delete_tag(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
