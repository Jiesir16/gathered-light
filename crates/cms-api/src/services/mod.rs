//! Service 层（≈ Spring `@Service`）：业务编排 + 缓存策略 + 事务边界。
//!
//! 约定：
//!   - 函数式风格：`pub async fn xxx(state: &AppState, ...) -> AppResult<T>`，
//!     不强制做成 struct trait（Spring 的 `@Service` 单例语义在这里被 `Arc<AppState>` 取代）。
//!   - 事务边界**由 service 层声明**（`db.begin().await?`），repository 不感知事务。
//!   - 缓存策略**由 service 层声明**（先 redis → 回源 → 回写），repository 不读 redis。
//!
//! Roadmap：
//!   - `auth_service.rs`     M1
//!   - `photo_service.rs`    M3
//!   - `media_service.rs`    M2 (presign + complete)
//!   - `category_service.rs` M3

pub mod auth_service;
pub mod category_service;
pub mod content_render;
pub mod dashboard_service;
pub mod media_service;
pub mod photo_service;
pub mod post_service;
pub mod settings_service;
pub mod tag_service;
pub mod user_service;
