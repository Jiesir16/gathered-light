//! Post（文章/页面）Controller。
//!
//! 租户上下文：当前用默认租户/站点(1,1) + JWT 中的用户 id 构造 `ReqCtx`。
//! IAM 中间件落地后，改为从请求扩展里的 `Principal`/`TenantContext` 读取。

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::{
    bootstrap::AppState,
    dto::post_dto::{ArticleDto, ArticleListQuery, ArticleListResp, ArticleReq, RevisionSummary},
    error::AppResult,
    infra::jwt::Claims,
    services::post_service::{self, ReqCtx},
};

const DEFAULT_TENANT_ID: i64 = 1;
const DEFAULT_SITE_ID: i64 = 1;

fn admin_ctx(claims: &Claims) -> ReqCtx {
    ReqCtx {
        tenant_id: DEFAULT_TENANT_ID,
        site_id: DEFAULT_SITE_ID,
        user_id: claims.sub.parse::<i64>().ok(),
    }
}

fn public_ctx() -> ReqCtx {
    ReqCtx {
        tenant_id: DEFAULT_TENANT_ID,
        site_id: DEFAULT_SITE_ID,
        user_id: None,
    }
}

// ── 前台（公开） ────────────────────────────────────────────────
pub async fn list_public(
    State(state): State<AppState>,
    Query(query): Query<ArticleListQuery>,
) -> AppResult<Json<ArticleListResp>> {
    Ok(Json(
        post_service::list_public(&state, public_ctx(), query).await?,
    ))
}

pub async fn get_public(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<ArticleDto>> {
    Ok(Json(
        post_service::get_public(&state, public_ctx(), "post", &slug).await?,
    ))
}

// ── 管理端（需 jwt_guard） ──────────────────────────────────────
pub async fn list_admin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ArticleListQuery>,
) -> AppResult<Json<ArticleListResp>> {
    Ok(Json(
        post_service::list_admin(&state, admin_ctx(&claims), query).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ArticleReq>,
) -> AppResult<(StatusCode, Json<ArticleDto>)> {
    let dto = post_service::create(&state, admin_ctx(&claims), req).await?;
    Ok((StatusCode::CREATED, Json(dto)))
}

pub async fn get_admin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i64>,
) -> AppResult<Json<ArticleDto>> {
    Ok(Json(
        post_service::get_admin(&state, admin_ctx(&claims), id).await?,
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i64>,
    Json(req): Json<ArticleReq>,
) -> AppResult<Json<ArticleDto>> {
    Ok(Json(
        post_service::update(&state, admin_ctx(&claims), id, req).await?,
    ))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    post_service::delete(&state, admin_ctx(&claims), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_revisions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<RevisionSummary>>> {
    Ok(Json(
        post_service::list_revisions(&state, admin_ctx(&claims), id).await?,
    ))
}
