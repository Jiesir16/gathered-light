use axum::{Json, extract::State};

use crate::{
    bootstrap::AppState, dto::dashboard_dto::DashboardResp, error::AppResult,
    services::dashboard_service,
};

pub async fn get_overview(State(state): State<AppState>) -> AppResult<Json<DashboardResp>> {
    Ok(Json(dashboard_service::overview(&state).await?))
}
