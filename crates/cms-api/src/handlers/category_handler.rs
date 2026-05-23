use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    bootstrap::AppState,
    dto::{
        category_dto::{CategoryAdminDto, NewCategoryReq, UpdateCategoryReq},
        photo_dto::CategoryDto,
    },
    error::AppResult,
    services::category_service,
};

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<CategoryDto>>> {
    Ok(Json(category_service::list_public(&state).await?))
}

pub async fn list_admin(State(state): State<AppState>) -> AppResult<Json<Vec<CategoryAdminDto>>> {
    Ok(Json(category_service::list_admin_categories(&state).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewCategoryReq>,
) -> AppResult<(StatusCode, Json<CategoryAdminDto>)> {
    let category = category_service::create_category(&state, req).await?;
    Ok((StatusCode::CREATED, Json(category)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCategoryReq>,
) -> AppResult<Json<CategoryAdminDto>> {
    Ok(Json(
        category_service::update_category(&state, id, req).await?,
    ))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<StatusCode> {
    category_service::delete_category(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
