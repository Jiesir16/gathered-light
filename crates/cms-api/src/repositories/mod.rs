//! Repository 层（≈ Spring Data JPA / MyBatis Mapper）：纯 SeaORM 数据访问。
//!
//! 约定：
//!   - 入参只接收 `&DatabaseConnection` 或 `&DatabaseTransaction`，**不持有状态**。
//!   - 不读 Redis、不调其它 service，避免循环。
//!   - 返回 `Result<T, sea_orm::DbErr>`，由 service 层 `?` 上抛后转 `AppError`。
//!
//! Roadmap：
//!   - `photo_repo.rs`     M1 PR-2
//!   - `user_repo.rs`      M1
//!   - `media_repo.rs`     M2
//!   - `category_repo.rs`  M3

pub mod category_repo;
pub mod media_repo;
pub mod photo_repo;
pub mod post_repo;
pub mod settings_repo;
pub mod tag_repo;
pub mod user_repo;
