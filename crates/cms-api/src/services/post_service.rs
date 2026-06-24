//! Post（文章/页面）服务层：校验 + slug + TipTap 清洗 + 版本 + 缓存编排。
//!
//! 多租户上下文 `ReqCtx` 暂由 handler 用默认租户/站点(1,1)+当前用户构造；
//! 待 IAM 中间件落地后改为从 Principal 注入（见 docs/USER_CENTER_AND_IAM_DESIGN.md §5.4）。

use std::str::FromStr;

use cms_domain::{PostStatus, PostType, Privacy};
use sea_orm::DbErr;
use uuid::Uuid;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::post_dto::{
        ArticleDto, ArticleListItem, ArticleListQuery, ArticleListResp, ArticleReq,
        ArticleTranslationDto, ArticleTranslationSummary, RevisionSummary,
    },
    error::{AppError, AppResult},
    infra::cache,
    repositories::post_repo::{self, NewPost, NewTranslation, PostDetail},
    services::content_render,
};

#[derive(Clone, Copy, Debug)]
pub struct ReqCtx {
    pub tenant_id: i64,
    pub site_id: i64,
    pub user_id: Option<i64>,
}

const LIST_PREFIX: &str = "cms:post:list:*";

pub async fn create(state: &AppState, ctx: ReqCtx, req: ArticleReq) -> AppResult<ArticleDto> {
    let input = build_input(state, ctx, &req, None, None).await?;
    let id = post_repo::create(&state.db, ctx.tenant_id, input)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, LIST_PREFIX).await;
    find_dto(state, ctx, id).await
}

pub async fn update(
    state: &AppState,
    ctx: ReqCtx,
    id: i64,
    req: ArticleReq,
) -> AppResult<ArticleDto> {
    let current = post_repo::find_by_id(&state.db, ctx.tenant_id, id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    let input = build_input(state, ctx, &req, Some(id), Some(&current.post.slug)).await?;
    post_repo::update(&state.db, ctx.tenant_id, id, input)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, LIST_PREFIX).await;
    cache::del_one(&state.redis, &detail_key(&current.post.slug)).await;
    find_dto(state, ctx, id).await
}

pub async fn get_admin(state: &AppState, ctx: ReqCtx, id: i64) -> AppResult<ArticleDto> {
    find_dto(state, ctx, id).await
}

pub async fn get_public(
    state: &AppState,
    ctx: ReqCtx,
    post_type: &str,
    slug: &str,
) -> AppResult<ArticleDto> {
    let detail = post_repo::find_by_slug(&state.db, ctx.tenant_id, ctx.site_id, post_type, slug)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    // 前台只暴露已发布且公开的内容
    if detail.post.status != "published" || detail.post.visibility != "public" {
        return Err(AppError::NotFound);
    }
    Ok(to_dto(detail))
}

pub async fn list_admin(
    state: &AppState,
    ctx: ReqCtx,
    query: ArticleListQuery,
) -> AppResult<ArticleListResp> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let (rows, total) = post_repo::list_admin(
        &state.db,
        ctx.tenant_id,
        ctx.site_id,
        &query.post_type,
        query.status.as_deref(),
        query.q.as_deref(),
        page,
        page_size,
    )
    .await
    .map_err(db_err)?;
    Ok(ArticleListResp {
        items: rows.into_iter().map(to_list_item).collect(),
        total,
        page,
        page_size,
    })
}

pub async fn list_public(
    state: &AppState,
    ctx: ReqCtx,
    query: ArticleListQuery,
) -> AppResult<ArticleListResp> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let cache_key = format!(
        "cms:post:list:t={}:s={}:type={}:p={}:ps={}",
        ctx.tenant_id, ctx.site_id, query.post_type, page, page_size
    );
    if let Some(cached) = cache::get::<ArticleListResp>(&state.redis, &cache_key).await {
        return Ok(cached);
    }
    let (rows, total) =
        post_repo::list_published(&state.db, ctx.tenant_id, ctx.site_id, &query.post_type, page, page_size)
            .await
            .map_err(db_err)?;
    let resp = ArticleListResp {
        items: rows.into_iter().map(to_list_item).collect(),
        total,
        page,
        page_size,
    };
    cache::set(&state.redis, &cache_key, &resp, 60).await;
    Ok(resp)
}

pub async fn delete(state: &AppState, ctx: ReqCtx, id: i64) -> AppResult<()> {
    let current = post_repo::find_by_id(&state.db, ctx.tenant_id, id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    post_repo::delete(&state.db, ctx.tenant_id, id)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, LIST_PREFIX).await;
    cache::del_one(&state.redis, &detail_key(&current.post.slug)).await;
    Ok(())
}

pub async fn list_revisions(
    state: &AppState,
    ctx: ReqCtx,
    id: i64,
) -> AppResult<Vec<RevisionSummary>> {
    let rows = post_repo::list_revisions(&state.db, ctx.tenant_id, id)
        .await
        .map_err(db_err)?;
    Ok(rows
        .into_iter()
        .map(|r| RevisionSummary {
            id: r.id,
            locale: r.locale,
            title: r.title,
            author_id: r.author_id,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect())
}

async fn find_dto(state: &AppState, ctx: ReqCtx, id: i64) -> AppResult<ArticleDto> {
    let detail = post_repo::find_by_id(&state.db, ctx.tenant_id, id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    Ok(to_dto(detail))
}

async fn build_input(
    state: &AppState,
    ctx: ReqCtx,
    req: &ArticleReq,
    exclude_id: Option<i64>,
    current_slug: Option<&str>,
) -> AppResult<NewPost> {
    validate_req(req)?;

    // slug：显式给优先，其次沿用旧 slug（编辑），再次按首条标题生成，兜底 uuid
    let raw_slug = match (
        req.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        current_slug,
    ) {
        (Some(explicit), _) => slug::slugify(explicit),
        (None, Some(existing)) => existing.to_owned(),
        (None, None) => slug::slugify(&req.translations[0].title),
    };
    let slug = if raw_slug.is_empty() {
        format!("{}-{}", req.post_type, Uuid::new_v4())
    } else {
        raw_slug
    };
    validate_slug(&slug)?;
    if post_repo::slug_exists(
        &state.db,
        ctx.tenant_id,
        ctx.site_id,
        &req.post_type,
        &slug,
        exclude_id,
    )
    .await
    .map_err(db_err)?
    {
        return Err(AppError::Conflict("slug already exists"));
    }

    let mut translations = Vec::with_capacity(req.translations.len());
    for tr in &req.translations {
        let rendered = match &tr.body_json {
            Some(json) => Some(content_render::render_body(json)?),
            None => None,
        };
        let (body_html, word_count, auto_excerpt) = match rendered {
            Some(r) => (Some(r.html), r.word_count, Some(r.excerpt)),
            None => (None, 0, None),
        };
        translations.push(NewTranslation {
            locale: tr.locale.clone(),
            title: tr.title.clone(),
            excerpt: tr
                .excerpt
                .clone()
                .filter(|s| !s.trim().is_empty())
                .or(auto_excerpt),
            body_json: tr.body_json.clone(),
            body_html,
            word_count,
        });
    }

    let published_at = if req.status == PostStatus::Published.as_str() {
        Some(chrono::Utc::now().fixed_offset())
    } else {
        None
    };

    Ok(NewPost {
        site_id: ctx.site_id,
        post_type: req.post_type.clone(),
        slug,
        status: req.status.clone(),
        visibility: req.visibility.clone(),
        author_id: ctx.user_id,
        menu_order: req.menu_order,
        published_at,
        translations,
        term_taxonomy_ids: req.term_taxonomy_ids.clone(),
    })
}

fn validate_req(req: &ArticleReq) -> AppResult<()> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let post_type =
        PostType::from_str(&req.post_type).map_err(|_| AppError::Validation("invalid post_type".into()))?;
    if matches!(post_type, PostType::Photo) {
        return Err(AppError::Validation(
            "photo content uses the /photos endpoints".into(),
        ));
    }
    PostStatus::from_str(&req.status).map_err(|_| AppError::Validation("invalid status".into()))?;
    Privacy::from_str(&req.visibility)
        .map_err(|_| AppError::Validation("invalid visibility".into()))?;
    Ok(())
}

fn validate_slug(slug: &str) -> AppResult<()> {
    let len_ok = (1..=120).contains(&slug.len());
    let chars_ok = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if len_ok && chars_ok {
        Ok(())
    } else {
        Err(AppError::Validation(
            "slug must match [a-z0-9-]{1,120}".into(),
        ))
    }
}

fn to_dto(detail: PostDetail) -> ArticleDto {
    ArticleDto {
        id: detail.post.id,
        slug: detail.post.slug,
        post_type: detail.post.post_type,
        status: detail.post.status,
        visibility: detail.post.visibility,
        author_id: detail.post.author_id,
        menu_order: detail.post.menu_order,
        term_taxonomy_ids: detail.term_taxonomy_ids,
        published_at: detail.post.published_at.map(|t| t.to_rfc3339()),
        created_at: detail.post.created_at.to_rfc3339(),
        updated_at: detail.post.updated_at.to_rfc3339(),
        translations: detail
            .translations
            .into_iter()
            .map(|t| ArticleTranslationDto {
                locale: t.locale,
                title: t.title,
                excerpt: t.excerpt,
                body_html: t.body_html,
                word_count: t.word_count,
            })
            .collect(),
    }
}

fn to_list_item(detail: PostDetail) -> ArticleListItem {
    ArticleListItem {
        id: detail.post.id,
        slug: detail.post.slug,
        post_type: detail.post.post_type,
        status: detail.post.status,
        visibility: detail.post.visibility,
        author_id: detail.post.author_id,
        published_at: detail.post.published_at.map(|t| t.to_rfc3339()),
        updated_at: detail.post.updated_at.to_rfc3339(),
        translations: detail
            .translations
            .into_iter()
            .map(|t| ArticleTranslationSummary {
                locale: t.locale,
                title: t.title,
                excerpt: t.excerpt,
            })
            .collect(),
    }
}

fn detail_key(slug: &str) -> String {
    format!("cms:post:detail:{slug}")
}

fn db_err(error: DbErr) -> AppError {
    match error {
        DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}
