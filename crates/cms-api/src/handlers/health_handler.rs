//! `/healthz` 进程存活；`/readyz` 依赖就绪。

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use fred::prelude::ClientLike;
use serde::Serialize;

use crate::bootstrap::AppState;

#[derive(Serialize)]
pub struct HealthResp {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

pub async fn healthz(State(_state): State<AppState>) -> Json<HealthResp> {
    Json(HealthResp {
        status: "ok",
        service: "gathered-light · cms-api",
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(error) = state.db.ping().await {
        tracing::error!(error = ?error, "postgres readyz ping failed");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResp {
                status: "not_ready",
                service: "gathered-light · cms-api",
                version: env!("CARGO_PKG_VERSION"),
            }),
        );
    }

    if let Err(error) = state.redis.ping::<String>().await {
        tracing::error!(error = ?error, "redis readyz ping failed");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResp {
                status: "not_ready",
                service: "gathered-light · cms-api",
                version: env!("CARGO_PKG_VERSION"),
            }),
        );
    }

    (
        StatusCode::OK,
        Json(HealthResp {
            status: "ready",
            service: "gathered-light · cms-api",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}
