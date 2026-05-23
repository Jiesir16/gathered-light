use cms_entity::categories;
use sea_orm::DbErr;
use serde_json::Value;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::{
        category_dto::{CategoryAdminDto, NewCategoryReq, UpdateCategoryReq},
        photo_dto::{CategoryDto, I18nText},
    },
    error::{AppError, AppResult},
    infra::cache,
    repositories::category_repo::{self, CategoryWithCount},
};

#[tracing::instrument(skip(state))]
pub async fn list_public(state: &AppState) -> AppResult<Vec<CategoryDto>> {
    const KEY: &str = "cms:taxonomy:categories";
    if let Some(categories) = cache::get(&state.redis, KEY).await {
        tracing::debug!(key = KEY, "cache hit");
        return Ok(categories);
    }
    tracing::debug!(key = KEY, "cache miss");

    let categories = category_repo::list(&state.db)
        .await
        .map_err(db_err)?
        .iter()
        .map(to_public_dto)
        .collect();
    cache::set(&state.redis, KEY, &categories, 600).await;
    Ok(categories)
}

#[tracing::instrument(skip(state))]
pub async fn list_admin_categories(state: &AppState) -> AppResult<Vec<CategoryAdminDto>> {
    Ok(category_repo::list(&state.db)
        .await
        .map_err(db_err)?
        .iter()
        .map(to_admin_dto)
        .collect())
}

#[tracing::instrument(skip(state, req))]
pub async fn create_category(state: &AppState, req: NewCategoryReq) -> AppResult<CategoryAdminDto> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    validate_slug(&req.slug)?;
    if category_repo::find_by_slug(&state.db, &req.slug)
        .await
        .map_err(db_err)?
        .is_some()
    {
        return Err(AppError::Conflict("slug already exists"));
    }

    let category = category_repo::insert(
        &state.db,
        req.slug,
        req.name.zh,
        req.name.en,
        req.sort_order,
    )
    .await
    .map_err(db_err)?;
    let photo_count = category_repo::count_photos_in(&state.db, category.id)
        .await
        .map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:categories").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(admin_dto_from_model(&category, photo_count))
}

#[tracing::instrument(skip(state, req))]
pub async fn update_category(
    state: &AppState,
    id: i64,
    req: UpdateCategoryReq,
) -> AppResult<CategoryAdminDto> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    if category_repo::find_by_id(&state.db, id)
        .await
        .map_err(db_err)?
        .is_none()
    {
        return Err(AppError::NotFound);
    }
    if let Some(slug) = req.slug.as_deref() {
        validate_slug(slug)?;
        if let Some(existing) = category_repo::find_by_slug(&state.db, slug)
            .await
            .map_err(db_err)?
        {
            if existing.id != id {
                return Err(AppError::Conflict("slug already exists"));
            }
        }
    }

    let (name_zh, name_en) = match req.name {
        Some(name) => (Some(name.zh), Some(name.en)),
        None => (None, None),
    };
    let category = category_repo::update(&state.db, id, req.slug, name_zh, name_en, req.sort_order)
        .await
        .map_err(db_err)?;
    let photo_count = category_repo::count_photos_in(&state.db, category.id)
        .await
        .map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:categories").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(admin_dto_from_model(&category, photo_count))
}

#[tracing::instrument(skip(state))]
pub async fn delete_category(state: &AppState, id: i64) -> AppResult<()> {
    let photo_count = category_repo::count_photos_in(&state.db, id)
        .await
        .map_err(db_err)?;
    if photo_count > 0 {
        return Err(AppError::Conflict(
            "category still has photos; reassign or delete them first",
        ));
    }
    category_repo::delete(&state.db, id).await.map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:categories").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(())
}

fn to_public_dto(category: &CategoryWithCount) -> CategoryDto {
    CategoryDto {
        key: category.slug.clone(),
        zh: i18n_value(&category.name_i18n, "zh"),
        en: i18n_value(&category.name_i18n, "en"),
    }
}

fn to_admin_dto(category: &CategoryWithCount) -> CategoryAdminDto {
    CategoryAdminDto {
        id: category.id,
        slug: category.slug.clone(),
        name: I18nText {
            zh: i18n_value(&category.name_i18n, "zh"),
            en: i18n_value(&category.name_i18n, "en"),
        },
        sort_order: category.sort_order,
        photo_count: category.photo_count,
    }
}

fn admin_dto_from_model(category: &categories::Model, photo_count: i64) -> CategoryAdminDto {
    CategoryAdminDto {
        id: category.id,
        slug: category.slug.clone(),
        name: I18nText {
            zh: i18n_value(&category.name_i18n, "zh"),
            en: i18n_value(&category.name_i18n, "en"),
        },
        sort_order: category.sort_order,
        photo_count,
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
    match value.get(key).and_then(Value::as_str) {
        Some(text) => text.to_owned(),
        None => String::new(),
    }
}

fn db_err(error: DbErr) -> AppError {
    match error {
        DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}
