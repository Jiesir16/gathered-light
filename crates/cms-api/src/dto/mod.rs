//! 跨层 DTO 集中地（≈ Java 里的 VO / DTO 包）。
//!
//! 命名约定：
//!   - `XxxReq`   入参（POST / PATCH 请求体或 Query）
//!   - `XxxResp`  出参（顶层响应 envelope）
//!   - `XxxDto`   业务对象（list / detail 元素）
//!   - `I18nField { zh, en }` 与前端原型 `data.jsx` 中 `title.zh / title.en` 对齐
//!
//! Roadmap：
//!   - `photo_dto.rs`    M3
//!   - `auth_dto.rs`     M1
//!   - `media_dto.rs`    M2

pub mod auth_dto;
pub mod category_dto;
pub mod dashboard_dto;
pub mod media_dto;
pub mod photo_dto;
pub mod tag_dto;
pub mod user_dto;
