//! gathered-light · cms-api
//!
//! 分层结构（≈ 一个 Spring Boot 模块的拆分）：
//!
//! ```text
//! handlers   ↔ Controller    （请求/响应、状态码、入参校验）
//! services   ↔ Service       （编排、事务、缓存策略）
//! repositories ↔ Mapper / Repository（纯 SeaORM 数据访问）
//! middleware ↔ Interceptor / Filter（Tower Layer）
//! infra      ↔ infrastructure（Redis / S3 / Search 客户端封装）
//! workers    ↔ @Async         （后台任务）
//! dto        ↔ VO / DTO       （跨层输入输出形状）
//! ```

pub mod bootstrap;
pub mod config;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod infra;
pub mod middleware;
pub mod observability;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod workers;
