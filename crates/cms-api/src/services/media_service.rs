use std::{collections::BTreeMap, time::Duration};

use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::ObjectCannedAcl;
use sea_orm::DbErr;

use crate::{
    bootstrap::AppState,
    config::S3Cfg,
    dto::media_dto::{CompleteReq, CompleteResp, PresignReq, PresignResp},
    error::{AppError, AppResult},
    repositories::media_repo::{self, NewAsset},
};

pub async fn presign_upload(state: &AppState, req: PresignReq) -> AppResult<PresignResp> {
    if !req.mime_type.starts_with("image/") {
        return Err(AppError::Validation("only image uploads".to_owned()));
    }
    let max = state.config.s3.upload_max_bytes;
    if req.byte_size == 0 || req.byte_size > max {
        return Err(AppError::Validation(format!("file size 1..={max}")));
    }

    let safe = sanitize_file_name(&req.file_name);
    let storage_key = format!(
        "uploads/{}/{}-{}",
        chrono::Utc::now().format("%Y/%m"),
        uuid::Uuid::new_v4(),
        safe
    );

    let presigning = PresigningConfig::expires_in(Duration::from_secs(900)).map_err(|error| {
        tracing::error!(error = ?error, "build presigning config failed");
        AppError::Internal("presign failed")
    })?;
    let request = state
        .s3
        .put_object()
        .bucket(&state.config.s3.bucket)
        .key(&storage_key)
        .content_type(&req.mime_type)
        .content_length(req.byte_size as i64)
        .presigned(presigning)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "presign failed");
            AppError::Internal("presign failed")
        })?;

    let upload_url = request.uri().to_string();
    let public_url = permanent_url(state, &storage_key);

    let mut headers = BTreeMap::new();
    headers.insert("content-type".to_owned(), req.mime_type);

    Ok(PresignResp {
        method: "PUT",
        upload_url,
        public_url,
        storage_key,
        headers,
        max_bytes: max,
        expires_in: 900,
    })
}

pub async fn complete_upload(state: &AppState, req: CompleteReq) -> AppResult<CompleteResp> {
    let head = state
        .s3
        .head_object()
        .bucket(&state.config.s3.bucket)
        .key(&req.storage_key)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(error = ?error, key = %req.storage_key, "head_object failed");
            AppError::NotFound
        })?;

    let mime_type = head
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_owned();
    let byte_size = head.content_length().unwrap_or(0);
    let max = state.config.s3.upload_max_bytes as i64;
    if !mime_type.starts_with("image/") {
        return Err(AppError::Validation("only image uploads".to_owned()));
    }
    if byte_size <= 0 || byte_size > max {
        return Err(AppError::Validation(format!("file size 1..={max}")));
    }

    let asset = media_repo::insert_with_status(
        &state.db,
        NewAsset {
            storage_key: req.storage_key.clone(),
            mime_type,
            byte_size,
        },
        "pending",
    )
    .await
    .map_err(db_err)?;

    Ok(CompleteResp {
        asset_id: asset.id,
        storage_key: req.storage_key,
        public_url: permanent_url(state, &asset.storage_key),
    })
}

pub async fn presign_get(
    s3: &S3Client,
    cfg: &S3Cfg,
    key: &str,
    ttl_secs: u64,
) -> AppResult<String> {
    let presigning =
        PresigningConfig::expires_in(Duration::from_secs(ttl_secs)).map_err(|error| {
            tracing::error!(error = ?error, key, "build GET presigning config failed");
            AppError::Internal("presign GET failed")
        })?;
    let request = s3
        .get_object()
        .bucket(&cfg.bucket)
        .key(key)
        .response_content_disposition("inline")
        .presigned(presigning)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, key, "presign GET failed");
            AppError::Internal("presign GET failed")
        })?;
    Ok(request.uri().to_string())
}

fn sanitize_file_name(input: &str) -> String {
    let s: String = input
        .chars()
        .filter_map(|ch| match ch {
            ' ' => Some('-'),
            _ if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') => Some(ch),
            _ => None,
        })
        .collect();
    if s.is_empty() {
        "upload.jpg".to_owned()
    } else {
        s
    }
}

/// 拼公开访问 URL（依赖对象 ACL=public-read）。
///
/// 如果配置了 public_base_url，就走自定义/CDN 域名；否则保持历史形态：
/// endpoint + bucket + key。注意 endpoint 仍然只用于 S3 API，不要改成 CDN 域名。
pub fn permanent_url(state: &AppState, storage_key: &str) -> String {
    if let Some(base) = state
        .config
        .s3
        .public_base_url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
    {
        format!("{}/{}", base.trim_end_matches('/'), storage_key)
    } else {
        format!(
            "{}/{}/{}",
            state.config.s3.endpoint.trim_end_matches('/'),
            state.config.s3.bucket,
            storage_key
        )
    }
}

/// 把 OSS 对象 ACL 翻成 public-read（公开照片用）。
pub async fn set_object_public(s3: &S3Client, bucket: &str, key: &str) -> AppResult<()> {
    s3.put_object_acl()
        .bucket(bucket)
        .key(key)
        .acl(ObjectCannedAcl::PublicRead)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(error = ?error, bucket, key, "set object public failed");
            AppError::Internal("set object public failed")
        })?;
    Ok(())
}

/// 把 OSS 对象 ACL 翻成 private（locked/private 用 / 切回非公开时）。
pub async fn set_object_private(s3: &S3Client, bucket: &str, key: &str) -> AppResult<()> {
    s3.put_object_acl()
        .bucket(bucket)
        .key(key)
        .acl(ObjectCannedAcl::Private)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(error = ?error, bucket, key, "set object private failed");
            AppError::Internal("set object private failed")
        })?;
    Ok(())
}

fn db_err(error: DbErr) -> AppError {
    tracing::error!(error = ?error, "database operation failed");
    AppError::Internal("database operation failed")
}
