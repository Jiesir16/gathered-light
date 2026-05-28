use std::str::FromStr;

use cms_domain::Privacy;
use serde_json::Value;

use crate::{
    bootstrap::AppState,
    dto::{
        dashboard_dto::{CategoryCount, DashboardResp, PhotoStats, PhotoSummary},
        photo_dto::I18nText,
    },
    error::{AppError, AppResult},
    repositories::{category_repo, media_repo, photo_repo, tag_repo, user_repo},
    services::photo_service,
};

pub async fn overview(state: &AppState) -> AppResult<DashboardResp> {
    let total = photo_repo::count(&state.db).await.map_err(db_err)?;
    let by_privacy = photo_repo::count_by_privacy(&state.db)
        .await
        .map_err(db_err)?;
    let by_category = photo_repo::count_by_category(&state.db)
        .await
        .map_err(db_err)?
        .into_iter()
        .map(|(slug, name_json, count)| CategoryCount {
            slug,
            name: i18n_from_json(&name_json),
            count,
        })
        .collect();

    let recent_fulls = photo_repo::list_recent(&state.db, 5)
        .await
        .map_err(db_err)?;
    let asset_ids: Vec<i64> = recent_fulls.iter().map(|photo| photo.asset.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids)
        .await
        .map_err(db_err)?;
    let mut recent = Vec::with_capacity(recent_fulls.len());
    for photo in recent_fulls {
        let variants = variants_map
            .get(&photo.asset.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let privacy = Privacy::from_str(&photo.photo.privacy).unwrap_or(Privacy::Private);
        let src = photo_service::cover_url_for(state, &photo.asset, variants, privacy).await?;
        recent.push(PhotoSummary {
            id: photo.photo.id,
            slug: photo.photo.slug,
            title: I18nText {
                zh: photo.title_zh,
                en: photo.title_en,
            },
            src,
            created_at: photo.photo.created_at.to_rfc3339(),
        });
    }

    let media = media_repo::count_by_status(&state.db)
        .await
        .map_err(db_err)?;
    let users_total = user_repo::count(&state.db).await.map_err(db_err)?;
    let tags_total = tag_repo::count(&state.db).await.map_err(db_err)?;
    let categories_total = category_repo::count(&state.db).await.map_err(db_err)?;

    Ok(DashboardResp {
        photos: PhotoStats {
            total,
            by_privacy,
            by_category,
            recent,
        },
        media,
        users_total,
        tags_total,
        categories_total,
    })
}

fn i18n_from_json(value: &Value) -> I18nText {
    I18nText {
        zh: value
            .get("zh")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        en: value
            .get("en")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    }
}

fn db_err(error: sea_orm::DbErr) -> AppError {
    match error {
        sea_orm::DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}
