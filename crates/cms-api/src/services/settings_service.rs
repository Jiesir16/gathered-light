use sea_orm::DbErr;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::settings_dto::{SettingsResp, UpdateThemeReq, validate_theme},
    error::{AppError, AppResult},
    infra::cache,
    repositories::settings_repo,
};

const THEME_KEY: &str = "theme";
const PUBLIC_CACHE_KEY: &str = "cms:settings:public";
const DEFAULT_THEME: &str = "warm";

#[tracing::instrument(skip(state))]
pub async fn get_public(state: &AppState) -> AppResult<SettingsResp> {
    if let Some(settings) = cache::get(&state.redis, PUBLIC_CACHE_KEY).await {
        tracing::debug!(cache = "hit", key = PUBLIC_CACHE_KEY);
        return Ok(settings);
    }
    tracing::debug!(cache = "miss", key = PUBLIC_CACHE_KEY);

    let theme = settings_repo::get(&state.db, THEME_KEY)
        .await
        .map_err(db_err)?
        .filter(|theme| validate_theme(theme).is_ok())
        .unwrap_or_else(|| DEFAULT_THEME.to_owned());
    let settings = SettingsResp { theme };
    cache::set(&state.redis, PUBLIC_CACHE_KEY, &settings, 60).await;
    Ok(settings)
}

#[tracing::instrument(skip(state, req), fields(theme = %req.theme))]
pub async fn update_theme(state: &AppState, req: UpdateThemeReq) -> AppResult<SettingsResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    settings_repo::set(&state.db, THEME_KEY, &req.theme)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, "cms:settings:*").await;
    Ok(SettingsResp { theme: req.theme })
}

fn db_err(error: DbErr) -> AppError {
    AppError::Other(error.into())
}
