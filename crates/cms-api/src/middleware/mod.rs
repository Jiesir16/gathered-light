//! Tower 中间件（≈ Spring HandlerInterceptor / OncePerRequestFilter）。
//!
//! Roadmap：
//!   - `auth.rs`   M1：JWT 解析 → Redis 黑名单 + 用户级 revoke epoch
//!   - `trace.rs`  M1：request_id / trace_id 注入 span（W3C traceparent）
//!   - `rate.rs`   M2：业务级限流（unlock / presign）

pub mod auth;
