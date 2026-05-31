use std::{collections::HashSet, str::FromStr};

use cms_domain::Privacy;
use cms_entity::{categories, media_assets, media_variants, tags};
use fred::interfaces::KeysInterface;
use sea_orm::{DbErr, EntityTrait, QueryOrder, TransactionTrait};
use serde_json::Value;
use uuid::Uuid;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::media_dto::RepairResp,
    dto::photo_dto::{
        BulkDeleteReq, BulkPrivacyReq, BulkResp, BulkTagMode, BulkTagsReq, CategoryDto, I18nText,
        PhotoDto, PhotoListQuery, PhotoListResp, PhotoQuery, PhotoReq, PhotoVariants, PrivacyReq,
        TagSummary, UnlockReq, UnlockResp,
    },
    error::{AppError, AppResult},
    infra::cache,
    repositories::{
        media_repo,
        photo_repo::{self, NewPhoto, PhotoFull},
        tag_repo,
    },
    services::media_service,
};

#[derive(Debug)]
pub struct SeedReport {
    pub total: usize,
    pub public: usize,
    pub locked: usize,
    pub private: usize,
}

pub async fn list_public(state: &AppState, query: PhotoQuery) -> AppResult<Vec<PhotoDto>> {
    let category = query
        .category
        .as_deref()
        .filter(|category| !category.is_empty() && *category != "all")
        .unwrap_or("all");
    let cache_key = format!("cms:photo:list:cat={category}");
    if let Some(photos) = cache::get(&state.redis, &cache_key).await {
        tracing::debug!(key = %cache_key, "cache hit");
        return Ok(photos);
    }
    tracing::debug!(key = %cache_key, "cache miss");

    let rows = photo_repo::list_public(&state.db, category_filter(&query))
        .await
        .map_err(db_err)?;
    let photos = assemble_list(state, rows, DtoScope::Public).await?;
    cache::set(&state.redis, &cache_key, &photos, 60).await;
    Ok(photos)
}

pub async fn list_admin(state: &AppState, query: PhotoListQuery) -> AppResult<PhotoListResp> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let (rows, total) = photo_repo::list_admin_paginated(
        &state.db,
        query.q.as_deref(),
        query.category.as_deref(),
        query.privacy.as_deref(),
        page,
        page_size,
    )
    .await
    .map_err(db_err)?;
    let items = assemble_list(state, rows, DtoScope::Admin).await?;
    Ok(PhotoListResp {
        items,
        total,
        page,
        page_size,
    })
}

async fn list_admin_items(state: &AppState, query: PhotoListQuery) -> AppResult<Vec<PhotoDto>> {
    Ok(list_admin(state, query).await?.items)
}

pub async fn list_admin_all(state: &AppState, query: PhotoQuery) -> AppResult<Vec<PhotoDto>> {
    let rows = photo_repo::list_admin(&state.db, category_filter(&query))
        .await
        .map_err(db_err)?;
    assemble_list(state, rows, DtoScope::Admin).await
}

pub async fn categories(state: &AppState) -> AppResult<Vec<CategoryDto>> {
    let rows = categories::Entity::find()
        .order_by_asc(categories::Column::SortOrder)
        .all(&state.db)
        .await
        .map_err(db_err)?;
    Ok(rows
        .into_iter()
        .map(|category| {
            let zh = category
                .name_i18n
                .get("zh")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let en = category
                .name_i18n
                .get("en")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            CategoryDto {
                key: category.slug,
                zh,
                en,
            }
        })
        .collect())
}

pub async fn create(state: &AppState, req: PhotoReq) -> AppResult<PhotoDto> {
    validate(&req)?;
    let privacy = req.privacy;
    let slug = create_slug(state, &req).await?;
    let src_url = normalize_stored_src(&req.src);
    let input = new_photo_from_req(slug, req, None, src_url);
    let id = photo_repo::create(&state.db, input).await.map_err(db_err)?;
    // worker 常在 photo 创建前就处理完 variants（那时查不到照片→默认按 private 传），
    // 这里按本张照片的真实 privacy 回填一次 OSS ACL，否则 public 照片的 variant 仍是私有 → 前端 403。
    if let Err(error) = sync_variants_acl_for_photo(state, id, privacy).await {
        tracing::warn!(error = ?error, photo_id = id, "sync variants ACL after create failed");
    }
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    find_dto(state, id, DtoScope::Admin).await
}

pub async fn update(state: &AppState, id: i64, req: PhotoReq) -> AppResult<PhotoDto> {
    validate(&req)?;
    let current = photo_repo::find_by_id(&state.db, id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    let variants_map = media_repo::find_variants_for_assets(&state.db, &[current.asset.id])
        .await
        .map_err(db_err)?;
    let variants = variants_map
        .get(&current.asset.id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let slug = update_slug(state, id, &req, &current.photo.slug).await?;
    let new_privacy = req.privacy;
    let src_url = normalize_update_src(&req.src, &current.asset, variants);
    let input = new_photo_from_req(slug, req, current.photo.passcode, src_url);
    photo_repo::update(&state.db, id, input)
        .await
        .map_err(db_err)?;
    // privacy 可能在 update 里改了（前端可以编辑），同步 OSS ACL；也兼做存量数据 ACL 回填
    if let Err(error) = sync_variants_acl_for_photo(state, id, new_privacy).await {
        tracing::warn!(error = ?error, photo_id = id, "sync variants ACL after update failed");
    }
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    find_dto(state, id, DtoScope::Admin).await
}

pub async fn update_privacy(state: &AppState, id: i64, req: PrivacyReq) -> AppResult<PhotoDto> {
    let passcode = match req.privacy {
        Privacy::Locked => Some("1234".to_owned()),
        Privacy::Public | Privacy::Private => None,
    };
    photo_repo::update_privacy(&state.db, id, req.privacy, passcode)
        .await
        .map_err(db_err)?;
    // 同步把这张照片所有 variants 在 OSS 的 ACL 翻一下
    if let Err(error) = sync_variants_acl_for_photo(state, id, req.privacy).await {
        tracing::warn!(error = ?error, photo_id = id, "sync variants ACL after update_privacy failed");
    }
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    find_dto(state, id, DtoScope::Admin).await
}

#[tracing::instrument(skip(state))]
pub async fn recover_urls(state: &AppState, id: i64) -> AppResult<PhotoDto> {
    let current = photo_repo::find_by_id(&state.db, id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    let variants_map = media_repo::find_variants_for_assets(&state.db, &[current.asset.id])
        .await
        .map_err(db_err)?;
    let variant_repairs: Vec<(i64, String)> = variants_map
        .get(&current.asset.id)
        .map(|variants| {
            variants
                .iter()
                .filter_map(|variant| {
                    object_key_from_stored_url(&variant.storage_key, &["variants/"])
                        .map(|key| (variant.id, key))
                })
                .collect()
        })
        .unwrap_or_default();
    let recovered_asset_key =
        object_key_from_stored_url(&current.asset.storage_key, &["uploads/", "variants/"]);

    let effective_asset_id = photo_repo::recover_expired_media_urls(
        &state.db,
        id,
        current.asset.id,
        recovered_asset_key.as_deref(),
        &variant_repairs,
    )
    .await
    .map_err(db_err)?;

    if effective_asset_id != current.asset.id {
        let variants_map = media_repo::find_variants_for_assets(&state.db, &[effective_asset_id])
            .await
            .map_err(db_err)?;
        if let Some(variants) = variants_map.get(&effective_asset_id) {
            let variant_repairs: Vec<(i64, String)> = variants
                .iter()
                .filter_map(|variant| {
                    object_key_from_stored_url(&variant.storage_key, &["variants/"])
                        .map(|key| (variant.id, key))
                })
                .collect();
            if !variant_repairs.is_empty() {
                photo_repo::recover_expired_media_urls(
                    &state.db,
                    id,
                    effective_asset_id,
                    None,
                    &variant_repairs,
                )
                .await
                .map_err(db_err)?;
            }
        }
    }

    let privacy = Privacy::from_str(&current.photo.privacy).unwrap_or(Privacy::Public);
    if let Err(error) = sync_variants_acl_for_photo(state, id, privacy).await {
        tracing::warn!(error = ?error, photo_id = id, "sync variants ACL after URL recovery failed");
    }
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    find_dto(state, id, DtoScope::Admin).await
}

pub async fn delete(state: &AppState, id: i64) -> AppResult<()> {
    photo_repo::delete(&state.db, id).await.map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(())
}

pub async fn bulk_delete(state: &AppState, req: BulkDeleteReq) -> AppResult<BulkResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let existing = photo_repo::find_existing_ids(&state.db, &req.ids)
        .await
        .map_err(db_err)?;
    let skipped = skipped_ids(&req.ids, &existing);
    let affected = photo_repo::delete_many(&state.db, &existing)
        .await
        .map_err(db_err)?;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(BulkResp { affected, skipped })
}

pub async fn bulk_update_privacy(state: &AppState, req: BulkPrivacyReq) -> AppResult<BulkResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let existing = photo_repo::find_existing_ids(&state.db, &req.ids)
        .await
        .map_err(db_err)?;
    let skipped = skipped_ids(&req.ids, &existing);
    let passcode = if matches!(req.privacy, Privacy::Locked) {
        Some("1234".to_owned())
    } else {
        None
    };
    let affected = photo_repo::update_privacy_many(&state.db, &existing, req.privacy, passcode)
        .await
        .map_err(db_err)?;
    // 批量翻 ACL（容错：单条失败不打断整批）
    for photo_id in &existing {
        if let Err(error) = sync_variants_acl_for_photo(state, *photo_id, req.privacy).await {
            tracing::warn!(error = ?error, photo_id, "sync variants ACL in bulk failed");
        }
    }
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(BulkResp { affected, skipped })
}

pub async fn bulk_set_tags(state: &AppState, req: BulkTagsReq) -> AppResult<BulkResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let existing = photo_repo::find_existing_ids(&state.db, &req.ids)
        .await
        .map_err(db_err)?;
    let skipped = skipped_ids(&req.ids, &existing);
    let txn = state.db.begin().await.map_err(db_err)?;
    let replace = matches!(req.mode, BulkTagMode::Replace);
    tag_repo::assign_many(&txn, &existing, &req.tag_ids, replace)
        .await
        .map_err(db_err)?;
    txn.commit().await.map_err(db_err)?;
    cache::del_one(&state.redis, "cms:taxonomy:tags").await;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    Ok(BulkResp {
        affected: existing.len() as i64,
        skipped,
    })
}

pub async fn reset(state: &AppState) -> AppResult<Vec<PhotoDto>> {
    photo_repo::truncate_all(&state.db).await.map_err(db_err)?;
    seed_demo_photos(state).await?;
    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
    list_admin_items(
        state,
        PhotoListQuery {
            page: 1,
            page_size: 100,
            q: None,
            category: None,
            privacy: None,
        },
    )
    .await
}

pub async fn unlock(state: &AppState, id: i64, req: UnlockReq) -> AppResult<UnlockResp> {
    let rate_limit_key = format!("auth:photo:unlock:rl:{id}");
    let attempts: i64 = state
        .redis
        .incr(rate_limit_key.as_str())
        .await
        .map_err(redis_err)?;
    if attempts == 1 {
        let _: bool = state
            .redis
            .expire(rate_limit_key.as_str(), 600)
            .await
            .map_err(redis_err)?;
    }
    if attempts > 10 {
        return Err(AppError::TooManyRequests("unlock attempts"));
    }

    let photo = photo_repo::find_by_id(&state.db, id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::NotFound)?;
    if photo.photo.privacy == "private" {
        return Err(AppError::Forbidden);
    }
    if photo.photo.privacy == "public" {
        return Ok(UnlockResp { unlocked: true });
    }

    let stored = photo
        .photo
        .passcode
        .ok_or(AppError::Internal("locked photo missing passcode"))?;
    // 明文等长比较；passcode 是分享口令不是登录密码，可接受简单 string eq
    let unlocked = stored == req.passcode;
    Ok(UnlockResp { unlocked })
}

pub async fn seed_demo_photos(state: &AppState) -> AppResult<SeedReport> {
    let mut report = SeedReport {
        total: 0,
        public: 0,
        locked: 0,
        private: 0,
    };

    for (idx, photo) in DEMO_PHOTOS.iter().enumerate() {
        let passcode = if photo.privacy == Privacy::Locked {
            Some("1234".to_owned())
        } else {
            None
        };
        let input = NewPhoto {
            slug: format!("demo-{}", idx + 1),
            category_slug: photo.cat.to_owned(),
            src_url: photo.src.to_owned(),
            mime_type: "image/jpeg".to_owned(),
            privacy: photo.privacy,
            passcode,
            taken_at_label: photo.date.to_owned(),
            title_zh: photo.title_zh.to_owned(),
            title_en: photo.title_en.to_owned(),
            loc_zh: photo.loc_zh.to_owned(),
            loc_en: photo.loc_en.to_owned(),
            caption_zh: None,
            caption_en: None,
            alt_text_zh: None,
            alt_text_en: None,
            tag_ids: Vec::new(),
        };
        photo_repo::create(&state.db, input).await.map_err(db_err)?;
        report.total += 1;
        match photo.privacy {
            Privacy::Public => report.public += 1,
            Privacy::Locked => report.locked += 1,
            Privacy::Private => report.private += 1,
        }
    }

    Ok(report)
}

/// Build DTO 区分作用域：
/// - `DtoScope::Public`：passcode 字段不下发（管理隐私）
/// - `DtoScope::Admin`：包含 passcode 明文，便于在后台显示
#[derive(Clone, Copy)]
pub enum DtoScope {
    Public,
    Admin,
}

async fn build_dto(
    state: &AppState,
    full: PhotoFull,
    variants: &[media_variants::Model],
    tags: Vec<tags::Model>,
    scope: DtoScope,
) -> AppResult<PhotoDto> {
    let privacy = Privacy::from_str(&full.photo.privacy).unwrap_or(Privacy::Private);
    let urls = build_variants_urls(state, &full.asset, variants, privacy).await?;
    // src 字段保持兼容：取 medium，没有就 fallback thumb / original
    let src = urls
        .medium
        .clone()
        .or_else(|| urls.thumb.clone())
        .or_else(|| urls.original.clone())
        .unwrap_or_default();
    let passcode = match scope {
        DtoScope::Admin => full.photo.passcode.clone(),
        DtoScope::Public => None,
    };
    // 卡片显示的是 medium/webp，取它的像素宽高交给前端设 aspect-ratio；
    // 各 variant 同比例缩放，挑不到 medium 就退而求其次，保证比例正确即可。
    let dims = ["medium_900", "webp_900", "thumb_400", "full_1800"]
        .iter()
        .find_map(|name| variants.iter().find(|v| v.variant == *name))
        .map(|v| (v.width, v.height))
        .or_else(|| full.asset.width.zip(full.asset.height));
    Ok(PhotoDto {
        id: full.photo.id,
        slug: full.photo.slug.clone(),
        src,
        cat: full.category_slug,
        title: I18nText {
            zh: full.title_zh,
            en: full.title_en,
        },
        loc: I18nText {
            zh: full.loc_zh,
            en: full.loc_en,
        },
        caption: i18n_optional(full.caption_zh, full.caption_en),
        alt_text: i18n_optional(full.alt_text_zh, full.alt_text_en),
        date: full.photo.taken_at_label,
        privacy,
        width: dims.map(|(w, _)| w),
        height: dims.map(|(_, h)| h),
        tags: tags.into_iter().map(tag_summary).collect(),
        variants: urls,
        passcode,
    })
}

/// 把单张图所有 variants（含 original）按 privacy 转成 URL：
/// - public：永久 URL（依赖 OSS ACL=public-read，零 TTL 开销）
/// - locked/private：1h presigned
/// - original：用户原始上传，**始终 presigned**（即使是 public 照片，避免无意中把 10MB 原图设公开外网可下）；TTL 24h
async fn build_variants_urls(
    state: &AppState,
    asset: &media_assets::Model,
    variants: &[media_variants::Model],
    privacy: Privacy,
) -> AppResult<PhotoVariants> {
    let mut out = PhotoVariants::default();

    // demo seed 数据 src 直接是外链 URL；4 个 variant 字段拿不到，但 original 直接用 URL
    if asset.storage_key.starts_with("http://") || asset.storage_key.starts_with("https://") {
        out.original = Some(asset.storage_key.clone());
        out.medium = Some(asset.storage_key.clone());
        out.full = Some(asset.storage_key.clone());
        out.thumb = Some(asset.storage_key.clone());
        return Ok(out);
    }

    let find = |name: &str| {
        variants
            .iter()
            .find(|v| v.variant == name)
            .map(|v| v.storage_key.as_str())
    };

    if let Some(key) = find("thumb_400") {
        out.thumb = Some(variant_url(state, key, privacy).await?);
    }
    if let Some(key) = find("medium_900") {
        out.medium = Some(variant_url(state, key, privacy).await?);
    }
    if let Some(key) = find("full_1800") {
        out.full = Some(variant_url(state, key, privacy).await?);
    }
    if let Some(key) = find("webp_900") {
        out.webp = Some(variant_url(state, key, privacy).await?);
    }
    // original 一律 presigned 1d（不公开 ACL，省得 10MB 原图被随便下）
    out.original = Some(
        media_service::presign_get(&state.s3, &state.config.s3, &asset.storage_key, 86400).await?,
    );

    Ok(out)
}

async fn variant_url(state: &AppState, key: &str, privacy: Privacy) -> AppResult<String> {
    match privacy {
        Privacy::Public => Ok(media_service::permanent_url(state, key)),
        Privacy::Locked | Privacy::Private => {
            media_service::presign_get(&state.s3, &state.config.s3, key, 3600).await
        }
    }
}

async fn find_dto(state: &AppState, id: i64, scope: DtoScope) -> AppResult<PhotoDto> {
    let row = photo_repo::find_by_id(&state.db, id)
        .await
        .map_err(db_err)?;
    let row = row.ok_or(AppError::NotFound)?;
    let variants_map = media_repo::find_variants_for_assets(&state.db, &[row.asset.id])
        .await
        .map_err(db_err)?;
    let variants = variants_map
        .get(&row.asset.id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut tags_map = tag_repo::find_for_photos(&state.db, &[row.photo.id])
        .await
        .map_err(db_err)?;
    let tags = tags_map.remove(&row.photo.id).unwrap_or_default();
    build_dto(state, row, variants, tags, scope).await
}

async fn assemble_list(
    state: &AppState,
    rows: Vec<PhotoFull>,
    scope: DtoScope,
) -> AppResult<Vec<PhotoDto>> {
    let asset_ids: Vec<i64> = rows.iter().map(|row| row.asset.id).collect();
    let photo_ids: Vec<i64> = rows.iter().map(|row| row.photo.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids)
        .await
        .map_err(db_err)?;
    let tags_map = tag_repo::find_for_photos(&state.db, &photo_ids)
        .await
        .map_err(db_err)?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let variants = variants_map
            .get(&row.asset.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let tags = tags_map.get(&row.photo.id).cloned().unwrap_or_default();
        out.push(build_dto(state, row, variants, tags, scope).await?);
    }
    Ok(out)
}

/// 生成首图（瀑布流卡片）的访问 URL。
/// - 老的 demo 外链直通
/// - **public 照片**：拼永久 URL（依赖 OSS 对象 ACL=public-read），永不过期
/// - **locked / private 照片**：返回 1h 有效的 presigned URL（短 TTL 是安全特性）
pub async fn cover_url_for(
    state: &AppState,
    asset: &media_assets::Model,
    variants: &[media_variants::Model],
    privacy: Privacy,
) -> AppResult<String> {
    if asset.storage_key.starts_with("http://") || asset.storage_key.starts_with("https://") {
        return Ok(asset.storage_key.clone());
    }

    let key = variants
        .iter()
        .find(|variant| variant.variant == "medium_900")
        .or_else(|| {
            variants
                .iter()
                .find(|variant| variant.variant == "thumb_400")
        })
        .map(|variant| variant.storage_key.as_str())
        .unwrap_or(asset.storage_key.as_str());

    match privacy {
        Privacy::Public => Ok(media_service::permanent_url(state, key)),
        Privacy::Locked | Privacy::Private => {
            media_service::presign_get(&state.s3, &state.config.s3, key, 3600).await
        }
    }
}

/// 把 photo 的所有 variants 在 OSS 的 ACL 调整为符合当前 privacy。
/// - public → public-read（永久 URL 才能用）
/// - locked / private → private（被 presign 短 TTL 保护）
/// 单个 variant 失败不抛错，只记日志：S3 偶发抖动不该让 DB 写好的事回滚。
async fn sync_variants_acl_for_photo(
    state: &AppState,
    photo_id: i64,
    privacy: Privacy,
) -> AppResult<()> {
    let row = photo_repo::find_by_id(&state.db, photo_id)
        .await
        .map_err(db_err)?;
    let Some(row) = row else { return Ok(()) };
    let bucket = &state.config.s3.bucket;
    if row.asset.storage_key.starts_with("variants/") {
        let result = match privacy {
            Privacy::Public => {
                media_service::set_object_public(&state.s3, bucket, &row.asset.storage_key).await
            }
            Privacy::Locked | Privacy::Private => {
                media_service::set_object_private(&state.s3, bucket, &row.asset.storage_key).await
            }
        };
        if let Err(error) = result {
            tracing::warn!(
                error = ?error,
                photo_id,
                key = %row.asset.storage_key,
                "set recovered primary variant ACL failed"
            );
        }
    }
    let variants_map = media_repo::find_variants_for_assets(&state.db, &[row.asset.id])
        .await
        .map_err(db_err)?;
    let Some(variants) = variants_map.get(&row.asset.id) else {
        return Ok(()); // worker 还没跑完，新生成的会自带正确 ACL
    };
    for variant in variants {
        let result = match privacy {
            Privacy::Public => {
                media_service::set_object_public(&state.s3, bucket, &variant.storage_key).await
            }
            Privacy::Locked | Privacy::Private => {
                media_service::set_object_private(&state.s3, bucket, &variant.storage_key).await
            }
        };
        if let Err(error) = result {
            tracing::warn!(
                error = ?error,
                photo_id,
                variant = %variant.variant,
                key = %variant.storage_key,
                "set variant ACL failed"
            );
        }
    }
    Ok(())
}

/// 修复历史「桶名双写」数据（admin「修复历史数据」按钮触发）。
///
/// 三步：①COS 把旧对象 `{bucket}/...` 服务端复制成干净 key；②DB 去掉 storage_key 的桶名前缀；
/// ③按各 photo 的 privacy 重刷 variants ACL（COPY 出来的新对象默认 private）。复制不删旧对象、
/// 留作备份。要求 `APP_S3__ENDPOINT` 已是地域级（不带桶名），否则先返回错误提示、不动数据。
pub async fn repair_legacy_keys(state: &AppState) -> AppResult<RepairResp> {
    let bucket = state.config.s3.bucket.as_str();

    // 守卫：endpoint 还带桶名（host 以「桶名.」开头）就别跑，否则 list/copy 会再叠一层桶名搬错。
    let host = state.config.s3.endpoint.split("://").nth(1).unwrap_or("");
    if host.starts_with(&format!("{bucket}.")) {
        return Err(AppError::Validation(
            "S3 endpoint 仍带桶名，请先把 APP_S3__ENDPOINT 改成地域级（cos.<region>.myqcloud.com）并重启服务，再修复历史数据。"
                .to_owned(),
        ));
    }

    // ① COS：旧对象 `{bucket}/...` 服务端复制成干净 key（复制、不删，留作备份）
    let objects_copied = media_service::copy_legacy_objects(&state.s3, bucket).await?;
    // ② DB：去掉 storage_key 的桶名前缀
    let (assets_fixed, variants_fixed) = media_repo::strip_bucket_prefix(&state.db, bucket)
        .await
        .map_err(db_err)?;
    // ③ ACL：COPY 出来的新对象默认 private，按各 photo 的 privacy 重刷一遍
    let photos = photo_repo::list_admin(&state.db, None).await.map_err(db_err)?;
    let photos_resynced = photos.len();
    for full in &photos {
        let privacy = Privacy::from_str(&full.photo.privacy).unwrap_or(Privacy::Private);
        if let Err(error) = sync_variants_acl_for_photo(state, full.photo.id, privacy).await {
            tracing::warn!(error = ?error, photo_id = full.photo.id, "repair: resync ACL failed");
        }
    }

    cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;

    tracing::info!(
        objects_copied,
        assets_fixed,
        variants_fixed,
        photos_resynced,
        "legacy key repair done"
    );
    Ok(RepairResp {
        objects_copied,
        assets_fixed,
        variants_fixed,
        photos_resynced,
    })
}

fn db_err(error: DbErr) -> AppError {
    match error {
        DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}

fn redis_err(error: fred::error::RedisError) -> AppError {
    AppError::Other(error.into())
}

fn category_filter(query: &PhotoQuery) -> Option<&str> {
    query
        .category
        .as_deref()
        .filter(|category| !category.is_empty() && *category != "all")
}

fn skipped_ids(requested: &[i64], existing: &[i64]) -> Vec<i64> {
    let existing_set: HashSet<i64> = existing.iter().copied().collect();
    requested
        .iter()
        .copied()
        .filter(|id| !existing_set.contains(id))
        .collect()
}

fn object_key_from_stored_url(value: &str, prefixes: &[&str]) -> Option<String> {
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return None;
    }

    let (start, prefix) = prefixes
        .iter()
        .filter_map(|prefix| value.find(prefix).map(|index| (index, *prefix)))
        .min_by_key(|(index, _)| *index)?;
    let key_with_query = &value[start..];
    let end = key_with_query
        .find(|c| c == '?' || c == '#')
        .unwrap_or(key_with_query.len());
    let key = key_with_query[..end].trim_matches('/');
    if key.is_empty() || !key.starts_with(prefix) {
        None
    } else {
        Some(key.to_owned())
    }
}

fn normalize_update_src(
    src: &str,
    current_asset: &media_assets::Model,
    variants: &[media_variants::Model],
) -> String {
    if let Some(key) = object_key_from_stored_url(src, &["uploads/", "variants/"]) {
        let is_current_display_url = key == current_asset.storage_key
            || variants.iter().any(|variant| variant.storage_key == key);
        if is_current_display_url {
            return current_asset.storage_key.clone();
        }
        return key;
    }
    normalize_stored_src(src)
}

fn normalize_stored_src(src: &str) -> String {
    object_key_from_stored_url(src, &["uploads/", "variants/"]).unwrap_or_else(|| src.to_owned())
}

/// 构造 NewPhoto。passcode 规则：
/// - privacy != Locked：强制清空（公开/私密照片不需要口令）
/// - privacy == Locked：
///   · req.passcode 非空 → 用它（前端传新口令）
///   · req.passcode 为 None 或空字符串 → fallback 用 `prev_passcode`（编辑时不动旧口令）
///   · 都没有 → 默认 "1234"（首次设 Locked 给个兜底，避免空口令）
fn new_photo_from_req(
    slug: String,
    req: PhotoReq,
    prev_passcode: Option<String>,
    src_url: String,
) -> NewPhoto {
    let caption_zh = req.caption.as_ref().map(|i18n| i18n.zh.clone());
    let caption_en = req.caption.as_ref().map(|i18n| i18n.en.clone());
    let alt_text_zh = req.alt_text.as_ref().map(|i18n| i18n.zh.clone());
    let alt_text_en = req.alt_text.as_ref().map(|i18n| i18n.en.clone());

    let passcode = match req.privacy {
        Privacy::Locked => req
            .passcode
            .as_ref()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .or(prev_passcode)
            .or_else(|| Some("1234".to_owned())),
        Privacy::Public | Privacy::Private => None,
    };

    NewPhoto {
        slug,
        category_slug: req.cat,
        src_url,
        mime_type: "image/jpeg".to_owned(),
        privacy: req.privacy,
        passcode,
        taken_at_label: req.date,
        title_zh: req.title.zh,
        title_en: req.title.en,
        loc_zh: req.loc.zh,
        loc_en: req.loc.en,
        caption_zh,
        caption_en,
        alt_text_zh,
        alt_text_en,
        tag_ids: req.tag_ids,
    }
}

async fn create_slug(state: &AppState, req: &PhotoReq) -> AppResult<String> {
    match normalized_slug(req.slug.as_deref()) {
        Some(slug) => {
            validate_slug(&slug)?;
            if photo_repo::find_by_slug(&state.db, &slug)
                .await
                .map_err(db_err)?
                .is_some()
            {
                return Err(AppError::Conflict("slug already exists"));
            }
            Ok(slug)
        }
        None => Ok(format!("photo-{}", Uuid::new_v4())),
    }
}

async fn update_slug(
    state: &AppState,
    id: i64,
    req: &PhotoReq,
    current_slug: &str,
) -> AppResult<String> {
    let slug = match normalized_slug(req.slug.as_deref()) {
        Some(slug) => slug,
        None => current_slug.to_owned(),
    };
    validate_slug(&slug)?;
    if let Some(existing) = photo_repo::find_by_slug(&state.db, &slug)
        .await
        .map_err(db_err)?
    {
        if existing.id != id {
            return Err(AppError::Conflict("slug already exists"));
        }
    }
    Ok(slug)
}

fn normalized_slug(slug: Option<&str>) -> Option<String> {
    slug.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn validate_slug(slug: &str) -> AppResult<()> {
    let len_ok = (1..=80).contains(&slug.len());
    let chars_ok = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if len_ok && chars_ok {
        Ok(())
    } else {
        Err(AppError::Validation(
            "slug must match [a-z0-9-]{1,80}".to_owned(),
        ))
    }
}

fn i18n_optional(zh: Option<String>, en: Option<String>) -> Option<I18nText> {
    if zh.is_none() && en.is_none() {
        return None;
    }
    Some(I18nText {
        zh: zh.unwrap_or_default(),
        en: en.unwrap_or_default(),
    })
}

fn tag_summary(tag: tags::Model) -> TagSummary {
    TagSummary {
        id: tag.id,
        slug: tag.slug,
        name: I18nText {
            zh: tag
                .name_i18n
                .get("zh")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            en: tag
                .name_i18n
                .get("en")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        },
    }
}

fn validate(req: &PhotoReq) -> AppResult<()> {
    if req.src.trim().is_empty() {
        return Err(AppError::Validation("image url is required".into()));
    }
    if req.title.zh.trim().is_empty() {
        return Err(AppError::Validation("zh title is required".into()));
    }
    if !["street", "landscape", "life"].contains(&req.cat.as_str()) {
        return Err(AppError::Validation("unknown category".into()));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct DemoPhoto {
    src: &'static str,
    cat: &'static str,
    title_zh: &'static str,
    title_en: &'static str,
    loc_zh: &'static str,
    loc_en: &'static str,
    date: &'static str,
    privacy: Privacy,
}

const DEMO_PHOTOS: &[DemoPhoto] = &[
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=900&q=80",
        cat: "landscape",
        title_zh: "松岭",
        title_en: "Pine Ridge",
        loc_zh: "北海道",
        loc_en: "Hokkaido, JP",
        date: "2024.11",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1519681393784-d120267933ba?w=900&q=80",
        cat: "landscape",
        title_zh: "冰川的安静",
        title_en: "Glacial Silence",
        loc_zh: "班夫",
        loc_en: "Banff, CA",
        date: "2024.08",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1444703686981-a3abbc4d4fe3?w=900&q=80",
        cat: "landscape",
        title_zh: "极光",
        title_en: "Aurora",
        loc_zh: "特罗姆瑟",
        loc_en: "Tromsø, NO",
        date: "2024.02",
        privacy: Privacy::Locked,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1493514789931-586cb221d7a7?w=900&q=80",
        cat: "street",
        title_zh: "斑马线",
        title_en: "Crosswalk",
        loc_zh: "东京",
        loc_en: "Tokyo, JP",
        date: "2024.05",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1473496169904-658ba7c44d8a?w=900&q=80",
        cat: "street",
        title_zh: "黄色出租车",
        title_en: "Yellow Cab",
        loc_zh: "纽约",
        loc_en: "New York, US",
        date: "2023.10",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1517021897933-0e0319cfbc28?w=900&q=80",
        cat: "street",
        title_zh: "地铁线条",
        title_en: "Subway Lines",
        loc_zh: "首尔",
        loc_en: "Seoul, KR",
        date: "2024.03",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=900&q=80",
        cat: "landscape",
        title_zh: "山中湖",
        title_en: "Mountain Lake",
        loc_zh: "阿尔卑斯",
        loc_en: "Alps, CH",
        date: "2023.07",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1464822759023-fed622ff2c3b?w=900&q=80",
        cat: "landscape",
        title_zh: "雾中山口",
        title_en: "Foggy Pass",
        loc_zh: "法罗群岛",
        loc_en: "Faroe Islands",
        date: "2023.09",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1507608616759-54f48f0af0ee?w=900&q=80",
        cat: "life",
        title_zh: "晨间一壶",
        title_en: "Morning Pour",
        loc_zh: "工作室",
        loc_en: "Studio",
        date: "2024.04",
        privacy: Privacy::Private,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1495365200479-c4ed1d35e1aa?w=900&q=80",
        cat: "life",
        title_zh: "阅读的光",
        title_en: "Reading Light",
        loc_zh: "家中",
        loc_en: "Home",
        date: "2024.01",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1465146344425-f00d5f5c8f07?w=900&q=80",
        cat: "life",
        title_zh: "手与花瓣",
        title_en: "Hands & Petals",
        loc_zh: "巴黎",
        loc_en: "Paris, FR",
        date: "2023.06",
        privacy: Privacy::Locked,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1485081669829-bacb8c7bb1f3?w=900&q=80",
        cat: "street",
        title_zh: "雨天的街",
        title_en: "Rainy Avenue",
        loc_zh: "伦敦",
        loc_en: "London, UK",
        date: "2024.06",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1500382017468-9049fed747ef?w=900&q=80",
        cat: "landscape",
        title_zh: "麦田",
        title_en: "Wheat Field",
        loc_zh: "普罗旺斯",
        loc_en: "Provence, FR",
        date: "2023.08",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1469474968028-56623f02e42e?w=900&q=80",
        cat: "landscape",
        title_zh: "穿过松林的光",
        title_en: "Sun Through Pines",
        loc_zh: "俄勒冈",
        loc_en: "Oregon, US",
        date: "2024.07",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1444930694458-01babe71870e?w=900&q=80",
        cat: "life",
        title_zh: "安静的伴",
        title_en: "Quiet Companion",
        loc_zh: "里斯本",
        loc_en: "Lisbon, PT",
        date: "2024.09",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1455741292689-49d9bd47c6e9?w=900&q=80",
        cat: "street",
        title_zh: "靠窗的座",
        title_en: "Window Seat",
        loc_zh: "柏林",
        loc_en: "Berlin, DE",
        date: "2024.02",
        privacy: Privacy::Private,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1469854523086-cc02fe5d8800?w=900&q=80",
        cat: "landscape",
        title_zh: "潮线",
        title_en: "Tideline",
        loc_zh: "冰岛",
        loc_en: "Iceland",
        date: "2023.11",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1501785888041-af3ef285b470?w=900&q=80",
        cat: "landscape",
        title_zh: "倒影",
        title_en: "Reflections",
        loc_zh: "哈尔施塔特",
        loc_en: "Hallstatt, AT",
        date: "2024.10",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=900&q=80&sat=-100",
        cat: "life",
        title_zh: "初霜",
        title_en: "First Frost",
        loc_zh: "院子",
        loc_en: "Garden",
        date: "2025.01",
        privacy: Privacy::Locked,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1517677208171-0bc6725a3e60?w=900&q=80",
        cat: "street",
        title_zh: "清晨六点的市场",
        title_en: "Market, 6 a.m.",
        loc_zh: "河内",
        loc_en: "Hanoi, VN",
        date: "2024.12",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=900&q=80&blur=10",
        cat: "landscape",
        title_zh: "最后的光",
        title_en: "Last Light",
        loc_zh: "多洛米蒂",
        loc_en: "Dolomites, IT",
        date: "2025.02",
        privacy: Privacy::Public,
    },
    DemoPhoto {
        src: "https://images.unsplash.com/photo-1490750967868-88aa4486c946?w=900&q=80",
        cat: "life",
        title_zh: "花瓣习作",
        title_en: "Petal Study",
        loc_zh: "工作室",
        loc_en: "Studio",
        date: "2024.05",
        privacy: Privacy::Public,
    },
];
