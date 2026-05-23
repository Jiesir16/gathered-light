//! 程序入口（≈ Spring Boot 的 main + @SpringBootApplication）。
//!
//! 启动顺序：
//!     1. 读取本地 `.env`，让 RUST_LOG 等启动期变量先进入环境
//!     2. 装配 tracing（必须早，否则后续 tracing! 会被丢弃）
//!     3. 加载配置（figment：默认 toml ◀ env override）
//!     4. 构造 AppState（DB / Redis / S3 / JWT 等客户端 — M1+）
//!     5. spawn 后台 worker（M2+）
//!     6. 启动 axum，挂优雅停机

use cms_api::{bootstrap, config, observability, routes, workers};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    observability::init();

    let cfg = config::load()?;
    let bind = cfg.bind_addr.clone();
    tracing::info!(?cfg, "✅ config loaded");

    let state = bootstrap::AppState::init(cfg).await?;
    tokio::spawn(workers::image_processor::run(state.clone()));
    let app = routes::build(state);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(addr = %bind, "🌅 gathered-light · cms-api is listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// SIGINT (Ctrl+C) / SIGTERM 任一触发即开始优雅停机。
async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        if let Err(error) = signal::ctrl_c().await {
            tracing::error!(error = ?error, "install Ctrl+C handler failed");
        }
    };
    #[cfg(unix)]
    let term = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(error = ?error, "install SIGTERM handler failed");
            }
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("📦 SIGINT received"),
        _ = term   => tracing::info!("📦 SIGTERM received"),
    }
}
