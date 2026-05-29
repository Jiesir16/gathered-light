use sea_orm::DbErr;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::settings_dto::{
        HeroCopy, SettingsResp, UpdateHeroReq, UpdateRangeReq, UpdateThemeReq, validate_range,
        validate_theme,
    },
    error::{AppError, AppResult},
    infra::cache,
    repositories::settings_repo,
};

const THEME_KEY: &str = "theme";
const RANGE_KEY: &str = "range";
const HERO_KEY: &str = "hero";
const PUBLIC_CACHE_KEY: &str = "cms:settings:public";
const DEFAULT_THEME: &str = "warm";
const DEFAULT_RANGE: &str = "2025 - 2026";

#[tracing::instrument(skip(state))]
pub async fn get_public(state: &AppState) -> AppResult<SettingsResp> {
    if let Some(settings) = cache::get(&state.redis, PUBLIC_CACHE_KEY).await {
        tracing::debug!(cache = "hit", key = PUBLIC_CACHE_KEY);
        return Ok(settings);
    }
    tracing::debug!(cache = "miss", key = PUBLIC_CACHE_KEY);

    let settings = read_public_settings(state).await?;
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
    read_public_settings(state).await
}

#[tracing::instrument(skip(state, req), fields(range = %req.range))]
pub async fn update_range(state: &AppState, req: UpdateRangeReq) -> AppResult<SettingsResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let range = req.range.trim();
    settings_repo::set(&state.db, RANGE_KEY, range)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, "cms:settings:*").await;
    read_public_settings(state).await
}

#[tracing::instrument(skip(state, req))]
pub async fn update_hero(state: &AppState, req: UpdateHeroReq) -> AppResult<SettingsResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let payload =
        serde_json::to_string(&req.hero).map_err(|error| AppError::Other(error.into()))?;
    settings_repo::set(&state.db, HERO_KEY, &payload)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, "cms:settings:*").await;
    read_public_settings(state).await
}

async fn read_public_settings(state: &AppState) -> AppResult<SettingsResp> {
    let theme = settings_repo::get(&state.db, THEME_KEY)
        .await
        .map_err(db_err)?
        .filter(|theme| validate_theme(theme).is_ok())
        .unwrap_or_else(|| DEFAULT_THEME.to_owned());
    let range = settings_repo::get(&state.db, RANGE_KEY)
        .await
        .map_err(db_err)?
        .filter(|range| validate_range(range).is_ok())
        .unwrap_or_else(|| DEFAULT_RANGE.to_owned());
    let hero = settings_repo::get(&state.db, HERO_KEY)
        .await
        .map_err(db_err)?
        .and_then(|raw| serde_json::from_str::<HeroCopy>(&raw).ok())
        .unwrap_or_else(HeroCopy::default_copy);

    Ok(SettingsResp { theme, range, hero })
}

fn db_err(error: DbErr) -> AppError {
    AppError::Other(error.into())
}
