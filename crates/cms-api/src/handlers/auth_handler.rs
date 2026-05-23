use axum::{Extension, Json, extract::State};

use crate::{
    bootstrap::AppState,
    dto::auth_dto::{LoginReq, MeResp, RefreshReq, TokenPairResp},
    error::AppResult,
    infra::jwt::Claims,
    services::auth_service,
};

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> AppResult<Json<TokenPairResp>> {
    Ok(Json(auth_service::login(&state, req).await?))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> AppResult<Json<TokenPairResp>> {
    Ok(Json(auth_service::refresh(&state, req).await?))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<serde_json::Value>> {
    auth_service::logout(&state, &claims).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn logout_all(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<serde_json::Value>> {
    auth_service::logout_all(&state, &claims).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<MeResp>> {
    Ok(Json(auth_service::me(&state, &claims).await?))
}
