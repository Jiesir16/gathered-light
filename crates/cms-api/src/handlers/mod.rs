//! Controller 层（≈ Spring `@RestController`）。
//!
//! 约定：
//!   - 仅做参数提取（`Path` / `Query` / `Json` / `Extension`）+ 调 service + 序列化返回。
//!   - 不写 SQL、不直接调 Redis、不做事务边界控制。
//!   - 所有错误统一 `Result<_, AppError>`，由 `IntoResponse` 折成 4xx/5xx。

pub mod auth_handler;
pub mod category_handler;
pub mod dashboard_handler;
pub mod health_handler;
pub mod media_handler;
pub mod photo_handler;
pub mod post_handler;
pub mod settings_handler;
pub mod tag_handler;
pub mod user_handler;
