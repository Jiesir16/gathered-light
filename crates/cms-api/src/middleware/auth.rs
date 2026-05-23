use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{bootstrap::AppState, error::AppError, infra::jwt::Claims, services::auth_service};

pub async fn jwt_guard(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let claims: Claims = auth_service::require_access(&state, req.headers()).await?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
