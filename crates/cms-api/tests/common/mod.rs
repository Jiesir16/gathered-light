//! 测试基础设施：**复用 dev compose 的 PG/Redis**（不再启 testcontainers）。
//!
//! 隔离方式：
//! - PG：每次 build_app 调用时 DROP+CREATE 名为 `gathered_light_test` 的库（独立于 dev `gathered_light`）
//! - Redis：用 DB 15（dev 用 DB 0）；每次 build_app 时 FLUSHDB
//! - MinIO：bucket = `gathered-light-test`（独立于 dev `gathered-light`）；首次自动创建
//!
//! 前提：本机已跑 `bash scripts/dev-up.sh --skip-dod`（gathered-pg / gathered-redis / gathered-minio Up）。
//!
//! 设计原因：testcontainers 在测试进程异常退出时会留 PG/Redis 孤儿容器（5h 不释放），
//! 复用 dev compose 0 容器副作用、与 dev 完全隔离（不同库 / 不同 redis db / 不同 bucket）。

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use cms_api::{bootstrap::AppState, config::*, repositories::user_repo, routes};
use migrations::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database};
use tokio::sync::OnceCell;

const ADMIN_DB_URL: &str = "postgres://postgres:postgres@127.0.0.1:5432/postgres";
const TEST_DB_NAME: &str = "gathered_light_test";

/// 整个测试 binary 共享一份 (Router, AppState)。
/// 第一次调用：DROP+CREATE 测试库 → 跑迁移 → seed admin。
/// 后续调用：直接 clone 共享实例（注意：tests 必须 serial 跑，否则会互踩共享状态）。
static APP: OnceCell<(axum::Router, AppState)> = OnceCell::const_new();

pub async fn build_app() -> (axum::Router, AppState) {
    let (app, state) = APP.get_or_init(init_once).await;
    (app.clone(), state.clone())
}

async fn init_once() -> (axum::Router, AppState) {
    // 1) 连 dev PG 的 default `postgres` DB，drop+create 测试库
    let admin = Database::connect(ADMIN_DB_URL).await.expect(
        "connect dev postgres failed; please run `bash scripts/dev-up.sh --skip-dod` first",
    );
    admin
        .execute_unprepared(&format!("DROP DATABASE IF EXISTS {TEST_DB_NAME}"))
        .await
        .expect("drop test db");
    admin
        .execute_unprepared(&format!("CREATE DATABASE {TEST_DB_NAME}"))
        .await
        .expect("create test db");
    drop(admin);

    let cfg = Config {
        bind_addr: "127.0.0.1:0".into(),
        database: DatabaseCfg {
            url: format!("postgres://postgres:postgres@127.0.0.1:5432/{TEST_DB_NAME}"),
            pool_min: 1,
            pool_max: 4,
        },
        redis: RedisCfg {
            // dev Redis DB 15 — 与 dev DB 0 完全隔离
            url: "redis://127.0.0.1:6379/15".into(),
            pool_size: 2,
        },
        s3: S3Cfg {
            endpoint: "http://localhost:9000".into(),
            public_base_url: None,
            region: "us-east-1".into(),
            bucket: "gathered-light-test".into(),
            access_key: "minioadmin".into(),
            secret_key: "minioadmin".into(),
            upload_max_bytes: 1024 * 1024,
        },
        jwt: JwtCfg {
            secret: "test-secret".into(),
            access_ttl_secs: 60,
            refresh_ttl_secs: 600,
            private_key_path: None,
            public_key_path: None,
        },
    };

    let state = AppState::init(cfg).await.expect("init AppState");
    Migrator::up(&state.db, None).await.expect("run migrations");

    // Redis DB 15 不主动清理：当前测试用 UUID jti 天然不冲突。
    // 若以后加 logout-all 测试，需要手动 DEL `auth:user-rev:*` 键，
    // 或用 `state.redis.next().flushdb(false).await` 拿单个 client 跑。

    // 3) seed 一个 admin
    if user_repo::find_by_email(&state.db, "admin@test.dev")
        .await
        .unwrap()
        .is_none()
    {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(b"Passw0rd!", &salt)
            .unwrap()
            .to_string();
        user_repo::insert(&state.db, "admin@test.dev".into(), hash, "owner".into())
            .await
            .unwrap();
    }

    let app = routes::build(state.clone());
    (app, state)
}

pub fn json_body<T: serde::Serialize>(v: &T) -> axum::body::Body {
    axum::body::Body::from(serde_json::to_vec(v).unwrap())
}

pub async fn read_json(resp: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap_or(serde_json::json!({
        "_raw": String::from_utf8_lossy(&body).into_owned()
    }))
}
