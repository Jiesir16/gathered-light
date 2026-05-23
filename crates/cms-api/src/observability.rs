//! tracing 装配（≈ Logback + MDC + Skywalking trace_id）。
//!
//! - dev 环境：彩色 pretty 输出，便于本地排查
//! - prod 环境：JSON 结构化输出，方便 ELK / Loki 检索
//! - OTLP 端点（`OTLP_ENDPOINT`）若设置，则注入 OpenTelemetry layer（M4 启用）

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,sea_orm=debug,sqlx=warn,hyper=warn,tower_http=info")
    });

    let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".into());
    let registry = tracing_subscriber::registry().with(filter);

    if env == "production" {
        registry
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(false)
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        registry
            .with(fmt::layer().pretty().with_target(true))
            .init();
    }

    tracing::info!(env = %env, "📡 tracing initialized");
}
