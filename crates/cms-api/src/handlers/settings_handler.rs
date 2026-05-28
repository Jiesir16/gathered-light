use axum::{Json, extract::State};

use crate::{
    bootstrap::AppState,
    dto::settings_dto::{SettingsResp, UpdateThemeReq},
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
