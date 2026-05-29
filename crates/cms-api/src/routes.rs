//! 路由总装（≈ 一份大表 + 各 Controller 子模块）。
//!
//! 设计原则：
//!   - **handler 内不组路由**，只暴露处理函数，路由表集中在这里。
//!   - **中间件分层**：洋葱模型（外层日志 → 限流 → CORS → 鉴权 → 业务）。

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{
        HeaderValue,
        header::{CACHE_CONTROL, EXPIRES, PRAGMA},
    },
    middleware,
    response::Response,
    routing::{get, patch, post, put},
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use crate::bootstrap::AppState;
use crate::handlers::{
    auth_handler, category_handler, dashboard_handler, health_handler, media_handler,
    photo_handler, settings_handler, tag_handler, user_handler,
};
use crate::middleware::auth::jwt_guard;

pub fn build(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health_handler::healthz))
        .route("/readyz", get(health_handler::readyz));

    let public_api = Router::new()
        .route("/auth/login", post(auth_handler::login))
        .route("/auth/refresh", post(auth_handler::refresh))
        .route("/settings", get(settings_handler::get_public))
        .route("/categories", get(category_handler::list))
        .route("/tags", get(tag_handler::list_public))
        .route("/photos", get(photo_handler::list_public))
        .route("/photos/:id/unlock", post(photo_handler::unlock));

    let protected_api = Router::new()
        .route("/auth/logout", post(auth_handler::logout))
        .route("/auth/logout-all", post(auth_handler::logout_all))
        .route("/auth/me", get(auth_handler::me))
        .route("/admin/dashboard", get(dashboard_handler::get_overview))
        .route(
            "/admin/settings/theme",
            patch(settings_handler::update_theme),
        )
        .route(
            "/admin/settings/range",
            patch(settings_handler::update_range),
        )
        .route("/admin/settings/hero", patch(settings_handler::update_hero))
        .route("/admin/media/presign", post(media_handler::presign))
        .route("/admin/media/complete", post(media_handler::complete))
        .route(
            "/admin/photos",
            get(photo_handler::list_admin).post(photo_handler::create),
        )
        .route(
            "/admin/photos/bulk/delete",
            post(photo_handler::bulk_delete),
        )
        .route(
            "/admin/photos/bulk/privacy",
            post(photo_handler::bulk_update_privacy),
        )
        .route(
            "/admin/photos/bulk/tags",
            post(photo_handler::bulk_set_tags),
        )
        .route("/admin/photos/reset", post(photo_handler::reset))
        .route(
            "/admin/photos/:id",
            put(photo_handler::update).delete(photo_handler::delete),
        )
        .route(
            "/admin/photos/:id/privacy",
            patch(photo_handler::update_privacy),
        )
        .route(
            "/admin/photos/:id/recover-url",
            post(photo_handler::recover_urls),
        )
        .route(
            "/admin/tags",
            get(tag_handler::list_admin).post(tag_handler::create),
        )
        .route(
            "/admin/tags/:id",
            patch(tag_handler::update).delete(tag_handler::delete),
        )
        .route(
            "/admin/categories",
            get(category_handler::list_admin).post(category_handler::create),
        )
        .route(
            "/admin/categories/:id",
            patch(category_handler::update).delete(category_handler::delete),
        )
        // GET /api/v1/admin/users
        // POST /api/v1/admin/users
        // PATCH /api/v1/admin/users/:id
        // POST /api/v1/admin/users/:id/password
        // DELETE /api/v1/admin/users/:id
        .route(
            "/admin/users",
            get(user_handler::list).post(user_handler::create),
        )
        .route(
            "/admin/users/:id/password",
            post(user_handler::reset_password),
        )
        .route(
            "/admin/users/:id",
            patch(user_handler::update).delete(user_handler::delete),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_guard));

    let api = public_api
        .merge(protected_api)
        .layer(middleware::from_fn(no_store_api));

    Router::new()
        .merge(public)
        .nest("/api/v1", api)
        .with_state(state)
        .fallback_service(
            ServeDir::new("frontend/dist")
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new("frontend/dist/index.html")),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(tracing::Level::INFO)
                        .include_headers(false),
                )
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(CorsLayer::permissive())
}

async fn no_store_api(req: Request<Body>, next: middleware::Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(EXPIRES, HeaderValue::from_static("0"));
    response
}
