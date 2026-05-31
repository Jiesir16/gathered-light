//! AppState 装配（≈ Spring 的 IoC 容器，但容器本身只是个 `Arc<struct>`）。
//!
//! M1：接入 SeaORM + fred(Redis) + JwtKeys。
//! M2：接入 aws-sdk-s3 + image worker channel。

use std::{sync::Arc, time::Duration};

use aws_config::BehaviorVersion;
use aws_credential_types::{Credentials, provider::SharedCredentialsProvider};
use aws_sdk_s3::{Client as S3Client, config::Region};
use fred::prelude::{Builder, ClientLike, RedisConfig};
use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::{Config, JwtCfg, S3Cfg};

#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl JwtKeys {
    fn from_config(cfg: &JwtCfg) -> Self {
        let secret = cfg.secret.as_bytes();
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DatabaseConnection,
    pub redis: fred::clients::RedisPool,
    pub s3: S3Client,
    pub jwt: Arc<JwtKeys>,
}

impl AppState {
    pub async fn init(cfg: Config) -> anyhow::Result<Self> {
        let mut db_options = ConnectOptions::new(cfg.database.url.clone());
        db_options
            .min_connections(cfg.database.pool_min)
            .max_connections(cfg.database.pool_max)
            .connect_timeout(Duration::from_secs(5))
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy(true)
            .sqlx_logging(false);

        let db = Database::connect(db_options).await?;
        tracing::info!(
            min = cfg.database.pool_min,
            max = cfg.database.pool_max,
            "postgres pool ready (min=2 max=16)"
        );

        let redis_config = RedisConfig::from_url(&cfg.redis.url)?;
        let redis = Builder::from_config(redis_config).build_pool(cfg.redis.pool_size)?;
        let _redis_task = redis.init().await?;
        let _: String = redis.ping().await?;
        tracing::info!("redis ping ok");

        let s3 = init_s3(&cfg.s3).await?;
        let jwt = Arc::new(JwtKeys::from_config(&cfg.jwt));

        Ok(Self {
            config: Arc::new(cfg),
            db,
            redis,
            s3,
            jwt,
        })
    }
}

/// 把误含桶名子域的 endpoint 规整成地域级：`https://{bucket}.cos.x` → `https://cos.x`。
/// 不含该前缀（已是地域级 / MinIO 等）则原样返回。
///
/// 根因：endpoint 带桶名 + `force_path_style(true)` 会把桶名既放进 host 又拼进 path，
/// COS 把整段 path 当 key → 对象 key 被双写成 `{bucket}/...`。规整后桶名只出现一次，
/// 上传/HEAD/ACL/复制全部寻址干净 key，与自定义域名直链一致。
fn normalize_endpoint(endpoint: &str, bucket: &str) -> String {
    endpoint.replace(&format!("://{bucket}."), "://")
}

fn is_production_env() -> bool {
    std::env::var("APP_ENV").as_deref() == Ok("production")
}

fn is_local_s3_endpoint(endpoint: &str) -> bool {
    let endpoint = endpoint.to_ascii_lowercase();
    endpoint.starts_with("http://localhost:")
        || endpoint.starts_with("http://127.0.0.1:")
        || endpoint.starts_with("http://[::1]:")
}

async fn init_s3(cfg: &S3Cfg) -> anyhow::Result<S3Client> {
    let creds = Credentials::new(
        cfg.access_key.clone(),
        cfg.secret_key.clone(),
        None,
        None,
        "gathered-light-static",
    );
    let endpoint = normalize_endpoint(&cfg.endpoint, &cfg.bucket);
    if endpoint != cfg.endpoint {
        tracing::info!(raw = %cfg.endpoint, used = %endpoint, "s3 endpoint 规整为地域级（剥掉桶名子域）");
    }
    let aws_cfg = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(cfg.region.clone()))
        .endpoint_url(endpoint.clone())
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .load()
        .await;
    let s3_cfg = aws_sdk_s3::config::Builder::from(&aws_cfg)
        .force_path_style(true)
        .build();
    let client = S3Client::from_conf(s3_cfg);

    match client.head_bucket().bucket(&cfg.bucket).send().await {
        Ok(_) => tracing::info!(bucket = %cfg.bucket, "s3 bucket exists"),
        Err(error) => {
            let service_error = error.into_service_error();
            if service_error.is_not_found() && is_local_s3_endpoint(&endpoint) {
                client
                    .create_bucket()
                    .bucket(&cfg.bucket)
                    .send()
                    .await
                    .map_err(|error| anyhow::anyhow!("create bucket: {error}"))?;
                tracing::info!(bucket = %cfg.bucket, "s3 bucket created");
            } else if is_production_env() {
                tracing::warn!(
                    bucket = %cfg.bucket,
                    endpoint = %endpoint,
                    region = %cfg.region,
                    error = ?service_error,
                    "s3 bucket startup probe failed; continuing because production startup must not depend on HeadBucket"
                );
            } else {
                anyhow::bail!("head_bucket failed: {service_error}");
            }
        }
    }

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::{is_local_s3_endpoint, normalize_endpoint};

    #[test]
    fn strips_bucket_subdomain_to_region_level() {
        assert_eq!(
            normalize_endpoint(
                "https://my-bucket-1300000000.cos.ap-x.myqcloud.com",
                "my-bucket-1300000000"
            ),
            "https://cos.ap-x.myqcloud.com"
        );
    }

    #[test]
    fn leaves_region_level_and_minio_untouched() {
        // 已是地域级：原样
        assert_eq!(
            normalize_endpoint("https://cos.ap-x.myqcloud.com", "my-bucket-1300000000"),
            "https://cos.ap-x.myqcloud.com"
        );
        // dev MinIO：原样
        assert_eq!(
            normalize_endpoint("http://localhost:9000", "gathered-light"),
            "http://localhost:9000"
        );
    }

    #[test]
    fn detects_local_s3_endpoint_for_bucket_auto_create() {
        assert!(is_local_s3_endpoint("http://localhost:9000"));
        assert!(is_local_s3_endpoint("http://127.0.0.1:9000"));
        assert!(!is_local_s3_endpoint(
            "https://cos.ap-guangzhou.myqcloud.com"
        ));
        assert!(!is_local_s3_endpoint("https://media.example.com"));
    }
}
