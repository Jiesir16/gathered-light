//! 全局错误模型（≈ `@RestControllerAdvice` + `@ExceptionHandler`）。
//!
//! 约定：handler/service 全部返回 `AppResult<T>`；
//! 凡 5xx 错误**只对外暴露 code**，详情走 tracing::error! 落日志。

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(&'static str),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("unauthorized: {0}")]
    Unauthorized(&'static str),

    #[error("forbidden")]
    Forbidden,

    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(&'static str),

    #[error("rate limited: {0}")]
    TooManyRequests(&'static str),

    #[error("internal error: {0}")]
    Internal(&'static str),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::BadRequest(_) | AppError::Validation(_) => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST")
            }
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::TooManyRequests(_) => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
            AppError::Internal(_) | AppError::Other(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL")
            }
        };
        if status.is_server_error() {
            tracing::error!(error = ?self, "server error");
        }
        (
            status,
            Json(json!({ "code": code, "message": self.to_string() })),
        )
            .into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
