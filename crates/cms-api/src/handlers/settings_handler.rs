use axum::{Json, extract::State};

use crate::{
    bootstrap::AppState,
    dto::settings_dto::{
        SettingsResp, UpdateBrandEffectReq, UpdateHeroReq, UpdateRangeReq, UpdateThemeReq,
    },
    error::AppResult,
    services::settings_service,
};

pub async fn get_public(State(state): State<AppState>) -> AppResult<Json<SettingsResp>> {
    Ok(Json(settings_service::get_public(&state).await?))
}

pub async fn update_theme(
    State(state): State<AppState>,
    Json(req): Json<UpdateThemeReq>,
) -> AppResult<Json<SettingsResp>> {
    Ok(Json(settings_service::update_theme(&state, req).await?))
}

pub async fn update_range(
    State(state): State<AppState>,
    Json(req): Json<UpdateRangeReq>,
) -> AppResult<Json<SettingsResp>> {
    Ok(Json(settings_service::update_range(&state, req).await?))
}

pub async fn update_hero(
    State(state): State<AppState>,
    Json(req): Json<UpdateHeroReq>,
) -> AppResult<Json<SettingsResp>> {
    Ok(Json(settings_service::update_hero(&state, req).await?))
}

pub async fn update_brand_effect(
    State(state): State<AppState>,
    Json(req): Json<UpdateBrandEffectReq>,
) -> AppResult<Json<SettingsResp>> {
    Ok(Json(
        settings_service::update_brand_effect(&state, req).await?,
    ))
}
