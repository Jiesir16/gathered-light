use cms_entity::tags;
use sea_orm::DbErr;
use serde_json::Value;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::{
        photo_dto::I18nText,
        tag_dto::{NewTagReq, TagDto, TagListQuery, TagListResp, UpdateTagReq},
    },
    error::{AppError, AppResult},
    infra::cache,
    repositories::tag_repo,
};

#[tracing::instrument(skip(state))]
pub async fn list_admin_tags(state: &AppState, query: TagListQuery) -> AppResult<TagListResp> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 200);
    let items = tag_repo::list(&state.db, page, page_size)
        .await
        .map_err(db_err)?
        .iter()
        .map(to_dto)
        .collect();
    let total = tag_repo::count(&state.db).await.map_err(db_err)?;
    Ok(TagListResp {
        items,
        total,
        page,
        page_size,
    })
}

#[tracing::instrument(skip(state))]
pub async fn list_public_tags(state: &AppState) -> AppResult<Vec<TagDto>> {
    const KEY: &str = "cms:taxonomy:tags";
    if let Some(tags) = cache::get(&state.redis, KEY).await {
        tracing::debug!(key = KEY, "cache hit");
        return Ok(tags);
    }
    tracing::debug!(key = KEY, "cache miss");

    let tags = tag_repo::list_all(&state.db)
        .await
        .map_err(db_err)?
        .iter()
        .map(to_dto)
        .collect();
    cache::set(&state.redis, KEY, &tags, 600).await;
    Ok(tags)
}

#[tracing::instrument(skip(state, req))]
pub async fn create_tag(state: &AppState, req: NewTagReq) -> AppResult<TagDto> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    validate_slug(&req.slug)?;
    if tag_repo::find_by_slug(&state.db, &req.slug)
        .await
        .map_err(db_err)?
        .is_some()
    {
        return Err(AppError::Conflict("tag slug already exists"));
    }

    let tag = tag_repo::insert(&state.db, req.slug, req.name.zh, req.name.en)
        .await
        .map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:tags").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(to_dto(&tag))
}

#[tracing::instrument(skip(state, req))]
pub async fn update_tag(state: &AppState, id: i64, req: UpdateTagReq) -> AppResult<TagDto> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    if let Some(slug) = req.slug.as_deref() {
        validate_slug(slug)?;
        if let Some(existing) = tag_repo::find_by_slug(&state.db, slug)
            .await
            .map_err(db_err)?
        {
            if existing.id != id {
                return Err(AppError::Conflict("tag slug already exists"));
            }
        }
    }

    let (name_zh, name_en) = req
        .name
        .map(|name| (Some(name.zh), Some(name.en)))
        .unwrap_or((None, None));
    let tag = tag_repo::update(&state.db, id, req.slug, name_zh, name_en)
        .await
        .map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:tags").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(to_dto(&tag))
}

#[tracing::instrument(skip(state))]
pub async fn delete_tag(state: &AppState, id: i64) -> AppResult<()> {
    tag_repo::delete(&state.db, id).await.map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:tags").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(())
}

fn to_dto(tag: &tags::Model) -> TagDto {
    TagDto {
        id: tag.id,
        slug: tag.slug.clone(),
        name: I18nText {
            zh: i18n_value(&tag.name_i18n, "zh"),
            en: i18n_value(&tag.name_i18n, "en"),
        },
    }
}

fn validate_slug(slug: &str) -> AppResult<()> {
    let len_ok = (1..=40).contains(&slug.len());
    let chars_ok = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if len_ok && chars_ok {
        Ok(())
    } else {
        Err(AppError::Validation(
            "slug must match [a-z0-9-]{1,40}".to_owned(),
        ))
    }
}

fn i18n_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn db_err(error: DbErr) -> AppError {
    match error {
        DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}
