//! 配置加载（≈ Spring `application.yml` + `@ConfigurationProperties`）。
//!
//! 优先级（由低到高）：`config/default.toml` < 本地 `.env` < 环境变量 `APP_*`。
//! 嵌套字段使用 `__` 分隔：`APP_DATABASE__URL` → `database.url`。

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::Deserialize;
use std::{fmt, path::PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind_addr: String,
    pub database: DatabaseCfg,
    pub redis: RedisCfg,
    pub s3: S3Cfg,
    pub jwt: JwtCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseCfg {
    pub url: String,
    #[serde(default = "DatabaseCfg::default_pool_min")]
    pub pool_min: u32,
    #[serde(default = "DatabaseCfg::default_pool_max")]
    pub pool_max: u32,
}
impl DatabaseCfg {
    const fn default_pool_min() -> u32 {
        2
    }
    const fn default_pool_max() -> u32 {
        16
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisCfg {
    pub url: String,
    #[serde(default = "RedisCfg::default_pool")]
    pub pool_size: usize,
}
impl RedisCfg {
    const fn default_pool() -> usize {
        8
    }
}

#[derive(Clone, Deserialize)]
pub struct S3Cfg {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(default = "S3Cfg::default_upload_max")]
    pub upload_max_bytes: u64,
}
impl S3Cfg {
    const fn default_upload_max() -> u64 {
        25 * 1024 * 1024
    }
}

impl fmt::Debug for S3Cfg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Cfg")
            .field("endpoint", &self.endpoint)
            .field("region", &self.region)
            .field("bucket", &self.bucket)
            .field("access_key", &"<redacted>")
            .field("secret_key", &"<redacted>")
            .field("upload_max_bytes", &self.upload_max_bytes)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct JwtCfg {
    pub access_ttl_secs: u64,
    pub refresh_ttl_secs: u64,
    #[serde(default = "JwtCfg::default_secret")]
    pub secret: String,
    #[serde(default)]
    pub private_key_path: Option<PathBuf>,
    #[serde(default)]
    pub public_key_path: Option<PathBuf>,
}

impl JwtCfg {
    fn default_secret() -> String {
        "dev-secret-change-me".into()
    }
}

impl fmt::Debug for JwtCfg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtCfg")
            .field("access_ttl_secs", &self.access_ttl_secs)
            .field("refresh_ttl_secs", &self.refresh_ttl_secs)
            .field("secret", &"<redacted>")
            .field("private_key_path", &self.private_key_path)
            .field("public_key_path", &self.public_key_path)
            .finish()
    }
}

pub fn load() -> anyhow::Result<Config> {
    let _ = dotenvy::dotenv();
    let path = std::env::var("APP_CONFIG").unwrap_or_else(|_| "config/default.toml".into());
    let cfg: Config = Figment::new()
        .merge(Toml::file(&path))
        .merge(Env::prefixed("APP_").split("__"))
        .extract()?;
    Ok(cfg)
}
