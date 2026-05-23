//! 纯领域类型 — 不依赖任何运行时（无 tokio / sea-orm / axum）。
//!
//! 目标：让最核心的语义类型（隐私态、语言、ID newtype 等）在 CLI 工具、单测、
//! 甚至前端的 JSON Schema 生成中都能复用，不被 Web 框架绑死。

pub mod error;
pub mod photo;

pub use error::DomainError;
pub use photo::{Locale, Privacy};
