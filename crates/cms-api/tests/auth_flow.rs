//! 鉴权闭环：login → admin → logout → admin 401（Redis 黑名单生效）。

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn login_admin_logout_blacklist_full_loop() {
    let (app, _state) = common::build_app().await;

    // 1) 错密码必 401
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(common::json_body(&serde_json::json!({
                    "email": "admin@test.dev", "password": "wrong"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "wrong password should 401"
    );

    // 2) 正确密码 → 拿 access + refresh
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(common::json_body(&serde_json::json!({
                    "email": "admin@test.dev", "password": "Passw0rd!"
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::read_json(resp).await;
    let access = body["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    assert!(!access.is_empty());

    // 3) 无 token → admin/photos 必 401
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/photos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no token → 401");

    // 4) 带 token → admin/photos 必 200
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/photos")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "with token → 200");

    // 5) logout → 黑名单写入
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6) 同 token 再访问 admin/photos 必 401（黑名单生效）
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/photos")
                .header("authorization", format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "after logout → 401 from blacklist"
    );
}
