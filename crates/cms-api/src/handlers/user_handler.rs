use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::{
    bootstrap::AppState,
    dto::user_dto::{
        NewUserReq, ResetPasswordReq, UpdateUserReq, UserListItem, UserListQuery, UserListResp,
    },
    error::AppResult,
    infra::jwt::Claims,
    services::user_service,
};

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<UserListQuery>,
) -> AppResult<Json<UserListResp>> {
    Ok(Json(user_service::list_users(&state, query).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewUserReq>,
) -> AppResult<(StatusCode, Json<UserListItem>)> {
    let user = user_service::create_user(&state, req).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserReq>,
) -> AppResult<Json<UserListItem>> {
    Ok(Json(
        user_service::update_user(&state, &claims, id, req).await?,
    ))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ResetPasswordReq>,
) -> AppResult<StatusCode> {
    user_service::reset_password(&state, id, req).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    user_service::delete_user(&state, &claims, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
