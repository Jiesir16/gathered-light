//! 基础设施客户端封装。
//!
//! 这一层和 Spring 里的 `RedisTemplate` / `S3Client` 配置类是同一个生态位——
//! 把第三方 SDK 的不友好 API 收敛成业务可读的最小函数集。
//!
//! Roadmap：
//!   - `redis.rs`   M1：cache_aside! 宏 + key 命名约束
//!   - `s3.rs`      M2：presigned PUT / GET + HEAD 校验
//!   - `search.rs`  M5：Meilisearch 同步（可选）

pub mod cache;
pub mod cos_sign;
pub mod jwt;
