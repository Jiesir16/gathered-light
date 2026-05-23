//! 后台异步任务（≈ Spring `@Async` / Spring Task / 简单消息消费者）。
//!
//! 默认实现：进程内 mpsc + tokio::spawn；
//! 高一致场景升级：pgmq（PostgreSQL）或 Redis Streams XADD/XREADGROUP。
//!
//! Roadmap：
//!   - `image_processor.rs` M2：HEAD asset → 解码 → 生成 thumb_400 / medium_900 /
//!     full_1800 / webp_900 → 写 media_variants → 更新 status=ready

pub mod image_processor;
