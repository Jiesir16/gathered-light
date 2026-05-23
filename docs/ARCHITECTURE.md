# 拾光集 · Gathered Light — Headless CMS 架构设计稿

> 作者视角：写给一位资深 Java/Spring Cloud 后端、正在深入实战 Rust 的全栈架构师。
>
> **对应前端契约**：`design/` 目录下的 React 原型（index.html 前台 + admin.html 后台）。后端的所有数据形状、API 路径、可见性枚举值都以这份原型为准，不另立标准。
>
> **后端栈**：Rust 2024 · Axum 0.7 · SeaORM · **PostgreSQL 18** · Redis 7（fred 客户端）· S3 兼容 OSS（aws-sdk-s3）· `image` crate 处理图片 · `tracing` 链路 · 可选 Meilisearch。
>
> **前端栈**：本架构稿默认采用 **React 18 + Zustand + TanStack Query + Axios**（与原型 `design/` 同源，无迁移成本）。如需 Vue3，文末附 Pinia 等价方案。

---

## 0. 心智映射：Rust 后端 ↔ Java Spring 生态

读这份稿子之前先在脑里建立一张对应表，可以让 80% 的 Rust 概念秒懂：

| 关注点 | Spring Cloud Alibaba 世界 | 本项目 Rust 世界 |
| --- | --- | --- |
| Web 框架 | Spring MVC + Tomcat（阻塞）/ WebFlux（响应式） | **Axum** on `tokio`（全异步、零反射，编译期确定路由） |
| 路由声明 | `@RestController` + `@GetMapping` | `Router::new().route("/api/v1/photos", get(handler))` |
| 路径参数 / 查询参数 | `@PathVariable` / `@RequestParam` | **Extractor**：`Path<T>` / `Query<T>` / `Json<T>` |
| 依赖注入 | `@Autowired` 由 IoC 容器注入 | **AppState**：`Arc<AppState>` 显式传入，无反射、零魔法 |
| 拦截器 / Filter | `HandlerInterceptor` / `OncePerRequestFilter` | **Tower Middleware**：`tower::Layer` + `axum::middleware::from_fn` |
| ORM | MyBatis-Plus（动态 SQL + Wrapper） | **SeaORM**：`Entity::find().filter(Column::Cat.eq("street"))` |
| 事务 | `@Transactional` | `db.begin().await?` → `txn.commit().await?`（显式作用域） |
| 缓存抽象 | Spring Cache + RedisTemplate | **fred** crate 直接异步调用 + 自封装 `CacheKey` |
| 配置 | `application.yml` + `@ConfigurationProperties` | `figment` / `config` crate + serde 反序列化到 struct |
| 日志框架 | SLF4J + Logback + MDC | **tracing** + `tracing-subscriber` + `tracing::span!` |
| 链路追踪 | Skywalking / SkyAPM-Sleuth | tracing-opentelemetry → Jaeger / OTLP（trace_id 自动 W3C 透传） |
| 校验 | `@Valid` + Hibernate Validator | `validator` crate（派生宏） + 在 extractor 中校验 |
| 异步任务 | `@Async` / Spring Task | **`tokio::spawn`** + 进程内队列；强一致用 `pgmq` / Redis Stream |
| OpenAPI | springdoc-openapi | `utoipa` + `utoipa-swagger-ui` |
| Bean 作用域 | singleton / prototype | 几乎全是 `Arc<T>` 持有的 singleton；prototype = 函数局部 |

> 一句话总结：Spring 用反射 + 容器在运行期 magic 起来；Rust 让你在编译期把每根线都接对，类型系统替你做了 IoC 校验。

---

## 0.5 接力须知 / Handoff Manifest（Codex / 任何 AI 接力代理 必读）

> 这一节是写给 **Codex / Claude Code / 任意自动化代理** 的进门指南。**不读完这一节就开始写代码 = 不合格交付**。
>
> 本文档（`docs/ARCHITECTURE.md`）是项目的 **唯一设计真理来源（SSOT）**。`AGENTS.md` 与 `CLAUDE.md` 只承载身份/角色信息；当三者冲突，**一律以本文档为准**。

### 你接手时必须做的三件事

1. **先读完 §0.5 → §13 → §14**（这条路径告诉你"硬约束 / 现在长什么样 / 该写什么"），再读其他章节作为参考。
2. **不要重新设计** — §2 数据模型、§3 API 形状、§9 字段对照、§15 缓存失效矩阵 都已定稿。直接照抄。
3. **不要走"低阻力路径"** — 见 [§12 硬约束](#12-硬约束hard-constraints--不达标即不合格) + [§20 禁止模式](#20-禁止模式forbidden-patterns)。任何用 `HashMap` 替代 Redis、用 `Vec<T>` 替代 PG、用字符串替代 S3 签名的实现都视为**回滚级别**返工。

### 你提交的每个 PR 必须在描述里回答这五个问题

> 把这五点当 PR 模板复制进去：

```text
## DoD checklist
- [ ] 命中的硬约束编号：H-?, H-?
- [ ] 引入的禁止模式（应为空）：F-? — 若非空，请论证为何不可避免
- [ ] DoD 验收命令贴出 + 实际输出（见 §14 各 PR 末尾）
- [ ] 前端契约（§9）是否破坏：是 / 否（若是，需同步改 frontend/）
- [ ] 是否新增 tracing span / 业务字段：是 / 否
```

### 你的工作面板

- **当前状态**：见 [§13 当前仓库状态](#13-当前仓库状态诚实评估)。M0 已交付；Codex 此前提交了一份 in-memory 原型 + React 前端，**多数能力违反硬约束**，必须按 §14 替换。
- **下一步任务**：[§14 M1 任务规格](#14-m1-任务规格--auth--db--redis-接入pr-by-pr) 的 PR-1 → PR-5。
- **本地基础设施**：[§11 Quickstart](#11-quickstart本地开发环境) 的 docker-compose 必须先 `up`，再写代码。
- **常见踩坑**：[§22 接力速查](#22-接力速查给-codex--后续-ai-代理的常见踩坑指南)。

### 沟通约定

- 你不知道的设计决策 → 在 PR 描述里 `## Open questions` 列出，**不要自行揣测**（默认走最低阻力路径就是这样栽的）。
- §21 「冻结决策」中的条款**不接受讨论**——已被业务/前端/合规反复确认。

---

## 1. 项目骨架（Directory Structure）

采用 **"按层切（layered）+ 按域分模块（feature）"** 的混合方式。和 Spring Boot 单模块项目目录直觉非常接近。

```
gathered-light/
├── Cargo.toml                       # workspace 根
├── crates/
│   ├── cms-api/                     # ★ 主程序：HTTP 服务（Axum）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs              # bin 入口：tracing init → AppState → router → serve
│   │       ├── bootstrap.rs         # AppState 装配（DB / Redis / S3 / Meili 客户端）
│   │       ├── config.rs            # 配置反序列化（对应 application.yml）
│   │       ├── error.rs             # 全局 AppError + IntoResponse（≈ ControllerAdvice）
│   │       ├── routes.rs            # Router 总装（≈ 总 Controller 路由表）
│   │       │
│   │       ├── middleware/          # Tower 中间件（≈ Interceptor / Filter）
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs          # JWT 解析 + Redis 黑名单校验
│   │       │   ├── trace.rs         # request_id / trace_id 注入 MDC
│   │       │   └── error_handler.rs # panic / 5xx 兜底
│   │       │
│   │       ├── handlers/            # ★ ≈ Controller 层：参数提取 + 状态码 + 序列化
│   │       │   ├── mod.rs
│   │       │   ├── auth_handler.rs
│   │       │   ├── photo_handler.rs # 前台公开 + 后台管理
│   │       │   ├── media_handler.rs # presigned URL / 直传回调
│   │       │   ├── category_handler.rs
│   │       │   ├── tag_handler.rs
│   │       │   └── settings_handler.rs # 前台 Tweaks 默认值（可选）
│   │       │
│   │       ├── services/            # ★ ≈ Service 层：编排 + 事务 + 缓存策略
│   │       │   ├── mod.rs
│   │       │   ├── auth_service.rs
│   │       │   ├── photo_service.rs
│   │       │   ├── media_service.rs # 签名 / 异步处理 dispatch
│   │       │   ├── category_service.rs
│   │       │   └── i18n.rs          # 多语字段（zh/en）打包/解包
│   │       │
│   │       ├── repositories/        # ★ ≈ Mapper / Repository 层：纯 SeaORM
│   │       │   ├── mod.rs
│   │       │   ├── photo_repo.rs
│   │       │   ├── user_repo.rs
│   │       │   └── ...
│   │       │
│   │       ├── workers/             # ≈ @Async 异步任务消费者
│   │       │   ├── mod.rs
│   │       │   └── image_processor.rs # 监听队列：生成 thumb/webp、提 EXIF
│   │       │
│   │       ├── infra/               # 基础设施客户端
│   │       │   ├── mod.rs
│   │       │   ├── redis.rs         # fred client wrapper + cache_aside!
│   │       │   ├── s3.rs            # presigned PUT / GET 封装
│   │       │   └── search.rs        # Meilisearch 客户端
│   │       │
│   │       └── dto/                 # 入参 / 返回 schema（≈ VO/DTO，开启 utoipa 注解）
│   │           ├── mod.rs
│   │           ├── photo_dto.rs
│   │           └── ...
│   │
│   ├── cms-domain/                  # 领域 crate：纯类型 + Trait，无任何运行时
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── photo.rs             # PhotoId / Privacy / Locale 等 newtype
│   │       └── error.rs             # DomainError
│   │
│   └── cms-entity/                  # SeaORM 自动生成的 Entity（独立 crate，避免循环）
│       └── src/
│           ├── lib.rs
│           ├── prelude.rs
│           ├── photo.rs
│           ├── photo_translation.rs
│           ├── media_asset.rs
│           ├── tag.rs
│           ├── photo_tag.rs
│           ├── category.rs
│           └── user.rs
│
├── migrations/                      # SeaORM CLI 生成的 schema migration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── m20260506_000001_init.rs
│       └── ...
│
├── docs/
│   ├── ARCHITECTURE.md              # ★ 本文
│   ├── api.openapi.json             # `cargo run --bin cms-api -- export-openapi` 产出
│   └── erd.mmd                      # mermaid ER 图
│
├── design/                          # 现有原型（前端契约源头）
└── frontend/                        # （后续创建）React 18 + Vite 实现
    └── ...（详见 §6）
```

**为什么 workspace 拆 3 个 crate？**

- `cms-entity` 由 `sea-orm-cli generate entity` 直出，会被高频重写——独立成 crate 防止覆盖到手写代码。
- `cms-domain` 不依赖 SeaORM、Axum、tokio，可在测试 / CLI 工具中复用，类似 Spring 里的 `xxx-api` 接口模块。
- `cms-api` 是真正的可执行 bin，依赖前两者。**编译增量友好**：改 handler 不会触发 entity 重新生成。

---

## 2. 核心数据模型与缓存规范

### 2.1 ER 图（mermaid，可粘进 docs/erd.mmd）

```mermaid
erDiagram
    USERS ||--o{ PHOTOS : "uploaded_by"
    PHOTOS ||--|{ PHOTO_TRANSLATIONS : "i18n"
    PHOTOS }o--|| CATEGORIES : "category_id"
    PHOTOS ||--o{ PHOTO_TAGS : ""
    TAGS ||--o{ PHOTO_TAGS : ""
    PHOTOS ||--|| MEDIA_ASSETS : "primary_asset_id"
    MEDIA_ASSETS }|--|| MEDIA_VARIANTS : "variants"

    USERS {
        bigint id PK
        text email UK
        text password_hash
        text role  "owner|editor|viewer"
        timestamptz created_at
    }
    PHOTOS {
        bigint id PK
        text slug UK
        bigint category_id FK
        bigint primary_asset_id FK
        text privacy           "public|locked|private"
        text passcode_hash     "argon2, only when privacy=locked"
        text taken_at_label    "2024.05 — 不强约束格式"
        date  taken_at_date    "用于排序/查询"
        bigint uploaded_by FK
        bigint sort_order
        timestamptz published_at
        timestamptz created_at
        timestamptz updated_at
    }
    PHOTO_TRANSLATIONS {
        bigint id PK
        bigint photo_id FK
        text locale            "zh|en|..."
        text title
        text location
        text caption
        text alt_text
    }
    CATEGORIES {
        bigint id PK
        text slug UK            "street|landscape|life"
        jsonb name_i18n         "{zh, en}"
        int  sort_order
    }
    TAGS { bigint id PK; text slug UK; jsonb name_i18n }
    PHOTO_TAGS { bigint photo_id FK; bigint tag_id FK }
    MEDIA_ASSETS {
        bigint id PK
        text storage_key UK     "OSS object key"
        text mime_type
        int width
        int height
        bigint byte_size
        text checksum_sha256
        jsonb exif              "提取出的关键 EXIF（光圈/快门/ISO/相机）"
        text status             "pending|processing|ready|failed"
        timestamptz created_at
    }
    MEDIA_VARIANTS {
        bigint id PK
        bigint asset_id FK
        text   variant          "thumb_400|medium_900|full_1800|webp_900"
        text   storage_key
        int    width
        int    height
    }
```

### 2.2 PostgreSQL DDL（迁移 m20260506_000001_init）

> 关键设计点：把 i18n 字段拆到 **PHOTO_TRANSLATIONS** 子表，避免 `jsonb` 检索带来的索引膨胀；同时方便后续接入更多语言。

```sql
-- 用户
CREATE TABLE users (
    id              BIGSERIAL PRIMARY KEY,
    email           CITEXT UNIQUE NOT NULL,
    password_hash   TEXT   NOT NULL,
    display_name    TEXT,
    role            TEXT   NOT NULL DEFAULT 'editor'
                    CHECK (role IN ('owner','editor','viewer')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 分类
CREATE TABLE categories (
    id          BIGSERIAL PRIMARY KEY,
    slug        TEXT UNIQUE NOT NULL,
    name_i18n   JSONB NOT NULL,        -- {"zh":"街拍","en":"Street"}
    sort_order  INT  NOT NULL DEFAULT 0
);

-- 媒资（OSS 物理对象）
CREATE TABLE media_assets (
    id              BIGSERIAL PRIMARY KEY,
    storage_key     TEXT UNIQUE NOT NULL,
    mime_type       TEXT NOT NULL,
    width           INT,
    height          INT,
    byte_size       BIGINT,
    checksum_sha256 TEXT,
    exif            JSONB NOT NULL DEFAULT '{}'::jsonb,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','processing','ready','failed')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_media_assets_status ON media_assets(status);

CREATE TABLE media_variants (
    id          BIGSERIAL PRIMARY KEY,
    asset_id    BIGINT NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    variant     TEXT NOT NULL,        -- thumb_400 / medium_900 / full_1800 / webp_900
    storage_key TEXT NOT NULL,
    width       INT NOT NULL,
    height      INT NOT NULL,
    UNIQUE(asset_id, variant)
);

-- 帖子（前端的 photo 卡）
CREATE TABLE photos (
    id                 BIGSERIAL PRIMARY KEY,
    slug               TEXT UNIQUE NOT NULL,
    category_id        BIGINT REFERENCES categories(id),
    primary_asset_id   BIGINT NOT NULL REFERENCES media_assets(id),
    privacy            TEXT NOT NULL DEFAULT 'public'
                       CHECK (privacy IN ('public','locked','private')),
    passcode_hash      TEXT,           -- argon2(passcode)，仅 locked 用
    taken_at_label     TEXT NOT NULL DEFAULT '',
    taken_at_date      DATE,
    uploaded_by        BIGINT REFERENCES users(id),
    sort_order         BIGINT NOT NULL DEFAULT 0,
    published_at       TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_photos_listing
    ON photos(privacy, category_id, sort_order DESC, taken_at_date DESC);
CREATE INDEX idx_photos_published_desc
    ON photos(published_at DESC) WHERE privacy = 'public';

-- i18n 子表
CREATE TABLE photo_translations (
    id          BIGSERIAL PRIMARY KEY,
    photo_id    BIGINT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    locale      TEXT   NOT NULL,
    title       TEXT   NOT NULL DEFAULT '',
    location    TEXT   NOT NULL DEFAULT '',
    caption     TEXT,
    alt_text    TEXT,
    UNIQUE(photo_id, locale)
);
CREATE INDEX idx_pt_locale ON photo_translations(locale);

-- 标签 + 多对多
CREATE TABLE tags (
    id         BIGSERIAL PRIMARY KEY,
    slug       TEXT UNIQUE NOT NULL,
    name_i18n  JSONB NOT NULL
);
CREATE TABLE photo_tags (
    photo_id   BIGINT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    tag_id     BIGINT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (photo_id, tag_id)
);
```

### 2.3 SeaORM Entity（`cms-entity` crate · 摘录核心两张表）

> Entity 是 `sea-orm-cli generate entity -u $DATABASE_URL -o crates/cms-entity/src` 直出的，下面只给可读注释版便于理解。

```rust
// crates/cms-entity/src/photo.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "photos")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub slug: String,
    pub category_id: Option<i64>,
    pub primary_asset_id: i64,
    pub privacy: String,           // 通过 Privacy newtype 在 service 层强类型化
    pub passcode_hash: Option<String>,
    pub taken_at_label: String,
    pub taken_at_date: Option<Date>,
    pub uploaded_by: Option<i64>,
    pub sort_order: i64,
    pub published_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::category::Entity",
              from = "Column::CategoryId", to = "super::category::Column::Id")]
    Category,
    #[sea_orm(belongs_to = "super::media_asset::Entity",
              from = "Column::PrimaryAssetId", to = "super::media_asset::Column::Id")]
    PrimaryAsset,
    #[sea_orm(has_many = "super::photo_translation::Entity")]
    Translations,
    #[sea_orm(has_many = "super::photo_tag::Entity")]
    PhotoTags,
}

impl ActiveModelBehavior for ActiveModel {}
```

```rust
// 在 cms-domain 里给 privacy 一个强类型壳——避免在 service 里满天飞 magic string
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Privacy { Public, Locked, Private }
```

### 2.4 Redis Key 设计规范（≈ Spring Cache 的 key 命名公约）

> 约定：**`{namespace}:{scope}:{entity}:{discriminator}[:{slot}]`**，全部小写、冒号分隔，TTL 写注释。"写优先（写后失效）"为主，复杂列表才走"双删 + 短 TTL 兜底"。

| 类别 | Key 模板 | TTL | 用途 |
| --- | --- | --- | --- |
| 前台首页瀑布流（按分类） | `cms:photo:list:cat={cat}:lang={lang}:p={page}:ps={size}` | 60s | 命中率最高的前台聚合数据；写操作触发 `DEL cms:photo:list:*` |
| 单帧详情 | `cms:photo:detail:{slug}:lang={lang}` | 300s | 命中后直接返回 DTO；photo 任何字段更新 → 失效 |
| 分类树 | `cms:taxonomy:categories:lang={lang}` | 1h | 几乎不变；通过 admin 写时显式刷新 |
| 标签云 | `cms:taxonomy:tags:lang={lang}` | 1h | 同上 |
| OSS 直传节流 | `cms:upload:rl:user={uid}` | 60s（INCR） | 防止脚本刷签 |
| 锁解口令尝试限流 | `cms:photo:unlock:rl:{photo_id}:{ip_hash}` | 600s | INCR，>10 拒服务 |
| Tweaks 默认值（可选） | `cms:settings:tweaks:default` | 1d | 给前台 SSR / 首屏拿初始 UI 风格 |
| Search miss 防穿透 | `cms:photo:list:404:cat={cat}:p={page}` | 30s | 占位 `__NIL__` |
| **认证** | | | |
| JWT 黑名单（精确单 token） | `auth:blacklist:{jti}` | = 原 token 剩余 TTL | **被踢下线 / 主动登出**，存 `1` 即可 |
| 全用户失活水位（踢全设备） | `auth:user-rev:{user_id}` | 30d（业务 TTL） | 存最新 epoch；JWT 中 `iat` < epoch 即失效（无需逐 token 拉黑） |
| Refresh token | `auth:refresh:{jti}` | 7d | 存关联 user_id 与设备指纹 |
| 登录失败计数 | `auth:login:fail:{email_hash}` | 15min | INCR，>5 锁 5 分钟 |

**双删模式**（写后立即删 + 异步延迟删）伪代码：

```text
fn invalidate_photo_list():
    redis.del("cms:photo:list:*")           # 立即一次（用 SCAN+UNLINK 防卡）
    spawn delay(500ms) → redis.del(...)    # 防止"读回填了脏数据"的尾巴
```

---

## 3. 核心 API 表（基于前端原型反推）

> 全量带版本前缀 `/api/v1`。返回包统一 `{ code, message, data }`，错误见 §5。

### 3.1 公开（前台）

| Method | Path | 说明 | 缓存 |
| --- | --- | --- | --- |
| GET | `/api/v1/photos` | 列表，参数 `?cat=street&lang=zh&page=1&page_size=24` | ★ |
| GET | `/api/v1/photos/:slug` | 单帧详情；private 返回 403、locked 返回脱敏 | ★ |
| POST | `/api/v1/photos/:slug/unlock` | 锁解口令校验，返回临时签名 token | — |
| GET | `/api/v1/categories` | 分类树（含计数） | ★ |
| GET | `/api/v1/tags` | 标签云 | ★ |
| GET | `/api/v1/settings/tweaks` | 前台 Tweaks 默认值（layout/density/...） | ★ |

### 3.2 认证

| Method | Path | 说明 |
| --- | --- | --- |
| POST | `/api/v1/auth/login` | 出 access(15m) + refresh(7d) |
| POST | `/api/v1/auth/refresh` | refresh 换新 access |
| POST | `/api/v1/auth/logout` | 把 access.jti 写入黑名单，删 refresh |
| POST | `/api/v1/auth/logout-all` | 自增 `auth:user-rev:{uid}` 踢全设备 |

### 3.3 管理（需 `Authorization: Bearer ...`）

| Method | Path | 说明 |
| --- | --- | --- |
| GET | `/api/v1/admin/photos` | 列表（含 private），支持过滤 `privacy=` `cat=` `q=` |
| POST | `/api/v1/admin/photos` | 提交元数据（绑定已上传的 asset） |
| PATCH | `/api/v1/admin/photos/:id` | 改标题 / 隐私 / 分类 / 排序 |
| DELETE | `/api/v1/admin/photos/:id` | 软删（`deleted_at`，未在 DDL 体现可加） |
| POST | `/api/v1/admin/photos/:id/cycle-privacy` | 对应 admin.jsx 里点圆点循环 public→locked→private |
| POST | `/api/v1/admin/media/presign` | ★ 出 S3 PUT presigned URL |
| POST | `/api/v1/admin/media/complete` | 客户端直传完毕回调，触发异步处理 |
| GET | `/api/v1/admin/stats` | 后台首页 Stats 卡片所需统计 |
| CRUD | `/api/v1/admin/categories`, `/api/v1/admin/tags` | 略 |

---

## 4. 核心业务流程代码

### 4.1 启动装配（`main.rs` + `bootstrap.rs`）

```rust
// crates/cms-api/src/bootstrap.rs
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db:     sea_orm::DatabaseConnection,
    pub redis:  fred::clients::RedisPool,
    pub s3:     aws_sdk_s3::Client,
    pub config: Arc<Config>,
    pub jwt:    Arc<JwtKeys>,
}

impl AppState {
    pub async fn init(cfg: Config) -> anyhow::Result<Self> {
        let db = sea_orm::Database::connect({
            let mut o = sea_orm::ConnectOptions::new(&cfg.database_url);
            o.max_connections(cfg.db_pool_max)
             .min_connections(cfg.db_pool_min)
             .acquire_timeout(std::time::Duration::from_secs(5))
             .sqlx_logging_level(tracing::log::LevelFilter::Debug);
            o
        }).await?;

        let redis = fred::clients::RedisPool::new(
            fred::types::RedisConfig::from_url(&cfg.redis_url)?,
            None, None, None, cfg.redis_pool_size,
        )?;
        redis.connect();
        redis.wait_for_connect().await?;

        let s3 = aws_sdk_s3::Client::new(
            &aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_sdk_s3::config::Region::new(cfg.s3_region.clone()))
                .endpoint_url(&cfg.s3_endpoint)
                .load().await,
        );

        Ok(Self {
            db, redis, s3,
            jwt: Arc::new(JwtKeys::from_pem(&cfg.jwt_private_key, &cfg.jwt_public_key)?),
            config: Arc::new(cfg),
        })
    }
}
```

```rust
// crates/cms-api/src/main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cms_api::observability::init();           // 见 §7
    let cfg = cms_api::config::load()?;       // figment: env > toml > defaults
    let state = cms_api::bootstrap::AppState::init(cfg).await?;

    // 后台 worker（≈ Spring 启动 @Async 线程池）
    tokio::spawn(cms_api::workers::image_processor::run(state.clone()));

    let app = cms_api::routes::build(state.clone());
    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    tracing::info!(addr=%state.config.bind_addr, "🌅 gathered-light is listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal()) // SIGTERM 优雅停机
        .await?;
    Ok(())
}
```

### 4.2 JWT 中间件 + Redis 黑名单（替 Spring 的 OncePerRequestFilter）

```rust
// crates/cms-api/src/middleware/auth.rs
pub async fn jwt_guard(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized("missing bearer"))?;

    // 1) 验签 + 解析 claims
    let claims = state.jwt.decode(token)?;            // ≈ jjwt parse

    // 2) 单 token 黑名单
    let blk = format!("auth:blacklist:{}", claims.jti);
    let hit: bool = state.redis.exists(&blk).await?;
    if hit { return Err(AppError::Unauthorized("token revoked")); }

    // 3) 全用户水位线（踢全设备）
    let rev_key = format!("auth:user-rev:{}", claims.sub);
    if let Some(epoch) = state.redis.get::<Option<i64>, _>(&rev_key).await? {
        if claims.iat < epoch {
            return Err(AppError::Unauthorized("session terminated"));
        }
    }

    req.extensions_mut().insert(CurrentUser::from(claims));
    Ok(next.run(req).await)
}
```

> **登出**：把 `claims.jti` 写入 `auth:blacklist:{jti}`，TTL 设为 `claims.exp - now`。
> **踢全设备**：`SET auth:user-rev:{uid} <now_epoch>`，老 token 因 `iat < epoch` 全数无效——无需扫描。

### 4.3 OSS 直传链路（前端直传，后端只签名）

> 这是 OSS 最佳实践的核心：**绝不让 Rust 进程吃 100 MB 上传流量**，签 URL → 浏览器 → OSS → 回调元数据 → 异步处理。

```rust
// crates/cms-api/src/handlers/media_handler.rs
#[derive(Deserialize, ToSchema)]
pub struct PresignReq {
    pub filename: String,
    pub mime_type: String,                // image/jpeg
    pub byte_size: u64,
}

#[derive(Serialize, ToSchema)]
pub struct PresignResp {
    pub upload_url: String,               // PUT 这个 URL
    pub storage_key: String,              // 后续用它再 POST /complete
    pub headers: serde_json::Value,       // 浏览器需要回带的 header
    pub expires_in: u64,                  // 秒
}

pub async fn presign(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<PresignReq>,
) -> Result<Json<PresignResp>, AppError> {
    // 简单限频
    let rl_key = format!("cms:upload:rl:user={}", user.id);
    let n: i64 = state.redis.incr(&rl_key).await?;
    if n == 1 { let _: () = state.redis.expire(&rl_key, 60).await?; }
    if n > 30 { return Err(AppError::TooManyRequests("upload rate")); }

    // mime / size 白名单
    if !ALLOWED_IMAGE_MIME.contains(req.mime_type.as_str()) {
        return Err(AppError::BadRequest("mime not allowed"));
    }
    if req.byte_size > 25 * 1024 * 1024 {
        return Err(AppError::BadRequest("file too large"));
    }

    let key = format!("photos/{}/{}-{}",
        chrono::Utc::now().format("%Y/%m"),
        uuid::Uuid::new_v4(),
        sanitize(&req.filename));

    let presigning = aws_sdk_s3::presigning::PresigningConfig::expires_in(
        std::time::Duration::from_secs(900))?;

    let req_built = state.s3.put_object()
        .bucket(&state.config.s3_bucket)
        .key(&key)
        .content_type(&req.mime_type)
        .content_length(req.byte_size as i64)
        .presigned(presigning).await?;

    Ok(Json(PresignResp {
        upload_url: req_built.uri().to_string(),
        storage_key: key,
        headers: serde_json::json!({
            "Content-Type": req.mime_type,
            "Content-Length": req.byte_size,
        }),
        expires_in: 900,
    }))
}
```

直传完成后前端调用 `/admin/media/complete`，service 层落库 `media_assets(status='pending')` 并丢进异步处理队列：

```rust
// crates/cms-api/src/services/media_service.rs
pub async fn complete_upload(
    state: &AppState,
    user_id: i64,
    storage_key: String,
) -> Result<MediaAsset, AppError> {
    // 头校验（HEAD 对象）—— 只信 OSS，不信前端报的尺寸
    let head = state.s3.head_object()
        .bucket(&state.config.s3_bucket).key(&storage_key)
        .send().await?;

    let asset = media_repo::insert_pending(&state.db, NewAsset {
        storage_key: storage_key.clone(),
        mime_type: head.content_type().unwrap_or("application/octet-stream").into(),
        byte_size: head.content_length().unwrap_or(0),
        ..Default::default()
    }).await?;

    // 投递处理任务（轻量场景：进程内 channel；高一致：pgmq / Redis Stream XADD）
    state.image_queue.send(ImageJob { asset_id: asset.id }).await
        .map_err(|_| AppError::Internal("queue closed"))?;
    Ok(asset)
}
```

worker 端用 `image` crate 生成 webp 缩略图、提取 EXIF，写 `media_variants` 与 `media_assets.status = 'ready'`。

### 4.4 经典 Cache-Aside（前台列表 Service）

```rust
// crates/cms-api/src/services/photo_service.rs
pub async fn list_public(
    state: &AppState,
    q: ListQuery,
) -> Result<PageResp<PhotoCardDto>, AppError> {
    // 1) 先查 Redis（≈ Spring @Cacheable）
    let key = format!(
        "cms:photo:list:cat={}:lang={}:p={}:ps={}",
        q.cat.as_deref().unwrap_or("all"),
        q.lang.as_str(), q.page, q.page_size,
    );
    if let Some(raw) = state.redis.get::<Option<String>, _>(&key).await? {
        if raw == "__NIL__" {                                      // 防穿透占位
            return Ok(PageResp::empty(q.page, q.page_size));
        }
        if let Ok(hit) = serde_json::from_str::<PageResp<_>>(&raw) {
            tracing::debug!(cache="hit", %key);
            return Ok(hit);
        }
    }
    tracing::debug!(cache="miss", %key);

    // 2) 回源 PG（≈ Mapper）
    let result = photo_repo::list_public_paged(&state.db, &q).await?;

    // 3) 回写 Redis
    let payload = if result.items.is_empty() {
        let _: () = state.redis.set(&key, "__NIL__", Some(Expiration::EX(30)), None, false).await?;
        result
    } else {
        let json = serde_json::to_string(&result)?;
        let _: () = state.redis.set(&key, &json, Some(Expiration::EX(60)), None, false).await?;
        result
    };
    Ok(payload)
}

// 写路径：admin 创建/编辑/删除
pub async fn invalidate_listing_caches(redis: &fred::clients::RedisPool) -> anyhow::Result<()> {
    // 用 SCAN + UNLINK 防止阻塞 Redis 主线程
    let mut scan = redis.scan("cms:photo:list:*", Some(500), None);
    while let Some(page) = scan.next().await {
        let keys = page?.results().clone();
        if !keys.is_empty() {
            let _: () = redis.unlink(keys).await?;
        }
    }
    Ok(())
}
```

> 对应 Spring：`@Cacheable(cacheNames="photoList", key="...")` + `@CacheEvict(allEntries=true)`，不过这里的失效粒度**手动**控制更精准。

### 4.5 Locked 解锁的"短期临时凭证"

`POST /photos/:slug/unlock` 校验 argon2 后，签发一个 **15 min、scope 仅限本帧 full 资源** 的短 JWT 给前端，前端把它放到 `Authorization` 调 `/photos/:slug/full` 才能拿到 1800px 原图签名 URL。这样比直接把 full URL 返回更安全。

---

## 5. 全局错误模型（≈ `@RestControllerAdvice`）

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("bad request: {0}")]   BadRequest(&'static str),
    #[error("unauthorized: {0}")]  Unauthorized(&'static str),
    #[error("forbidden")]          Forbidden,
    #[error("not found")]          NotFound,
    #[error("conflict: {0}")]      Conflict(&'static str),
    #[error("rate limited: {0}")]  TooManyRequests(&'static str),
    #[error("validation: {0}")]    Validation(String),
    #[error("internal: {0}")]      Internal(&'static str),
    #[error(transparent)]          Db(#[from] sea_orm::DbErr),
    #[error(transparent)]          Redis(#[from] fred::error::RedisError),
    #[error(transparent)]          Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::BadRequest(_) | AppError::Validation(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Unauthorized(_)                          => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden                                => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::NotFound                                 => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Conflict(_)                              => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::TooManyRequests(_)                       => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
            AppError::Db(_) | AppError::Redis(_) | AppError::Internal(_) | AppError::Other(_)
                                                               => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
        };
        // 重要：5xx 不把内部细节抛给客户端，但**结构化日志要带全栈**
        if status.is_server_error() {
            tracing::error!(error=?self, "server error");
        }
        (status, Json(json!({ "code": code, "message": self.to_string() }))).into_response()
    }
}
```

---

## 6. 前端集成（React 18 + Zustand + TanStack Query）

> 与 `design/` 里的 React 原型同源。原型用 `localStorage` 模拟"后端"，迁到真实 API 时只需要把 `Store.load()` 替换为 `api.photos.list()`，组件不动。

### 6.1 目录建议

```
frontend/
├── src/
│   ├── api/
│   │   ├── client.ts          # axios 实例 + 拦截器（无感刷新）
│   │   ├── auth.ts
│   │   ├── photos.ts
│   │   └── media.ts
│   ├── stores/
│   │   ├── authStore.ts       # Zustand：accessToken / user
│   │   └── uiStore.ts         # 语言、Tweaks（沿用原型 localStorage 思路）
│   ├── hooks/
│   │   ├── usePhotos.ts       # TanStack Query 封装（≈ SWR）
│   │   └── useUploader.ts     # presign → PUT → complete
│   ├── pages/
│   │   ├── site/              # 前台（index.html → React Router）
│   │   └── admin/
│   ├── components/            # 复用原型里的 PhotoCard / Lightbox / Tweaks
│   └── main.tsx
```

### 6.2 Axios 客户端 + 无感刷新（核心）

```ts
// src/api/client.ts
import axios, { AxiosError, AxiosRequestConfig } from "axios";
import { useAuth } from "@/stores/authStore";

export const http = axios.create({ baseURL: "/api/v1", timeout: 15_000 });

// 1) 注入 access token
http.interceptors.request.use((cfg) => {
  const t = useAuth.getState().accessToken;
  if (t) cfg.headers.Authorization = `Bearer ${t}`;
  return cfg;
});

// 2) 401 → 单航班刷新（防并发风暴），其它错误统一抛 AppError
let refreshing: Promise<string> | null = null;

http.interceptors.response.use(
  (r) => r,
  async (err: AxiosError<{ code: string; message: string }>) => {
    const original = err.config as AxiosRequestConfig & { _retry?: boolean };
    if (err.response?.status === 401 && !original._retry) {
      original._retry = true;
      try {
        const newToken = await (refreshing ??= refreshOnce());
        return http({ ...original, headers: { ...original.headers, Authorization: `Bearer ${newToken}` } });
      } finally {
        refreshing = null;
      }
    }
    // 全局 toast
    toast.error(err.response?.data?.message ?? err.message);
    return Promise.reject(err);
  },
);

async function refreshOnce(): Promise<string> {
  const { refreshToken, setTokens, clear } = useAuth.getState();
  if (!refreshToken) { clear(); throw new Error("no refresh"); }
  try {
    const r = await axios.post("/api/v1/auth/refresh", { refresh_token: refreshToken });
    setTokens(r.data.data);                 // { access_token, refresh_token }
    return r.data.data.access_token;
  } catch (e) {
    clear(); window.location.href = "/admin/login"; throw e;
  }
}
```

### 6.3 Zustand auth store（≈ Pinia store）

```ts
// src/stores/authStore.ts
import { create } from "zustand";
import { persist } from "zustand/middleware";

type Tokens = { access_token: string; refresh_token: string };

interface AuthState {
  accessToken: string | null;
  refreshToken: string | null;
  user: { id: number; email: string; role: string } | null;
  setTokens: (t: Tokens) => void;
  clear: () => void;
}

export const useAuth = create<AuthState>()(
  persist(
    (set) => ({
      accessToken: null, refreshToken: null, user: null,
      setTokens: ({ access_token, refresh_token }) =>
        set({ accessToken: access_token, refreshToken: refresh_token }),
      clear: () => set({ accessToken: null, refreshToken: null, user: null }),
    }),
    { name: "shiguangji.auth" }, // localStorage
  ),
);
```

### 6.4 OSS 直传 hook

```ts
// src/hooks/useUploader.ts
export async function uploadPhoto(file: File): Promise<{ assetId: number }> {
  const { data: presign } = await http.post("/admin/media/presign", {
    filename: file.name, mime_type: file.type, byte_size: file.size,
  });
  // 浏览器直传 OSS，跟后端无关
  await axios.put(presign.upload_url, file, { headers: presign.headers });
  const { data: asset } = await http.post("/admin/media/complete", {
    storage_key: presign.storage_key,
  });
  return { assetId: asset.id };
}
```

### 6.5 列表用 TanStack Query（自带 stale-while-revalidate，省一层缓存）

```ts
// src/hooks/usePhotos.ts
export const usePhotos = (params: ListParams) => useQuery({
  queryKey: ["photos", params],
  queryFn: () => http.get("/photos", { params }).then((r) => r.data.data),
  staleTime: 60_000,              // 与后端 60s Redis TTL 对齐，避免双重热点
  placeholderData: keepPreviousData,
});
```

### 6.6 Vue3 等价（如果你最后选 Vue）

- `axios + interceptor` 写法完全照搬。
- Zustand → **Pinia**：`defineStore('auth', () => { ... })`。
- TanStack Query → **`@tanstack/vue-query`**（同 API）。
- 路由：vue-router，组件命名沿用原型。

---

## 7. 可观测性初始化（tracing 启动模板）

```rust
// crates/cms-api/src/observability.rs
use tracing_subscriber::{prelude::*, EnvFilter, fmt, registry::Registry};
use tracing_subscriber::fmt::time::ChronoUtc;

pub fn init() {
    // 1) 过滤层（≈ logback.xml 里 logger 级别）：默认 info，env 可覆盖
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sea_orm=debug,sqlx=warn,hyper=warn"));

    // 2) 控制台层：JSON 结构化（生产）or pretty（开发）
    let stdout_layer = if std::env::var("APP_ENV").as_deref() == Ok("production") {
        fmt::layer().with_timer(ChronoUtc::rfc_3339())
            .with_target(true).with_thread_ids(true)
            .json().with_current_span(true).with_span_list(false)
            .boxed()
    } else {
        fmt::layer().with_timer(ChronoUtc::rfc_3339()).pretty().boxed()
    };

    // 3) OpenTelemetry → Jaeger / OTLP（≈ Skywalking trace_id）
    let otel_layer = std::env::var("OTLP_ENDPOINT").ok().map(|endpoint| {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint))
            .with_trace_config(opentelemetry_sdk::trace::config()
                .with_resource(opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", "cms-api"),
                ])))
            .install_batch(opentelemetry_sdk::runtime::Tokio).unwrap();
        tracing_opentelemetry::layer().with_tracer(tracer).boxed()
    });

    let registry = Registry::default().with(filter).with(stdout_layer);
    if let Some(otel) = otel_layer { registry.with(otel).init(); }
    else { registry.init(); }

    tracing::info!("📡 tracing initialized");
}
```

请求级 span（让每条 access log 自带 trace_id / request_id，方便你像 Skywalking 那样在 Kibana 里搜）：

```rust
// 中间件层（Tower）：把 tower-http::trace::TraceLayer 插到最外层
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

let app = Router::new()
    /* ... routes ... */
    .layer(TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO).include_headers(false))
        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO).latency_unit(LatencyUnit::Millis)));
```

> 业务代码里只用 `tracing::info!(photo_id, slug, "publishing")`，跨服务调用时 `reqwest-middleware` + `tracing-opentelemetry::Inject` 自动把 W3C `traceparent` 注入下游，等价 Spring Cloud Sleuth。

---

## 8. 安全 / 合规要点（实际上线前必查清单）

1. **Argon2** 而非 bcrypt：所有口令、`photo.passcode_hash`。`argon2id` 默认参数即可。
2. **JWT** 用 RS256 / EdDSA，密钥从 KMS 注入；access 15m + refresh 7d 旋转。
3. **限流** 三层：Nginx → tower-governor → Redis 业务级（登录失败、unlock、presign）。
4. **OSS Bucket** 私有读 + 全部走签名 URL；CDN 边缘签名（CloudFront/CDN 域名 KMS）。
5. **图片处理 worker** 限定 mime + 解码内存上限（`image::ImageReader::with_guessed_format()` + 尺寸阈值），防 zip-bomb / 解码炸弹。
6. **CSP**：前端只允许加载自家 OSS / CDN 域。
7. **审计日志**：所有 admin 写操作 → 一张 `audit_logs` 表 + 结构化 tracing。

---

## 9. 与 design 原型的字段对照（确保前端零改动可对接）

| design 字段 | 后端响应字段（DTO） | 备注 |
| --- | --- | --- |
| `id` | `id` | bigint 序列 |
| `src` | `cover_url` | 由后端选择合适的 variant + 签名 URL 拼出 |
| `cat` | `category.slug` | 同步返回 `category.name_i18n` |
| `title.zh` / `title.en` | `title.{locale}` | 由 photo_translations join + i18n 折叠 |
| `loc.zh` / `loc.en` | `location.{locale}` | 同上 |
| `date` | `taken_at_label` | 字符串 "2024.05" 原样保留；同时给 `taken_at` ISO |
| `privacy` | `privacy` | "public" / "locked" / "private" 严格一致 |

具体 DTO：

```rust
#[derive(Serialize, ToSchema)]
pub struct PhotoCardDto {
    pub id: i64,
    pub slug: String,
    pub cover_url: String,
    pub category: CategoryRefDto,
    pub title: I18nField,
    pub location: I18nField,
    pub taken_at_label: String,
    pub privacy: Privacy,
}

#[derive(Serialize, ToSchema)]
pub struct I18nField { pub zh: String, pub en: String }
```

前端使用：

```tsx
// 直接替换原型 data.jsx 的 DEFAULT_PHOTOS
const { data: photos = [] } = usePhotos({ cat: category, lang });
return <Gallery photos={photos} ... />;   // 组件代码完全不变
```

---

## 10. 实施路线图

| 阶段 | 目标 | 关键产物 |
| --- | --- | --- |
| **M0：脚手架（1 天）** | Workspace + Axum + 健康检查 + tracing | `/healthz` 返 200，结构化日志可见 |
| **M1：数据 + 鉴权（2-3 天）** | DDL 迁移、SeaORM entity 生成、JWT + Redis 黑名单 | `auth/login` `auth/logout-all` 通跑 |
| **M2：媒资链路（2-3 天）** | presign + complete + image worker | 后台能上传出 thumb/full variants |
| **M3：CMS 核心（3-4 天）** | photo / category / tag CRUD + cache-aside | 把原型 `Store.*` 全替换为真实 API |
| **M4：观测 + 性能（1-2 天）** | OTLP 接入、慢 SQL 报警、瀑布流 P99 | Jaeger 看到全链路 trace |
| **M5：可选** | Meilisearch 同步、定时任务（清理失败上传） | `/photos?q=...` 毫秒级返回 |

---

## 11. Quickstart：本地开发环境

> 接力代理首次进入仓库的零摩擦启动流程。**写任何代码之前必须先按本节跑一遍**，确认基础设施齐备；否则你会和 PG / Redis / S3 客户端的连接错误纠缠半天，并很容易"暂时绕开"——见 §20 F-1。

### 11.1 必备

- macOS 14+ / Linux（kernel ≥ 5.15）
- Rust **1.85+**（已验证 cargo 1.94 / rustc 1.94）
- Docker 24+ + Docker Compose v2
- Node.js **20+** + npm（前端构建）
- `jq`（看 JSON 用，可选）

### 11.2 一键拉起依赖（PG 18 + Redis 7 + MinIO）

本仓的 `docker-compose.yml` 采用 **bind mount 到 `./data/`** 的 dev 偏好（`.gitignore` 已屏蔽 `data/`）：

```yaml
# dev 偏好：所有数据 bind mount 到仓库根 ./data/
# 优点：rm -rf data/<svc> 一秒重置；tar 整个 ./data 即可备份
services:
  postgres:
    image: postgres:18-alpine
    container_name: gathered-pg
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: gathered_light
    ports: ["5432:5432"]
    # ⚠️ PG 18+ 改了挂载约定：必须挂 /var/lib/postgresql（不是 /var/lib/postgresql/data），
    # 让 PG 自己管理 18/main 子目录。挂错会报 "(unused mount/volume)" 直接退出。
    volumes:
      - ./data/postgres:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres -d gathered_light"]
      interval: 3s
      timeout: 3s
      retries: 20

  redis:
    image: redis:7-alpine
    container_name: gathered-redis
    ports: ["6379:6379"]
    command: ["redis-server", "--appendonly", "yes"]
    volumes:
      - ./data/redis:/data

  minio:
    image: minio/minio:latest
    container_name: gathered-minio
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    ports: ["9000:9000", "9001:9001"]
    command: server /data --console-address ":9001"
    volumes:
      - ./data/minio:/data
```

> **替代方案**：如果你想用 Docker 命名卷（数据藏在 `/var/lib/docker/volumes/`），把每个 `volumes` 段改回 `["pg18_data:/var/lib/postgresql"]`、`["redis_data:/data"]`、`["minio_data:/data"]`，并在文件末尾加 `volumes:\n  pg18_data:\n  redis_data:\n  minio_data:`。但**重置数据**就要走 `docker volume rm gathered-light_pg18_data` 而不是 `rm -rf`。

```sh
docker compose up -d
docker compose ps   # 期望：postgres healthy / redis up / minio up
```

> **PG 18 注意点**：迁移 SQL 内已包含 `CREATE EXTENSION IF NOT EXISTS citext;`。PG 18 默认带 contrib，无需额外安装。云上托管 PG（AWS RDS / GCP CloudSQL）若不允许 SUPERUSER 装扩展，需提工单或自托管。

### 11.3 数据库迁移 + Entity 生成

```sh
cp .env.example .env  # 一次性

# 1) 跑迁移：建 8 张表 + 3 条种子分类
cargo run -p migrations -- up

# 2) 安装 sea-orm-cli（仅一次）
cargo install sea-orm-cli --locked

# 3) 反向工程 Entity 到 cms-entity
sea-orm-cli generate entity \
    -u "$APP_DATABASE__URL" \
    -o crates/cms-entity/src \
    --with-serde both \
    --serde-skip-deserializing-primary-key \
    --lib

# 4) 验证 workspace 绿
cargo check --workspace --all-targets
```

### 11.4 启动后端 + 前端

```sh
# 后端（监听 8080，SPA fallback 已挂）
APP_ENV=development RUST_LOG=info cargo run -p cms-api

# 前端（另起一个 shell；vite dev server 监听 5173）
cd frontend && npm install && npm run dev
```

### 11.5 MinIO Bucket 初始化（M2 起需要）

```sh
docker run --rm --network host \
  -e MC_HOST_local=http://minioadmin:minioadmin@localhost:9000 \
  minio/mc mb -p local/gathered-light
docker run --rm --network host \
  -e MC_HOST_local=http://minioadmin:minioadmin@localhost:9000 \
  minio/mc anonymous set download local/gathered-light
```

---

## 12. 硬约束（HARD CONSTRAINTS — 不达标即不合格）

> 每条都对应 CLAUDE.md 中用户写明的核心要求 或 §1–§10 的关键设计决策。**Codex / 后续 AI 代理在 PR 描述中必须能引用以下编号说明命中哪几条**。

### H-1：主数据存储 = PostgreSQL 18

- 所有持久化内容（users / photos / media_assets / media_variants / categories / tags / photo_tags / photo_translations）**必须**走 SeaORM + PG 18。
- **不允许**任何 in-memory `Vec` / `HashMap` / `RwLock<...>` 替代物。
- 进程重启后数据**必须**保留。
- 反例：见 [§20 F-1](#f-1in-memory-原始集合替代-pg)。

### H-2：JWT 主动失效 = Redis 黑名单 + 用户级 epoch

- 单 token 拉黑：`auth:blacklist:{jti}`，TTL 等于原 token 剩余 exp。
- 全设备踢下线：`auth:user-rev:{user_id}`，存当前 epoch（Unix 秒）；中间件比对 `claims.iat < epoch` 即拒。
- **不允许**用 `Arc<RwLock<HashSet<String>>>` 替代——多副本部署立刻穿透。
- CLAUDE.md 用户原话："**必须结合 Redis** 实现 Token 的主动失效"。

### H-3：密码 = Argon2id，永远不入仓

- 用户密码、photo locked 口令**必须** argon2id 哈希后入库，**禁止**明文比较。
- 工厂参数 `Argon2::default()` 即可。
- **任何**密码不得出现在 `config/*.toml` 或源码里——一律走 `.env`、KMS 或 Vault。
- 仓库根放 `.env.example` 占位，**不**放 `.env`（被 `.gitignore` 屏蔽）。

### H-4：S3 直传 = 真签名

- `POST /api/v1/admin/media/presign` **必须**返回真正可 PUT 的 presigned URL（`aws-sdk-s3` v1+ 的 `PresigningConfig`）。
- 不允许 mock 字符串、不允许带 `MOCK-*` 字样的占位。
- 后端**不**接受图片字节流上传——HTTP body 走 S3，不走 Rust 进程。

### H-5：图片处理 = 异步 worker

- `POST /admin/media/complete` 触发 → `tokio::spawn` worker（M2 起）。
- 生成 4 个 variants：`thumb_400` / `medium_900` / `full_1800` / `webp_900`，写入 `media_variants`，更新 `media_assets.status = 'ready'`。
- 不允许在 handler 内同步处理图片。

### H-6：API 路径与字段形状 = 与前端契约一致

- 路径前缀统一 `/api/v1`。
- 命名遵守 [§3 API 表](#3-核心-api-表基于前端原型反推)；前端 `design/data.jsx` 字段（`title.zh / loc.en / privacy / cat / date`）**不能改名**——除非同步改 `frontend/`。

### H-7：分层纪律

- handler **不写 SQL**；service **不直接调 redis-client**（走 `infra::redis` 封装）；repository **不调 service / 不读 Redis**。
- 鉴权**走 Tower middleware 一次挂上**（见 §4.2）；**不在每个 handler 里重复** `require_access(&headers)`。

### H-8：错误模型统一

- 所有 handler 返 `AppResult<T>`；任何 `panic!` / `.unwrap()` / `.expect()` 在生产路径上视为 bug。
- 5xx 不暴露内部细节给客户端；但**必须** `tracing::error!(error = ?e)` 落结构化日志。

### H-9：可观测性带得住业务字段

- 每个 service 函数入口 `#[tracing::instrument(skip(state), fields(user_id = ?u.id))]`。
- 缓存命中/未命中：`tracing::debug!(cache = "hit"|"miss", key = %key)`。

### H-10：前端契约不可破坏

- §9 字段对照表是合同。后端可**新增**返回字段（前端忽略即可），但**不能改名 / 不能删字段 / 不能改类型**。
- 改前先在 PR 描述里给出 `frontend/src/api/client.ts` 同步 diff。

---

## 13. 当前仓库状态（诚实评估）

> 接力者必读。在你写下任何一行代码之前，明确"现在哪里是 mock，哪里是 real"。
>
> 评估基线：2026-05-07，对应 working tree 当前状态（仓库尚未初始化 git）。

### 13.1 已经合规的部分 ✅

| 项 | 实现位置 | 状态 |
| --- | --- | --- |
| Workspace 4 crate | `Cargo.toml`, `crates/*` | ✅ M0 |
| Axum + tracing + figment 启动链路 | `crates/cms-api/src/{main,observability,config}.rs` | ✅ M0 |
| `/healthz` `/readyz` | `handlers/health_handler.rs` | ✅ M0（readyz 仅返回常量，M1 PR-1 改真 ping） |
| 强类型 `Privacy` / `Locale` + 单测 | `crates/cms-domain/src/photo.rs` | ✅ M0 |
| 完整 DDL（8 张表 + 3 条种子分类） | `migrations/src/sql/m20260506_000001_init_up.sql` | ✅ 已写但**未跑过** |
| 分层目录骨架 | `crates/cms-api/src/{handlers,services,dto,infra,middleware,workers,repositories}/` | ✅ M0 |
| AppError + IntoResponse 错误 envelope | `crates/cms-api/src/error.rs` | ✅ M0 |
| React 18 + Vite + TypeScript 前端（前台 + 后台） | `frontend/` | ✅ 已交付（架构稿之外的额外产物） |
| 前端 401 自动刷新 + Bearer 注入 | `frontend/src/api/client.ts` | ✅ |
| SPA 同源服务（axum `ServeDir` fallback） | `crates/cms-api/src/routes.rs` | ✅ |

### 13.2 必须替换的 mock / 占位（违反 §12 硬约束）

| 项 | 当前位置 | 现状 | 违反 | 替换 PR |
| --- | --- | --- | --- | --- |
| photos 存储 | `bootstrap.rs::AppState.photos: Arc<RwLock<Vec<PhotoDto>>>` | 内存数组（重启即丢） | **H-1 / F-1** | M1 PR-2 |
| 应用层缓存 | `bootstrap.rs::AppState.cache: Arc<RwLock<HashMap<...>>>` | 进程内 HashMap | **H-1** | M1 PR-1 |
| Token 黑名单 | `bootstrap.rs::AppState.token_blacklist: Arc<RwLock<HashSet>>` | 进程内 HashSet | **H-2 / F-2** | M1 PR-3 |
| admin 登录 | `auth_service.rs:13` 明文比较；`config/default.toml` 含 `admin_password = "admin"` | 不安全 + 密码入仓 | **H-3 / F-3** | M1 PR-3 |
| S3 签名 | `media_service.rs:28` `MOCK-HMAC-SHA256` 字符串 | 假 URL，无法 PUT | **H-4 / F-4** | M2 PR-1 |
| 图片处理 worker | （不存在） | 缺位 | **H-5** | M2 PR-2 |
| `logout-all` 端点 | （不存在） | 缺位 | **H-2** | M1 PR-3 |
| JWT 中间件 | 每个 handler 重复 `auth_service::require_access(&headers)` | 反洋葱模型 | **H-7 / F-5** | M1 PR-4 |
| categories 数据源 | `photo_service.rs:147` 4 个字面量 vec | 不读 DB（DDL 已种子） | **H-1** | M3 PR-1 |
| per-photo passcode | 全局常量 `UNLOCK_PASSCODE = "1234"` | 共享口令 | **H-3** | M3（迁到 `photos.passcode_hash`） |
| users 表使用 | （未读） | 整张表零使用 | **H-1 / H-3** | M1 PR-3 |
| JWT crate | 自实现 hmac+sha2+base64 | 可读但偏离 spec | **H-2 选型** | M1 PR-3 改 `jsonwebtoken` |

### 13.3 工程纪律层面的隐患

| # | 现状 | 期望 |
| --- | --- | --- |
| E-1 | `.gitignore` 仅 4 行（target/node_modules/tsbuildinfo/.DS_Store） | 至少补回：`.env*` `keys/*` `*.pem` `*.key` `*.log` `.idea/` `.vscode/`；防止 secret 误提交 |
| E-2 | `config/default.toml.[jwt]` 含明文 `admin_password` | 删除；改在 `.env` 注入；`JwtCfg` 不该承载用户凭据 |
| E-3 | `AGENTS.md` 是 `CLAUDE.md` 的副本 | 改为"指向本文档 + 复述硬约束"的薄版 |
| E-4 | 0 集成测试 | M1 PR-5：testcontainers 起 PG/Redis 写 login → admin/photos 闭环 |
| E-5 | `Cargo.toml` 缺 `fred / aws-sdk-s3 / argon2 / jsonwebtoken / validator / rand` | M1 PR-1 第一件事 |
| E-6 | `PhotoDto.id: u64` | 改 `i64`（与 PG `BIGSERIAL` 对齐）；前端 ts `number` 兼容 |
| E-7 | DTO 字段 `cat / src / loc / date` 随前端，但**没有** `slug / sort_order / published_at / cover_url / uploaded_by` | M3：补全字段（前端忽略即可，遵循 H-10） |

---

## 14. M1 任务规格 — Auth + DB + Redis 接入（PR-by-PR）

> **目标**：把 §13.2 全部 ❌ 项替换为合规实现，**前端 API 形状（§9）不变**。
>
> 拆 5 个原子 PR；每个 PR 末尾的 **DoD（Definition of Done）** 块**必须**复制到 PR 描述并贴实际输出，否则视为未完成。

### 14.1 PR-1：依赖与 AppState 装配（不破坏前端）

**改动文件**

`Cargo.toml`（workspace `[workspace.dependencies]`）追加：

```toml
fred         = { version = "9",  features = ["enable-rustls", "metrics"] }
jsonwebtoken = "9"
argon2       = "0.5"
rand         = "0.8"
validator    = { version = "0.18", features = ["derive"] }
```

`crates/cms-api/Cargo.toml` 引入上述 + `sea-orm` + `cms-entity`；同时**删除** `base64 / hmac / sha2`（不再自实现 JWT，见 PR-3）。

`crates/cms-api/src/bootstrap.rs` 替换：

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db:     sea_orm::DatabaseConnection,
    pub redis:  fred::clients::RedisPool,
    pub jwt:    Arc<JwtKeys>,
    // 严禁再持有 photos / cache / token_blacklist 三个 in-memory 字段
}
```

`AppState::init()` 真正连接 PG + Redis；连接失败必须 `Err` 退出（**禁止**静默回退到 in-memory）。

`handlers::health_handler::readyz` 改为：先 `state.db.ping().await`、`state.redis.ping().await`，任一失败返 503。

`docker-compose.yml`（仓库根，**新建**）：内容直抄 §11.2 yaml（postgres:18-alpine + redis:7-alpine + minio）。本 PR 顺手交付，DoD 才能跑。

`.gitignore`：补回 §13.3 E-1 列表（`.env*` `keys/*` `*.pem` `*.key` `*.log` `.idea/` `.vscode/`）。

##### 过渡策略：删除 in-memory 字段后下游编译会炸，怎么办？

`photo_service` / `auth_service` / `media_service` 现有函数体都在用被删字段。本 PR **不**重写这些业务逻辑（那是 PR-2/PR-3 的事），但必须让 `cargo check` 绿。具体做法：

```rust
// services/photo_service.rs — 函数签名不动，只把函数体整段替换：
pub async fn list_public(state: &AppState, query: PhotoQuery) -> AppResult<Vec<PhotoDto>> {
    Err(AppError::Internal("not implemented in PR-1, see PR-2"))
}
// 不返 Result 的工具函数：
fn seed_photos() { todo!("removed in PR-2; replaced by `cargo run --bin seed_demo`") }
// 直接整体删除 seed_photos() 函数也可以——本 PR 之后用不到了。
```

副作用（**预期**，不是 bug）：

- `/healthz` `/readyz` 仍 200（不依赖被删字段）
- `/api/v1/photos` `/api/v1/admin/*` `/api/v1/auth/login` 等所有业务端点本 PR 后**全部返 500 INTERNAL**——PR-2 / PR-3 会逐个救活
- 前端 admin 登录后会看到 500 列表——这是过渡态，PR-3 完成后自动恢复

这一步算"保编译"的过渡桥，不计入 §H-1 / §H-2 违规——只要 PR-2 / PR-3 按期把 `Err(...)` 替换为真实现即可。

**不能动**：handler 签名、DTO 字段名、`frontend/` 任意文件、其它 PR 范围内的代码。

#### DoD（贴到 PR 描述）

```sh
# 0) 拉起依赖并起 cms-api（必须先让 init() 连接成功）
docker compose up -d postgres redis
sleep 3
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api > /tmp/cms.log 2>&1 &
SERVER_PID=$!
for i in 1 2 3 4 5 6 7 8 9 10; do
  curl -fsS -o /dev/null http://127.0.0.1:18080/healthz 2>/dev/null && break
  sleep 0.5
done

# 1) 一切就绪 → readyz 200
curl -s -o /dev/null -w "step-1 expect 200: HTTP %{http_code}\n" http://127.0.0.1:18080/readyz

# 2) 关掉 postgres（进程不退出，但 readyz 现场 ping 失败 → 503）
docker compose stop postgres
sleep 2
curl -s -o /dev/null -w "step-2 expect 503: HTTP %{http_code}\n" http://127.0.0.1:18080/readyz

# 3) 起回来 → 200 恢复
docker compose start postgres
sleep 3
curl -s -o /dev/null -w "step-3 expect 200: HTTP %{http_code}\n" http://127.0.0.1:18080/readyz

# 4) 收工
kill -INT $SERVER_PID; wait $SERVER_PID 2>/dev/null
echo "── tail of startup log ──"; tail -20 /tmp/cms.log
```

启动日志必须出现下面两行（具体措辞可以调，关键字必须命中）：

- 含 `postgres` 和 `pool` / `connected` 字样（说明 SeaORM 连接成功）
- 含 `redis` 和 `ping` / `connected` 字样（说明 fred 连接成功）

> 顺序很重要：**必须先把 PG/Redis 起来再 `cargo run`**，因为 `AppState::init` 是严格模式（连接失败直接 `Err` 退出）。否则 step-2 才能验证 readyz 的运行时降级。

---

### 14.2 PR-2：跑迁移 + 生成 Entity + photo 真持久化

> **目标**：把 `photo_service` 全部 8 个 `Err("not implemented in PR-1, see PR-2")` 替换为真的 PG 持久化（仅 `unlock` 留给 PR-3）；`auth_service / media_service` 继续保持 `Err(...)` stub。
>
> **前提**：PR-1 已落地，PG/Redis 已 healthy（`bash scripts/dev-up.sh --skip-dod` 一次即可）。

#### 14.2.1 改动文件清单（按交付顺序）

| 步骤 | 文件 / 命令 | 动作 |
| --- | --- | --- |
| 1 | `cargo run -p migrations -- up` | 跑迁移：建 8 张表 + 3 条种子分类 |
| 2 | `cargo install sea-orm-cli --locked`（如未装） | 一次性 |
| 3 | `sea-orm-cli generate entity ...`（命令见下） | **会覆写** `crates/cms-entity/src/lib.rs` + `prelude.rs`（含原来的占位注释），属于预期 |
| 4 | `crates/cms-api/src/repositories/photo_repo.rs`（**新建**） | 纯 SeaORM 数据访问，签名只接 `&DatabaseConnection` |
| 5 | `crates/cms-api/src/repositories/mod.rs` | 加 `pub mod photo_repo;` |
| 6 | `crates/cms-api/src/dto/photo_dto.rs` | `PhotoDto.id: u64` → **`i64`**；其它字段不动 |
| 7 | `crates/cms-api/src/handlers/photo_handler.rs` | `Path<u64>` → `Path<i64>`（共 5 处：update/update_privacy/delete/unlock 等） |
| 8 | `crates/cms-api/src/services/photo_service.rs` | 8 个函数中 7 个替换为真实现（仅 `unlock` 仍 `Err(...)`） |
| 9 | `crates/cms-api/src/bin/seed_demo.rs`（**新建**） | 入库 22 条 demo + 5 条 locked 的 argon2 哈希 |
| 10 | `crates/cms-api/Cargo.toml` | 在 `[[bin]]` 段加 `name = "seed_demo"` 入口；如还缺 `argon2` 则加（PR-1 应已加） |

##### sea-orm-cli 命令（与 §11.3 一致）

```sh
sea-orm-cli generate entity \
    -u "$APP_DATABASE__URL" \
    -o crates/cms-entity/src \
    --with-serde both \
    --serde-skip-deserializing-primary-key \
    --lib
```

#### 14.2.2 photo_repo.rs API 形状（最小集）

只暴露这些函数；事务 / 缓存策略**不在 repo**做（H-7）：

```rust
pub struct PhotoFull {
    pub photo: cms_entity::photo::Model,
    pub asset: cms_entity::media_asset::Model,
    pub category_slug: String,
    pub title_zh: String, pub title_en: String,
    pub loc_zh: String,   pub loc_en: String,
}

pub async fn list_public(db: &DatabaseConnection, cat_slug: Option<&str>) -> Result<Vec<PhotoFull>, DbErr>;
pub async fn list_admin (db: &DatabaseConnection, cat_slug: Option<&str>) -> Result<Vec<PhotoFull>, DbErr>;
pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<PhotoFull>, DbErr>;
pub async fn create(db: &DatabaseConnection, input: NewPhoto) -> Result<i64, DbErr>;
pub async fn update(db: &DatabaseConnection, id: i64, input: NewPhoto) -> Result<(), DbErr>;
pub async fn update_privacy(db: &DatabaseConnection, id: i64, p: Privacy, passcode_hash: Option<String>) -> Result<(), DbErr>;
pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr>;
pub async fn truncate_all(db: &DatabaseConnection) -> Result<(), DbErr>;  // for reset()

pub struct NewPhoto {
    pub slug: String,
    pub category_slug: String,        // service 层先查 categories.id 再传 i64 也行
    pub src_url: String,              // 写入 media_assets.storage_key
    pub mime_type: String,            // 默认 "image/jpeg"
    pub privacy: Privacy,
    pub passcode_hash: Option<String>,
    pub taken_at_label: String,
    pub title_zh: String, pub title_en: String,
    pub loc_zh: String,   pub loc_en: String,
}
```

`list_*` 必须**一次 SQL 取齐**（INNER JOIN photos × media_assets × categories + LEFT JOIN photo_translations 折叠 zh/en）。不要走 N+1。SeaORM 的 `find_with_related` 或手写 join builder 都行。

`create / update` 必须在**同一个事务**里（`db.begin().await?`）依次：
1. INSERT INTO `media_assets` 拿 `asset_id`
2. INSERT INTO `photos` (primary_asset_id = asset_id) 拿 `photo_id`
3. INSERT INTO `photo_translations` 两行 (zh + en)
4. `txn.commit()`

#### 14.2.3 photo_service 各函数应该改成什么

| 函数 | 现在 | PR-2 后 |
| --- | --- | --- |
| `list_public` | `Err(...)` | `photo_repo::list_public(&state.db, query.category.as_deref())` → `assemble_dto(...)` |
| `list_admin`  | `Err(...)` | 同上 |
| `categories`  | `Err(...)` | 直接 `cms_entity::category::Entity::find().order_by_asc(SortOrder).all(&state.db)` → 拼成 `Vec<CategoryDto>` |
| `create`      | `Err(...)` | `validate(&req)?` → `photo_repo::create(&state.db, NewPhoto::from(req))` → `find_by_id` 回查返 DTO |
| `update`      | `Err(...)` | 同上，update 后 `find_by_id` |
| `update_privacy` | `Err(...)` | 切到 `Locked` 时 `passcode_hash = Some(argon2_hash("1234"))`；其它情况 `None` |
| `delete`      | `Err(...)` | `photo_repo::delete(...)` |
| `reset`       | `Err(...)` | `photo_repo::truncate_all(...)` 然后调用 seed 函数（与 seed_demo bin 共用） |
| `unlock`      | `Err(...)` | **保持 `Err(...)`**，PR-3 才用 argon2 verify |

同时**删除** photo_service 顶部的 `UNLOCK_PASSCODE = "1234"` 常量（已迁移到 seed/argon2）。

#### 14.2.4 DTO ↔ DB 字段映射（前端契约不变 → H-10）

| DTO（wire） | DB | 备注 |
| --- | --- | --- |
| `id: i64` | `photos.id BIGSERIAL` | 类型从 u64 改 i64；TS 端仍 `number` |
| `src: String` | `media_assets.storage_key` | M2 接 S3 之前直接当 URL 用（demo 数据填外部 Unsplash URL） |
| `cat: String` | `categories.slug`（JOIN） | 不返 category_id |
| `title.zh / title.en` | `photo_translations` 两行折叠 | service 层 fold |
| `loc.zh / loc.en` | 同上 | |
| `date: String` | `photos.taken_at_label` | 原样字符串 "2024.05" |
| `privacy: Privacy` | `photos.privacy` | enum 序列化 |

#### 14.2.5 seed_demo bin

`crates/cms-api/src/bin/seed_demo.rs`：

- 数据源：**逐条复制** `design/data.jsx` 的 `DEFAULT_PHOTOS` 数组（22 条）。每条：
  - `id` 不要复用 jsx 里的 1..22，让 PG `BIGSERIAL` 自增。
  - `slug = format!("demo-{}", idx)`（idx 是 1..=22）。
  - `category_slug` 直接用 jsx 的 `cat` 字段。
  - `src_url` 直接用 jsx 的 `src` 字段（外部 URL）。
  - `taken_at_label` 直接用 jsx 的 `date` 字段。
  - `title.zh / title.en / loc.zh / loc.en` 取 jsx 对应字段。
  - `privacy` map 自 jsx：`"public" → Public` / `"locked" → Locked` / `"private" → Private`。
  - `Locked` 的：`passcode_hash = Some(argon2_hash("1234"))`（用 argon2 crate `Argon2::default().hash_password(...)`）。其它为 `None`。
- 流程：
  1. 装配 AppState（复用 `bootstrap::AppState::init`）
  2. `photo_repo::truncate_all(&state.db).await?`（幂等，可反复跑）
  3. for each demo → `photo_repo::create(&state.db, NewPhoto { ... }).await?`
  4. 打印 `seeded N photos (public=A locked=B private=C)`

`crates/cms-api/Cargo.toml`：

```toml
[[bin]]
name = "cms-api"
path = "src/main.rs"

[[bin]]
name = "seed_demo"
path = "src/bin/seed_demo.rs"
```

#### 14.2.6 不能动的东西

- 前端 `frontend/src/api/client.ts` 与 `frontend/src/types.ts`（H-10）
- `auth_service` / `media_service` 的 `Err(...)` stub（PR-3 / M2 才动）
- `routes.rs`（路由表 PR-1 已对齐）
- `bootstrap.rs::AppState`（PR-1 已对齐）
- `unlock` handler 行为（仍返 500 INTERNAL，PR-3 救活）

#### DoD（贴到 PR 描述）

```sh
# 0) 前提：PG/Redis 已 healthy
bash scripts/dev-up.sh --skip-dod

# 1) 跑迁移
cargo run -p migrations -- up
# 期望：apply m20260506_000001_init OK

# 2) 验表 + 种子分类
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "\dt" \
  -c "SELECT slug FROM categories ORDER BY sort_order;"
# 期望：8 张表 + 3 条 categories（street/landscape/life）

# 3) 生成 entity
sea-orm-cli generate entity -u "$APP_DATABASE__URL" \
  -o crates/cms-entity/src --with-serde both --lib
ls crates/cms-entity/src/  # 期望：lib.rs + photo.rs + media_asset.rs + ... 等 ≥ 8 个

# 4) cargo check 全过
cargo check --workspace --all-targets

# 5) 跑 seed
cargo run -p cms-api --bin seed_demo
# 期望末行：seeded 22 photos (public=N locked=M private=K)

# 6) 验数据
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT count(*) AS photos FROM photos;" \
  -c "SELECT count(*) AS translations FROM photo_translations;" \
  -c "SELECT count(*) AS assets FROM media_assets;" \
  -c "SELECT count(*) AS locked FROM photos WHERE passcode_hash IS NOT NULL;"
# 期望：photos=22, translations=44, assets=22, locked=5

# 7) 起 cms-api 联调
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api &
SERVER_PID=$!
sleep 3

curl -s "http://127.0.0.1:18080/api/v1/photos" | jq 'length'
# 期望：>= 17（公开 + locked，排除 private）

curl -s "http://127.0.0.1:18080/api/v1/photos?category=street" | jq '.[0]'
# 期望：含 id/src/cat/title/loc/date/privacy 七个字段，title/loc 是 {zh,en} 对象

curl -s "http://127.0.0.1:18080/api/v1/categories" | jq
# 期望：3 条分类对象（不是 500）

# 8) 重启验持久化
kill -INT $SERVER_PID; wait $SERVER_PID 2>/dev/null
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api &
SERVER_PID=$!
sleep 3
curl -s "http://127.0.0.1:18080/api/v1/photos" | jq 'length'
# 期望：与第 7 步一致（数据从 PG 读，不丢）
kill -INT $SERVER_PID; wait $SERVER_PID 2>/dev/null
```

> **预期回归**：`/api/v1/admin/*` 仍 401（auth 中间件未挂）/ 500（auth_service 还是 stub）；`/photos/:id/unlock` 仍 500；`/media/presign` 仍 500。这都是 PR-3 / PR-4 / M2 的事。

---

### 14.3 PR-3：真 Auth — argon2 + Redis 黑名单 + logout-all + photo unlock

> **目标**：把 `auth_service` 5 个 `Err(...)` stub 全部替换为真实现 + 新增 `logout-all` 端点 + 让 `photo_service::unlock` 走 argon2 verify。
>
> **前提**：PR-2 已落地（`photo_service` 7 个函数已活、photos 已种 22 条）；PG/Redis 在线。

#### 14.3.1 改动文件清单

| 步骤 | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/repositories/user_repo.rs`（**新建**） | `find_by_email` / `find_by_id` / `insert` |
| 2 | `crates/cms-api/src/repositories/mod.rs` | 加 `pub mod user_repo;` |
| 3 | `crates/cms-api/src/dto/auth_dto.rs` | `LoginReq.email: Option<String>` → **`pub email: String`** + `#[validate(email)]`；`UserDto.id: u64` → **`i64`**（与 users 表 `BIGSERIAL` 一致） |
| 4 | `crates/cms-api/src/infra/jwt.rs` | `Claims` 增加 **`pub iat: i64`** 字段；`encode` 同步填 `iat = now`（user-rev 校验需要） |
| 5 | `crates/cms-api/src/services/auth_service.rs` | 5 个 stub 全改真实现 + 新增 `logout_all`；`bearer_token_from(headers)` 抽到模块顶部 |
| 6 | `crates/cms-api/src/services/photo_service.rs::unlock` | 替换 `Err(...)` 为 argon2 verify on `photos.passcode_hash` |
| 7 | `crates/cms-api/src/handlers/auth_handler.rs` | 新增 `logout_all` handler（签名同 `logout`） |
| 8 | `crates/cms-api/src/routes.rs` | `api` Router 加 `POST /auth/logout-all` |
| 9 | `crates/cms-api/src/config.rs` | 从 `JwtCfg` **删除** `admin_email` / `admin_password` 字段 + 对应 `default_*()` 工厂函数 |
| 10 | `config/default.toml` | **删除** `[jwt].admin_email` / `admin_password` 两行 |
| 11 | `crates/cms-api/src/bin/seed_admin.rs`（**新建**） | `cargo run --bin seed_admin -- <email> <password>` 用 argon2 入库 |
| 12 | `crates/cms-api/Cargo.toml` | `[[bin]]` 段加 `seed_admin` 入口 |

#### 14.3.2 Claims 增加 iat 字段

`infra/jwt.rs` 当前 Claims 缺 `iat`（user-rev 必须靠它）。补上：

```rust
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub typ: String,
    pub iat: i64,        // ← 新增
    pub exp: i64,
    pub jti: String,
}
```

`encode` 函数同时填 `iat = chrono::Utc::now().timestamp()`。`decode` 不需要新校验（serde 自动反序列化即可）。

#### 14.3.3 require_access 三段式

```rust
pub async fn require_access(state: &AppState, headers: &HeaderMap) -> AppResult<Claims> {
    // 1. 解析 + 验签 + 验 typ + 验 exp（jsonwebtoken 已经做了 exp）
    let token = bearer_token_from(headers)?;
    let claims = jwt::decode(&state.config.jwt, token, "access")?;

    // 2. 单 token 黑名单
    let blk_key = format!("auth:blacklist:{}", claims.jti);
    let blacklisted: bool = state.redis.exists(&blk_key).await?;
    if blacklisted { return Err(AppError::Unauthorized("token revoked")); }

    // 3. 全用户 epoch（踢全设备）
    let rev_key = format!("auth:user-rev:{}", claims.sub);
    if let Ok(Some(epoch_str)) = state.redis.get::<Option<String>, _>(&rev_key).await {
        if let Ok(epoch) = epoch_str.parse::<i64>() {
            if claims.iat < epoch {
                return Err(AppError::Unauthorized("session terminated"));
            }
        }
    }
    Ok(claims)
}

fn bearer_token_from(headers: &HeaderMap) -> AppResult<&str> {
    headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").or_else(|| s.strip_prefix("bearer ")))
        .ok_or(AppError::Unauthorized("missing bearer token"))
}
```

#### 14.3.4 login / logout / logout_all / refresh / me 行为契约

- **`login`**：
  1. `req.validate()?`
  2. `user_repo::find_by_email(&state.db, &req.email).await?` → `ok_or Unauthorized("invalid credentials")`
  3. argon2 verify；失败也映射为 `Unauthorized("invalid credentials")`（**不要泄露**用户存在 vs 密码错）
  4. 签 access(typ="access", ttl=cfg.access_ttl_secs) + refresh(typ="refresh", ttl=cfg.refresh_ttl_secs)
  5. refresh.jti 写 Redis：`auth:refresh:{jti} = user_id` TTL=refresh_ttl
  6. 返 `TokenPairResp { user: UserDto::from(&user), ... }`

- **`logout`**（保留 `headers: &HeaderMap` 签名，PR-4 才换中间件）：
  ```rust
  let token = bearer_token_from(headers)?;
  let claims = jwt::decode(&state.config.jwt, token, "access")?;
  let ttl = (claims.exp - chrono::Utc::now().timestamp()).max(0);
  let key = format!("auth:blacklist:{}", claims.jti);
  let _: () = state.redis.set(&key, "1",
      Some(fred::types::Expiration::EX(ttl)), None, false).await?;
  ```

- **`logout_all`**（新增，签名同 `logout`，从 token 里拿 user_id）：
  ```rust
  let claims = require_access(state, headers).await?;
  let key = format!("auth:user-rev:{}", claims.sub);
  let now = chrono::Utc::now().timestamp().to_string();
  let _: () = state.redis.set(&key, now,
      Some(fred::types::Expiration::EX(state.config.jwt.refresh_ttl_secs as i64)),
      None, false).await?;
  ```

- **`refresh`**：解析 refresh token (typ="refresh") → 校验 `auth:refresh:{jti}` 在 Redis 仍存在（防 logout-all 后还能 refresh）→ `user_repo::find_by_id` 拿最新 user → 签新 pair → **删旧 refresh.jti、写新 refresh.jti**（refresh token rotation）。

- **`me`**：`require_access(state, headers).await?` → `user_repo::find_by_id(claims.sub.parse::<i64>()?).await?` → 返 `MeResp { user }`。

#### 14.3.5 photo_service::unlock 真实现

```rust
pub async fn unlock(state: &AppState, id: i64, req: UnlockReq) -> AppResult<UnlockResp> {
    // 简化版限流：按 photo_id（生产应叠加 IP）
    let rl_key = format!("auth:photo:unlock:rl:{id}");
    let n: i64 = state.redis.incr(&rl_key).await?;
    if n == 1 { let _: () = state.redis.expire(&rl_key, 600).await?; }
    if n > 10 { return Err(AppError::TooManyRequests("unlock attempts")); }

    let p = photo_repo::find_by_id(&state.db, id).await?
        .ok_or(AppError::NotFound)?;
    if p.photo.privacy == "private" { return Err(AppError::Forbidden); }
    if p.photo.privacy == "public"  { return Ok(UnlockResp { unlocked: true }); }

    // locked
    let hash = p.photo.passcode_hash
        .ok_or(AppError::Internal("locked photo missing passcode_hash"))?;
    let parsed = argon2::password_hash::PasswordHash::new(&hash)
        .map_err(|_| AppError::Internal("hash parse"))?;
    let ok = argon2::Argon2::default()
        .verify_password(req.passcode.as_bytes(), &parsed)
        .is_ok();
    Ok(UnlockResp { unlocked: ok })
}
```

#### 14.3.6 user_repo.rs 最小集

```rust
use cms_entity::users::{Entity as Users, Column, Model, ActiveModel};
use sea_orm::*;

pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> Result<Option<Model>, DbErr> {
    Users::find().filter(Column::Email.eq(email)).one(db).await
}
pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<Model>, DbErr> {
    Users::find_by_id(id).one(db).await
}
pub async fn insert(
    db: &DatabaseConnection,
    email: String, password_hash: String, role: String,
) -> Result<Model, DbErr> {
    ActiveModel {
        email: Set(email),
        password_hash: Set(password_hash),
        role: Set(role),
        ..Default::default()
    }.insert(db).await
}
```

> ⚠️ sea-orm-cli 生成的 entity 模块名是**复数**（`cms_entity::users`、`cms_entity::photos`），别拼成单数。

#### 14.3.7 seed_admin bin

```rust
//! cargo run -p cms-api --bin seed_admin -- <email> <password>
use cms_api::{bootstrap, config, observability, repositories::user_repo};
use argon2::{Argon2, PasswordHasher,
    password_hash::{rand_core::OsRng, SaltString}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::init();
    let mut args = std::env::args().skip(1);
    let email    = args.next().expect("usage: seed_admin <email> <password>");
    let password = args.next().expect("usage: seed_admin <email> <password>");

    let cfg   = config::load()?;
    let state = bootstrap::AppState::init(cfg).await?;

    if user_repo::find_by_email(&state.db, &email).await?.is_some() {
        println!("user {email} already exists, skipping");
        return Ok(());
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2 hash: {e}"))?
        .to_string();
    let user = user_repo::insert(&state.db, email.clone(), hash, "owner".into()).await?;
    println!("seeded user id={} email={}", user.id, user.email);
    Ok(())
}
```

`Cargo.toml`：
```toml
[[bin]]
name = "seed_admin"
path = "src/bin/seed_admin.rs"
```

#### 14.3.8 不能动

- `frontend/` 任何文件
- `media_service.rs` 函数体（M2 才动）
- `bootstrap.rs::AppState`（PR-1 已定型）
- 现有 admin handler 中的 `auth_service::require_access(...)` 调用方式（PR-4 才换中间件）
- DTO 命名 / `error.rs` AppError 形状

#### DoD（贴 PR 描述；端口与 dev-up.sh 一致用 18080）

```sh
# 0) 前提：dev-up.sh --skip-dod 起好；photos 已 seeded（PR-2）
docker compose ps postgres redis
docker exec gathered-pg psql -U postgres -d gathered_light -c "SELECT count(*) FROM photos;"
# 期望：22

# 1) seed admin → users 表入库
cargo run -p cms-api --bin seed_admin -- admin@local.dev "Passw0rd!"
# 期望末行：seeded user id=1 email=admin@local.dev
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT id, email, role, length(password_hash) FROM users;"
# 期望：1 行；password_hash length > 60（argon2id 哈希）

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api > /tmp/cms.log 2>&1 &
SERVER_PID=$!
sleep 3

# 3) 错密码 / 错邮箱必 401（消息一致，不泄露用户存在性）
curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@local.dev","password":"wrong"}' | jq -r .code
curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"none@local.dev","password":"Passw0rd!"}' | jq -r .code
# 都期望：UNAUTHORIZED

# 4) 正确密码 → 拿 token + 验证 user 字段
LOGIN=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@local.dev","password":"Passw0rd!"}')
echo "$LOGIN" | jq
ACCESS=$(echo "$LOGIN"  | jq -r .access_token)
REFRESH=$(echo "$LOGIN" | jq -r .refresh_token)

# 5) admin 接口带 token → 200（PR-2 已活的 photo 列表）
curl -s -o /dev/null -w "expect 200: HTTP %{http_code}\n" \
  -H "Authorization: Bearer $ACCESS" http://127.0.0.1:18080/api/v1/admin/photos

# 6) /auth/me → 200 + 用户信息
curl -s -H "Authorization: Bearer $ACCESS" http://127.0.0.1:18080/api/v1/auth/me | jq

# 7) logout 后立即失效
curl -s -X POST -H "Authorization: Bearer $ACCESS" http://127.0.0.1:18080/api/v1/auth/logout | jq
curl -s -o /dev/null -w "expect 401: HTTP %{http_code}\n" \
  -H "Authorization: Bearer $ACCESS" http://127.0.0.1:18080/api/v1/admin/photos

# 8) Redis 真有 blacklist key
docker exec gathered-redis redis-cli --scan --pattern 'auth:blacklist:*' | head
# 期望：≥ 1 行

# 9) refresh 换新 access（rotation：旧 refresh 失效）
NEW=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/refresh \
  -H 'content-type: application/json' \
  -d "{\"refresh_token\":\"$REFRESH\"}")
NEW_ACCESS=$(echo "$NEW" | jq -r .access_token)
[ "$ACCESS" != "$NEW_ACCESS" ] && echo "refresh OK (token rotated)" || echo "FAIL"
# 老 refresh 再用应失败
curl -s -X POST http://127.0.0.1:18080/api/v1/auth/refresh \
  -H 'content-type: application/json' \
  -d "{\"refresh_token\":\"$REFRESH\"}" | jq -r .code
# 期望：UNAUTHORIZED

# 10) logout-all：起两个 session 全踢
T1=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@local.dev","password":"Passw0rd!"}' | jq -r .access_token)
T2=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@local.dev","password":"Passw0rd!"}' | jq -r .access_token)
curl -s -X POST -H "Authorization: Bearer $T1" http://127.0.0.1:18080/api/v1/auth/logout-all
sleep 1
curl -s -o /dev/null -w "T1: %{http_code}  " -H "Authorization: Bearer $T1" http://127.0.0.1:18080/api/v1/admin/photos
curl -s -o /dev/null -w "T2: %{http_code}\n" -H "Authorization: Bearer $T2" http://127.0.0.1:18080/api/v1/admin/photos
# 期望：T1: 401  T2: 401

# 11) Redis user-rev key
docker exec gathered-redis redis-cli --scan --pattern 'auth:user-rev:*' | head
# 期望：1 行（admin user_id=1）

# 12) photo unlock：locked 图错口令 → unlocked:false；正确口令 "1234" → true
LOCKED_ID=$(docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
  -c "SELECT id FROM photos WHERE privacy = 'locked' LIMIT 1;")
echo "locked photo id = $LOCKED_ID"
curl -s -X POST http://127.0.0.1:18080/api/v1/photos/$LOCKED_ID/unlock \
  -H 'content-type: application/json' -d '{"passcode":"wrong"}' | jq
# 期望：{"unlocked": false}
curl -s -X POST http://127.0.0.1:18080/api/v1/photos/$LOCKED_ID/unlock \
  -H 'content-type: application/json' -d '{"passcode":"1234"}' | jq
# 期望：{"unlocked": true}

# 13) 收工
kill -INT $SERVER_PID; wait $SERVER_PID 2>/dev/null
```

> **预期回归**：所有 admin handler 顶部仍保留 `auth_service::require_access(...)` 调用——这违反 H-7 / F-5，是**预期遗留**，PR-4 才统一改为中间件。本 PR 不动。

---

### 14.4 PR-4：抽出 Tower auth 中间件（消除 handler 内 require_access 重复）

> **目标**：消除 §F-5 反模式——把 admin handler 顶部那一行 `auth_service::require_access(&state, &headers).await?;` 抽到 Tower middleware，路由层一次性挂上。
>
> **特性**：纯机械重构。前端契约 / 业务行为 100% 不变；只换"鉴权在哪儿做"。
>
> **前提**：PR-3 已落地（auth_service::require_access 真实工作）。

#### 14.4.1 公开路径 vs 受保护路径分组

> 这一节是 PR-4 最容易拍脑袋出错的地方——Codex 必须**严格按此表分组**。

| 路径 | 方法 | 分组 |
| --- | --- | --- |
| `/healthz` `/readyz` | GET | 完全公开（无 `/api/v1` 前缀） |
| `/api/v1/auth/login` | POST | 公开（要靠它登录拿 token） |
| `/api/v1/auth/refresh` | POST | 公开（refresh token 内含校验） |
| `/api/v1/categories` | GET | 公开 |
| `/api/v1/photos` | GET | 公开（前台瀑布流） |
| `/api/v1/photos/:id/unlock` | POST | 公开（locked 图给未登录访客解锁） |
| `/api/v1/auth/logout` | POST | **受保护**（需要有效 access token 才能 invalidate jti） |
| `/api/v1/auth/logout-all` | POST | **受保护** |
| `/api/v1/auth/me` | GET | **受保护** |
| `/api/v1/media/presign` | POST | **受保护**（仅登录用户能拿签名 URL） |
| `/api/v1/admin/photos` | GET / POST | **受保护** |
| `/api/v1/admin/photos/reset` | POST | **受保护** |
| `/api/v1/admin/photos/:id` | PUT / DELETE | **受保护** |
| `/api/v1/admin/photos/:id/privacy` | PATCH | **受保护** |

#### 14.4.2 改动文件清单

| 步骤 | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/middleware/auth.rs`（**新建**） | `jwt_guard` 中间件函数 |
| 2 | `crates/cms-api/src/middleware/mod.rs` | 把当前注释改为 `pub mod auth;` |
| 3 | `crates/cms-api/src/routes.rs` | 把 `api` Router 拆成 `public_api` + `protected_api`，protected 挂 `route_layer(jwt_guard)` |
| 4 | `crates/cms-api/src/handlers/photo_handler.rs` | 删除 6 个 admin handler 顶部的 `auth_service::require_access(...)` 调用 + 对应 `headers: HeaderMap` 参数 |
| 5 | `crates/cms-api/src/handlers/media_handler.rs` | 删除 `presign` handler 的 require_access + headers |
| 6 | `crates/cms-api/src/handlers/auth_handler.rs` | `logout` / `logout_all` / `me` 改用 `Extension<Claims>` 取信息（不再从 headers 解析） |
| 7 | `crates/cms-api/src/services/auth_service.rs` | `logout` / `logout_all` / `me` 函数签名从 `&HeaderMap` 改为 `&Claims`（直接接收已验证 claims） |

#### 14.4.3 中间件函数

`crates/cms-api/src/middleware/auth.rs`：

```rust
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    bootstrap::AppState,
    error::AppError,
    infra::jwt::Claims,
    services::auth_service,
};

pub async fn jwt_guard(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 复用 auth_service::require_access 的完整三段式校验
    // (decode + blacklist check + user-rev epoch check)
    let claims: Claims = auth_service::require_access(&state, req.headers()).await?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
```

#### 14.4.4 routes.rs 重构示例

```rust
pub fn build(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health_handler::healthz))
        .route("/readyz", get(health_handler::readyz));

    let public_api = Router::new()
        .route("/auth/login",        post(auth_handler::login))
        .route("/auth/refresh",      post(auth_handler::refresh))
        .route("/categories",        get(category_handler::list))
        .route("/photos",            get(photo_handler::list_public))
        .route("/photos/:id/unlock", post(photo_handler::unlock));

    let protected_api = Router::new()
        .route("/auth/logout",     post(auth_handler::logout))
        .route("/auth/logout-all", post(auth_handler::logout_all))
        .route("/auth/me",         get(auth_handler::me))
        .route("/media/presign",   post(media_handler::presign))
        .route("/admin/photos",
            get(photo_handler::list_admin).post(photo_handler::create))
        .route("/admin/photos/reset", post(photo_handler::reset))
        .route("/admin/photos/:id",
            put(photo_handler::update).delete(photo_handler::delete))
        .route("/admin/photos/:id/privacy",
            patch(photo_handler::update_privacy))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::jwt_guard,
        ));

    Router::new()
        .merge(public)
        .nest("/api/v1", public_api.merge(protected_api))
        .with_state(state)
        .fallback_service(/* ServeDir/ServeFile 不变 */)
        .layer(/* TraceLayer 不变 */)
        .layer(CorsLayer::permissive())
}
```

#### 14.4.5 handler 改动样例

**改前**（PR-3 状态）：

```rust
pub async fn list_admin(
    State(state): State<AppState>,
    headers: HeaderMap,                             // ← 删
    Query(query): Query<PhotoQuery>,
) -> AppResult<Json<Vec<PhotoDto>>> {
    auth_service::require_access(&state, &headers).await?;  // ← 删
    Ok(Json(photo_service::list_admin(&state, query).await?))
}
```

**改后**（PR-4）：

```rust
pub async fn list_admin(
    State(state): State<AppState>,
    Query(query): Query<PhotoQuery>,
) -> AppResult<Json<Vec<PhotoDto>>> {
    Ok(Json(photo_service::list_admin(&state, query).await?))
}
```

需要 claims 的 3 个 handler（logout / logout_all / me）：

```rust
use axum::Extension;
use crate::infra::jwt::Claims;

pub async fn logout(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<serde_json::Value>> {
    auth_service::logout(&state, &claims).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<MeResp>> {
    Ok(Json(auth_service::me(&state, &claims).await?))
}

pub async fn logout_all(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<serde_json::Value>> {
    auth_service::logout_all(&state, &claims).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
```

#### 14.4.6 service 改动样例

`auth_service`：把 `logout` / `logout_all` / `me` 的签名 `(state, &HeaderMap)` 换成 `(state, &Claims)`，函数体里**删掉** `bearer_token_from(headers)?` 和 `jwt::decode(...)?` 这两步——claims 已经被中间件验过了。

```rust
pub async fn logout(state: &AppState, claims: &Claims) -> AppResult<()> {
    let ttl = (claims.exp - chrono::Utc::now().timestamp()).max(0);
    let key = format!("auth:blacklist:{}", claims.jti);
    let _: () = state.redis.set(&key, "1",
        Some(fred::types::Expiration::EX(ttl)), None, false).await?;
    Ok(())
}

pub async fn logout_all(state: &AppState, claims: &Claims) -> AppResult<()> {
    let key = format!("auth:user-rev:{}", claims.sub);
    let now = chrono::Utc::now().timestamp().to_string();
    let _: () = state.redis.set(&key, now,
        Some(fred::types::Expiration::EX(state.config.jwt.refresh_ttl_secs as i64)),
        None, false).await?;
    Ok(())
}

pub async fn me(state: &AppState, claims: &Claims) -> AppResult<MeResp> {
    let user_id: i64 = claims.sub.parse()
        .map_err(|_| AppError::Internal("malformed sub"))?;
    let user = user_repo::find_by_id(&state.db, user_id).await?
        .ok_or(AppError::NotFound)?;
    Ok(MeResp { user: UserDto::from(&user) })
}
```

`require_access` 函数本身**保留不动**——中间件就是调它，不要重写一份。

#### 14.4.7 不能动

- `frontend/` 任何文件
- `bootstrap.rs::AppState`
- `media_service.rs` 函数体（仍 stub）
- `photo_service.rs` 任意函数（PR-2 / PR-3 已定型）
- `infra/jwt.rs::Claims` 形状（PR-3 加完 iat 就定型）
- 任何 DTO 字段名 / 类型
- 路径 / 方法 / status code 行为（H-10）

#### DoD（贴 PR 描述）

```sh
# 1) 静态：handler 内 require_access 应清零
grep -rn "require_access" crates/cms-api/src/handlers/ && \
  echo "❌ 还有残留" || echo "✅ no leftover in handlers/"

# 2) 静态：middleware 里应有 1 个 require_access 调用
grep -rn "require_access" crates/cms-api/src/middleware/
# 期望：middleware/auth.rs:N: ... auth_service::require_access(...)

# 3) 编译绿
cargo check --workspace --all-targets 2>&1 | tail -3
# 期望：Finished `dev` profile

# 4) 端到端（同 PR-3 13 步浓缩版即可）
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api &
PID=$!
sleep 3

# 4.1 公开路径无 token 仍然通
curl -s -o /dev/null -w "/photos no-token expect 200: %{http_code}\n" \
  http://127.0.0.1:18080/api/v1/photos
curl -s -o /dev/null -w "/categories no-token expect 200: %{http_code}\n" \
  http://127.0.0.1:18080/api/v1/categories

# 4.2 protected 路径无 token 必 401（中间件挡）
curl -s -o /dev/null -w "/admin/photos no-token expect 401: %{http_code}\n" \
  http://127.0.0.1:18080/api/v1/admin/photos
curl -s -o /dev/null -w "/auth/me no-token expect 401: %{http_code}\n" \
  http://127.0.0.1:18080/api/v1/auth/me
curl -s -o /dev/null -w "/auth/logout no-token expect 401: %{http_code}\n" \
  -X POST http://127.0.0.1:18080/api/v1/auth/logout
curl -s -o /dev/null -w "/media/presign no-token expect 401: %{http_code}\n" \
  -X POST http://127.0.0.1:18080/api/v1/media/presign

# 4.3 带 token 全部正常
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@local.dev","password":"Passw0rd!"}' | jq -r .access_token)
curl -s -o /dev/null -w "/admin/photos with-token expect 200: %{http_code}\n" \
  -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos
curl -s -o /dev/null -w "/auth/me with-token expect 200: %{http_code}\n" \
  -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/auth/me

# 4.4 logout 仍生效（验证 Claims 注入对了）
curl -s -X POST -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/auth/logout > /dev/null
curl -s -o /dev/null -w "after-logout expect 401: %{http_code}\n" \
  -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos

kill -INT $PID; wait $PID 2>/dev/null
```

> 通过条件：第 1 步必出 `✅ no leftover`；第 4.2 全部 401；第 4.3 全部 200；第 4.4 401。

---

### 14.5 PR-5：集成测试（M1 收官 · 复用 dev compose）

> ⚠️ **历史教训**：本节早期版本用 `testcontainers` 起独立 PG/Redis 容器跑测试。实际部署后发现：测试进程异常退出（CI 超时 / kill -9 / Codex 沙箱中断）会留 PG+Redis 孤儿容器**长达数小时**。每跑一次测试就多一对孤儿，docker host 资源占用直线上升。
>
> **本节已重写**：测试**复用 dev compose** 已起的 `gathered-pg / gathered-redis / gathered-minio`（**0 新容器**）。隔离方式：
> - PG：测试库 `gathered_light_test`（每次 build_app 时 DROP+CREATE，独立于 dev `gathered_light`）
> - Redis：DB index 15（dev 用 0）
> - MinIO：bucket `gathered-light-test`（独立于 dev `gathered-light`，首次自动创建）
>
> **绝对禁止**：testcontainers 系列 crate（`testcontainers` / `testcontainers-modules`）。已写入 §20 F-11。
>
> **目标**：跑端到端鉴权闭环测试，证明 PR-1..PR-4 的功能契约不会回退。
>
> **前提**：`bash scripts/dev-up.sh --skip-dod` 已起好 PG/Redis/MinIO。
>
> **设计原则**：
> - `tower::ServiceExt::oneshot` 在进程内驱动 axum Router，不绑 TCP 端口
> - `OnceCell` 共享 (Router, AppState)：第一次调用 build_app 时 DROP+CREATE 测试库 + 跑迁移 + seed admin；后续调用直接 clone

#### 14.5.1 改动文件清单

| 步骤 | 文件 | 动作 |
| --- | --- | --- |
| 1 | `Cargo.toml`（workspace） | **不加** `testcontainers*`；只加 `tower = { version = "0.5", features = ["util"] }`（用于 `ServiceExt::oneshot`） |
| 2 | `crates/cms-api/Cargo.toml` | `[dev-dependencies]` 仅含 `migrations.workspace = true` + `tower.workspace = true`（不再有 testcontainers） |
| 3 | `crates/cms-api/tests/common/mod.rs`（**新建**） | 测试公用 fixture：连 dev `postgres` DB → DROP+CREATE 测试库 → 装配 AppState → 跑迁移 → seed admin |
| 4 | `crates/cms-api/tests/auth_flow.rs`（**新建**） | 端到端鉴权闭环 `#[tokio::test]` |

#### 14.5.2 测试 fixture（`tests/common/mod.rs`）

```rust
//! 复用 dev compose 的 PG/Redis/MinIO，0 新容器。

use argon2::{Argon2, PasswordHasher, password_hash::{rand_core::OsRng, SaltString}};
use cms_api::{bootstrap::AppState, config::*, repositories::user_repo, routes};
use migrations::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database};
use tokio::sync::OnceCell;

const ADMIN_DB_URL: &str = "postgres://postgres:postgres@127.0.0.1:5432/postgres";
const TEST_DB_NAME: &str = "gathered_light_test";

static APP: OnceCell<(axum::Router, AppState)> = OnceCell::const_new();

pub async fn build_app() -> (axum::Router, AppState) {
    let (app, state) = APP.get_or_init(init_once).await;
    (app.clone(), state.clone())
}

async fn init_once() -> (axum::Router, AppState) {
    // 1) 连 dev `postgres` DB → DROP + CREATE 测试库
    let admin = Database::connect(ADMIN_DB_URL).await
        .expect("connect dev postgres failed; run `bash scripts/dev-up.sh --skip-dod` first");
    admin.execute_unprepared(&format!("DROP DATABASE IF EXISTS {TEST_DB_NAME}")).await.unwrap();
    admin.execute_unprepared(&format!("CREATE DATABASE {TEST_DB_NAME}")).await.unwrap();
    drop(admin);

    let cfg = Config {
        bind_addr: "127.0.0.1:0".into(),
        database: DatabaseCfg {
            url: format!("postgres://postgres:postgres@127.0.0.1:5432/{TEST_DB_NAME}"),
            pool_min: 1, pool_max: 4,
        },
        redis: RedisCfg { url: "redis://127.0.0.1:6379/15".into(), pool_size: 2 },
        s3: S3Cfg {
            endpoint: "http://localhost:9000".into(), region: "us-east-1".into(),
            bucket: "gathered-light-test".into(),
            access_key: "minioadmin".into(), secret_key: "minioadmin".into(),
            upload_max_bytes: 1024 * 1024,
        },
        jwt: JwtCfg {
            secret: "test-secret".into(),
            access_ttl_secs: 60, refresh_ttl_secs: 600,
            private_key_path: None, public_key_path: None,
        },
    };

    let state = AppState::init(cfg).await.expect("init AppState");
    Migrator::up(&state.db, None).await.expect("run migrations");

    // Redis DB 15 不主动清理：当前测试用 UUID jti 天然不冲突。
    // 若以后加 logout-all 测试，DEL `auth:user-rev:*` 或 `state.redis.next().flushdb(false)`。

    if user_repo::find_by_email(&state.db, "admin@test.dev").await.unwrap().is_none() {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default().hash_password(b"Passw0rd!", &salt).unwrap().to_string();
        user_repo::insert(&state.db, "admin@test.dev".into(), hash, "owner".into()).await.unwrap();
    }

    let app = routes::build(state.clone());
    (app, state)
}

pub fn json_body<T: serde::Serialize>(v: &T) -> axum::body::Body {
    axum::body::Body::from(serde_json::to_vec(v).unwrap())
}

pub async fn read_json(resp: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    serde_json::from_slice(&body).unwrap_or(serde_json::json!({"_raw": String::from_utf8_lossy(&body).into_owned()}))
}
```

#### 14.5.3 端到端测试（`tests/auth_flow.rs`）

```rust
//! 鉴权闭环：login → admin → logout → admin 401（Redis 黑名单生效）。

mod common;

use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;          // for `oneshot`

#[tokio::test]
async fn login_admin_logout_blacklist_full_loop() {
    let (app, _state) = common::build_app().await;

    // 1) 错密码必 401
    let resp = app.clone().oneshot(
        Request::builder()
            .uri("/api/v1/auth/login").method("POST")
            .header("content-type", "application/json")
            .body(common::json_body(&serde_json::json!({
                "email": "admin@test.dev", "password": "wrong"
            }))).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "wrong password should 401");

    // 2) 正确密码 → 拿 access + refresh
    let resp = app.clone().oneshot(
        Request::builder()
            .uri("/api/v1/auth/login").method("POST")
            .header("content-type", "application/json")
            .body(common::json_body(&serde_json::json!({
                "email": "admin@test.dev", "password": "Passw0rd!"
            }))).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = common::read_json(resp).await;
    let access = body["access_token"].as_str().expect("access_token").to_string();
    assert!(!access.is_empty());

    // 3) 无 token → admin/photos 必 401
    let resp = app.clone().oneshot(
        Request::builder().uri("/api/v1/admin/photos").body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no token → 401");

    // 4) 带 token → admin/photos 必 200
    let resp = app.clone().oneshot(
        Request::builder()
            .uri("/api/v1/admin/photos")
            .header("authorization", format!("Bearer {access}"))
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "with token → 200");

    // 5) logout → 黑名单写入
    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/v1/auth/logout")
            .header("authorization", format!("Bearer {access}"))
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6) 同 token 再访问 admin/photos 必 401（黑名单生效）
    let resp = app.clone().oneshot(
        Request::builder()
            .uri("/api/v1/admin/photos")
            .header("authorization", format!("Bearer {access}"))
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "after logout → 401 from blacklist");
}
```

#### 14.5.4 注意事项

- **测试不并发跑**：`#[tokio::test]` 默认 cargo test 多线程；OnceCell 共享 (Router, AppState)，多测试并发会互踩 `gathered_light_test` 库。本 PR-5 仅交付**一个测试函数**避免共享状态污染。后续多测试加 `serial_test = "3"` crate 用 `#[serial]` 串行化。
- **不再用 testcontainers**：见 §20 F-11。复用 dev compose 0 副作用。
- **必须先起 dev compose**：测试连 `127.0.0.1:5432`/`6379`/`9000`；这三个端口必须有 gathered-pg/redis/minio 在监听，否则 `init_once` 立刻 panic 一条**清晰错误**（"run dev-up.sh first"）。
- **migrations 跑两次会失败**：每次 build_app 重建测试库 → 第一次 Migrator::up 干净跑通；OnceCell 保证只跑一次。
- **MinIO bucket 自动创建**：`AppState::init` 会 head_bucket → create_bucket，所以 `gathered-light-test` 会在第一次跑测试时自动出现。

#### 14.5.5 不能动

- `frontend/`、所有 service / handler / dto / repo（PR-1..PR-4 已定型）
- `bootstrap.rs::AppState`、`routes.rs`
- `infra/jwt.rs::Claims`

#### DoD（贴 PR 描述）

```sh
# 0) 前提：dev compose 已起
docker ps --filter "name=gathered-" --format "{{.Names}}"
# 期望：gathered-pg / gathered-redis / gathered-minio 三行

# 1) 静态：测试文件落盘
ls crates/cms-api/tests/auth_flow.rs crates/cms-api/tests/common/mod.rs

# 2) 编译
cargo check --workspace --all-targets --tests 2>&1 | tail -3

# 3) 跑测试前快照容器
docker ps --format "{{.Names}}" | sort > /tmp/before-test.txt

# 4) 跑测试
cargo test -p cms-api --test auth_flow -- --nocapture
# 期望：test login_admin_logout_blacklist_full_loop ... ok
#       test result: ok. 1 passed; 0 failed

# 5) ★ 反作弊：跑测试后容器数必须**完全一样**，0 个 testcontainers 残留
docker ps --format "{{.Names}}" | sort > /tmp/after-test.txt
diff /tmp/before-test.txt /tmp/after-test.txt
# 期望：无输出（完全相同）

docker ps -a --filter "label=org.testcontainers.managed-by=testcontainers" | wc -l
# 期望：1（只有表头），即 0 个 testcontainers 容器

# 6) 验证测试库被独立创建
docker exec gathered-pg psql -U postgres -l | grep gathered_light_test
# 期望：1 行
```

---

## 15. 缓存失效矩阵

> 当 service 层做写操作时，**必须**按下表对应失效 Redis keys。漏一行就是一条数据陈旧 bug。

| 写操作 | 立即失效（DEL / UNLINK） | 备注 |
| --- | --- | --- |
| `photo create / update / delete / reset` | `cms:photo:list:*`（SCAN+UNLINK）+ `cms:photo:detail:{slug}:lang=*` | 所有 list 缓存粒度太细，按前缀清；详情按 slug 精确清 |
| `photo cycle-privacy` | 同上 | 私有性变化影响公开列表 |
| `category create / update / delete` | `cms:taxonomy:categories:lang=*` + `cms:photo:list:*` | 分类树 + 列表都受影响 |
| `tag create / update / delete` | `cms:taxonomy:tags:lang=*` | 标签云 |
| `auth logout` | 写入 `auth:blacklist:{jti}` TTL=剩余 exp | 不是失效，是新增 |
| `auth logout-all` | `SET auth:user-rev:{user_id} <epoch>` TTL=refresh_ttl | 中间件比对 `claims.iat < epoch` 即拒 |
| `media complete` 入库 | （不需要清缓存） | 列表只展示 `status='ready'`，新 asset 等 worker 转完才会出现 |
| `media variant ready`（worker 写） | `cms:photo:detail:*`（如该 asset 关联了 photo） | photo 详情可能引用 thumb/full 路径 |

**实现要求**：所有失效逻辑放在 `services::photo_service::invalidate_listing_caches(state)` 等命名函数里，repo 层不参与。

---

## 16. 数据校验规则（输入边界）

> 接力代理实现 DTO / handler 时**必须**校验以下项，使用 `validator` crate 派生宏。

| 字段 | 类型 / 约束 |
| --- | --- |
| `email` | RFC 5322（`#[validate(email)]`） |
| `password`（登录入参） | length 8..=128（`#[validate(length(min = 8, max = 128))]`） |
| `slug`（photo / category / tag） | regex `^[a-z0-9-]{1,80}$` |
| `title.{zh,en}` | length 1..=200，前后 `trim` |
| `location.{zh,en}` | length 0..=200 |
| `taken_at_label` | length 0..=32（保留前端"2024.05"格式自由度） |
| `cat` / `category_slug` | 必须存在于 `categories` 表（service 层校验） |
| `privacy` | enum `Privacy`（`#[serde(rename_all = "lowercase")]` 已强制） |
| `passcode`（解锁入参） | length 1..=64 |
| 上传 mime | 白名单：`image/jpeg` `image/png` `image/webp` `image/avif` |
| 上传 byte_size | `1..=config.s3.upload_max_bytes`（默认 25 MiB） |
| 文件名 | 仅保留 `[A-Za-z0-9._-]`，空格转 `-`；其它 unicode 丢弃后留 `upload.jpg` 兜底 |

校验失败 → `AppError::Validation(e.to_string())` → 400 + `code: "BAD_REQUEST"`。

---

## 17. Locked 解锁详解（per-photo passcode）

> 当前实现把 `UNLOCK_PASSCODE` 写死成 `"1234"`（违反 §H-3）。M3 起改为下面的合规版本：

### 17.1 数据模型

- `photos.passcode_hash TEXT NULL`：`argon2id(passcode)`，仅当 `privacy = 'locked'` 时非空。
- 切换到 `public` / `private` 时，service 必须**显式** `UPDATE photos SET passcode_hash = NULL`。

### 17.2 接口

- `POST /api/v1/photos/:slug/unlock`，body `{ "passcode": "..." }`。
- 校验 argon2 → 命中：发一个 **15 min、scope 仅本帧 full 资源** 的短 JWT（`scope: "photo:1234:full"`）。
- 不命中：409 + `code: "WRONG_PASSCODE"`，并 `INCR auth:photo:unlock:rl:{photo_id}:{ip_hash}` TTL 600s；> 10 次拒服务。

### 17.3 取 full 图

- `GET /api/v1/photos/:slug/full` 必须带短 JWT；后端验签后返回 `aws-sdk-s3` 签出的临时 GET URL（TTL 同步 15 min）。
- 这样比直接把 full URL 返给前端更安全（防 URL 被截图后长期泄露）。

### 17.4 demo 数据兼容

`seed_demo` 时把 `design/data.jsx` 中所有 `privacy === "locked"` 的图片都写成 `argon2("1234")`（保留 demo 体验，但走真实 hash 通道）。

---

## 18. cover_url 构造算法

> 前端 `PhotoDto.src` 字段在 PG 接入后**不再是直接 URL**，而是后端按规则拼出来的签名 URL。

### 18.1 选 variant

| 调用场景 | variant | 兜底 |
| --- | --- | --- |
| 列表 / 卡片 | `medium_900` | 没生成则用 `thumb_400`；再没有用 `media_assets` 原图 |
| 详情 / Lightbox | `full_1800` | 同上向下兜 |
| 后台缩略 | `thumb_400` | 同上 |

### 18.2 出 URL

```rust
fn cover_url(state: &AppState, asset_id: i64, want: Variant) -> AppResult<String> {
    let target = media_repo::pick_variant_or_origin(&state.db, asset_id, want).await?;
    state.s3.presign_get(&target.storage_key, Duration::from_secs(3600)).await
    // 1h presigned GET。CDN 接入后改成 CDN 签名（CloudFront/AKAMAI EdgeAuth）
}
```

### 18.3 缓存

- `cover_url` 不缓存进 Redis（签名带签发时刻，缓存复用会过期）。
- 但**列表 DTO 整体可以缓存** —— TTL 取最短的 cover_url 寿命（建议 < 30 min）。

---

## 19. 测试策略

| 层级 | 工具 | 范围 | 何时运行 |
| --- | --- | --- | --- |
| 单元（pure） | `#[test]` + `#[tokio::test]` | `cms-domain`：Privacy/Locale parse；`infra::jwt`：sign/verify roundtrip | 每次 `cargo test` |
| 集成（DB+Redis） | `testcontainers-modules` 0.10+ | login → admin → CRUD → logout 闭环 | 每次 `cargo test` （CI） |
| 契约（API shape） | snapshot OpenAPI（`utoipa` 导出 + `insta` diff） | 防止 PR 不小心改了字段名 | CI |
| E2E（前后端） | Playwright（手动 / 后置） | 走通 design 原型的关键路径（登录、发图、解锁、隐私切换） | 上线前 |

testcontainers 的容器复用（避免每个测试 1-2s 启动开销）：

```rust
static PG: tokio::sync::OnceCell<ContainerAsync<Postgres>> = tokio::sync::OnceCell::const_new();
async fn pg() -> &'static ContainerAsync<Postgres> {
    PG.get_or_init(|| async { Postgres::default().with_tag("18-alpine").start().await.unwrap() }).await
}
```

---

## 20. 禁止模式（FORBIDDEN PATTERNS）

> 每个 PR 在描述中必须**显式声明**没有引入下面任一模式。如果不可避免，必须给出无法避免的理由 + 后续替换 PR 编号。

### F-1：in-memory 原始集合替代 PG

```rust
// 反例 — 严禁
pub photos: Arc<RwLock<Vec<PhotoDto>>>,
pub users:  Arc<Mutex<HashMap<String, User>>>,
```

理由：H-1。多副本部署立刻数据不一致；进程重启即丢。

### F-2：in-memory `HashSet` 替代 Redis 黑名单

```rust
// 反例 — 严禁
pub token_blacklist: Arc<RwLock<HashSet<String>>>,
```

理由：H-2。CLAUDE.md 用户硬要求"必须结合 Redis"。

### F-3：明文密码（任何形式）

```rust
// 反例 — 严禁
if req.password == cfg.admin_password { /* ... */ }
```

```toml
# 反例 — 严禁（config/*.toml 中出现密码）
[jwt]
admin_password = "admin"
```

理由：H-3。

### F-4：Mock 字符串替代 S3 签名

```rust
// 反例 — 严禁
let upload_url = format!("{public_url}?X-Amz-Algorithm=MOCK-HMAC-SHA256");
```

理由：H-4。

### F-5：在 handler 内逐个调 `require_access`

```rust
// 反例 — 严禁
pub async fn list_admin(State(s): State<AppState>, headers: HeaderMap) -> ... {
    auth_service::require_access(&s, &headers).await?;   // ← 每个 admin handler 重复
    /* ... */
}
```

理由：H-7。改用 `route_layer(...)` 一次挂上。

### F-6：handler 内写 SQL / 直接调 redis-client

理由：H-7。SQL 在 repo；redis 在 `infra::redis`；service 编排。

### F-7：`.unwrap()` / `.expect()` / `panic!` 在生产路径

```rust
// 反例 — 严禁
let user = user_repo::find_by_email(...).await.unwrap();
```

理由：H-8。改用 `?` + `AppError`。

### F-8：在仓库里提交 `.env` 或 `keys/*.pem`

理由：H-3。`.gitignore` 必须屏蔽。

### F-9：改前端字段名而不同步 `frontend/`

理由：H-10。前后端是合同关系，单方面改名 = 破坏合同。

### F-10：在 handler 同步处理图片字节

理由：H-5。阻塞 worker 线程，整个进程吞吐崩。

### F-11：用 testcontainers 启独立 PG/Redis 容器跑测试

```toml
# 反例 — 严禁
[dev-dependencies]
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres", "redis"] }
```

```rust
// 反例 — 严禁
let pg = Postgres::default().with_tag("18-alpine").start().await.unwrap();
let redis = Redis::default().with_tag("7-alpine").start().await.unwrap();
```

理由：测试进程异常退出（CI 超时 / kill -9 / 沙箱中断）会留 PG+Redis 孤儿容器，**长达数小时**不释放，docker host 资源占用直线上升。本项目实际碰到过：4 个 testcontainers 孤儿挂了 5 小时。

正解：**复用 dev compose** 的 `gathered-pg` / `gathered-redis` / `gathered-minio`，用独立测试库（`gathered_light_test`）+ Redis DB 15 + bucket `gathered-light-test` 隔离。模板见 §14.5.2。

---

## 21. 冻结决策（FROZEN — 不接受讨论）

下列决策已被业务 / 前端 / 安全 / 合规反复确认，**不接受 PR 中以"重构"、"性能"、"简洁"等理由变动**。如确有强需求，先开 design issue 经业务签字。

| # | 决策 | 反例（严禁） |
| --- | --- | --- |
| Z-1 | 隐私枚举 = `public / locked / private` 三态 | 改成布尔 `is_public` |
| Z-2 | i18n 分两个字段 `{zh, en}`（前端契约） | 用 `i18n: { ... }` 嵌套对象 |
| Z-3 | 列表分页参数 `?page=1&page_size=24` | 改 `?offset=&limit=` |
| Z-4 | 路径前缀 `/api/v1` | 省掉版本号 |
| Z-5 | API 响应 envelope = 直接是 data（**没有** `{code, data, message}` 包一层） | 加一层，会破坏所有前端 |
| Z-6 | 错误响应 envelope = `{ "code": "BAD_REQUEST", "message": "..." }`（顶层） | 嵌到 `error: { ... }` |
| Z-7 | photo 排序默认 `sort_order DESC, taken_at_date DESC` | 用 `created_at DESC` |
| Z-8 | 鉴权 = Bearer + 短 access (15m) + 长 refresh (7d) | 单 token / cookie |
| Z-9 | 数据库 = PG 18，不是 PG 16 / MySQL / SQLite | 任何替代 |
| Z-10 | 图片处理 = 后端异步生成 4 variants（thumb/medium/full/webp） | 前端缩放 / 浏览器自适应 |

---

## 22. 接力速查（给 Codex / 后续 AI 代理的常见踩坑指南）

> 你**很可能**在以下问题上踩坑。读完再写代码可省 30 分钟。

1. **不要用 `axum 0.8`** — 0.8 重做了 `Handler` trait，会让 `State<...>` 提取器报神秘类型错误。本仓 `Cargo.toml` 锁 `axum = "0.7"`。
2. **`figment::Toml::file().nested()` 是坑** — 顶层标量会被当成 profile map 失败解析。本仓已用 `.merge(Toml::file(&path))`（不带 `.nested()`），勿改。
3. **`sea-orm` 1.x 与 `runtime-*` feature 互斥** — 只用 `runtime-tokio-rustls`，别同时开 `runtime-actix-rustls`。
4. **`fred` 9.x API**：`RedisPool::new` 5 参数；`Expiration::EX` 在 `fred::types::Expiration`；用 `set_with_options` 时注意 `XX/NX` 参数顺序。
5. **`jsonwebtoken` 9.x** 的 `Validation::default()` 默认验 `exp`，但**不**验 `iat`——你需要手动在 service 里比对 `auth:user-rev:{uid}`。
6. **`citext`** — PG 18 默认带；某些云托管 PG 不允许 `CREATE EXTENSION`，要走 RDS 控制台开。
6.5. **PG 18 mount 路径**：`docker-compose.yml` 必须挂 `/var/lib/postgresql`（**不是** `/var/lib/postgresql/data`）。从 PG 17 升级或本仓初次拉 PG 18 时挂错路径，容器启动会立即 `Exited (1)` 并报 `(unused mount/volume)`。补救（本仓 bind mount 模式）：`rm -rf data/postgres` 后 `bash scripts/dev-up.sh --reset`；命名卷模式则 `docker volume rm gathered-light_pg18_data`。
7. **edition 2024 + resolver 3** — 别在新增 crate 时把 resolver 降回 "2"。
8. **本仓非 git 仓库** — 不要主动 `git init`；如需版本控制，先问用户。
9. **不要再创建 README.md** —— CLAUDE.md 明确不要 README，除非用户显式要求。
10. **i18n 用 `photo_translations` 子表，不是 JSONB** —— DDL 里两种我都给了选项，最终采用子表（H-1 + 索引友好）。
11. **`Privacy` / `Locale`** 在 `cms-domain` 是强类型；序列化是小写字符串 (`"public"` `"locked"` `"private"` `"zh"` `"en"`)，**不要**改成大写或别名。
12. **`Arc::clone(&state)` 不是性能问题** —— 引用计数原子加，比每次新建 IoC 便宜得多。Spring 同事直觉里"new 不起"在这里反过来：`Clone` 就是免费的。
13. **`.gitignore` 当前不全** —— 在你 PR-1 的同一个 PR 里顺手补回 `.env*` `keys/*` `*.pem` `*.key` `*.log` `.idea/` `.vscode/`。
14. **`Cargo.lock` 必须入仓**（这是 binary crate，不是 lib），但 `target/` 不入。
15. **集成测试不要用 testcontainers** — 见 §20 F-11。一律复用 dev compose 已起的 `gathered-pg`/`gathered-redis`/`gathered-minio`，按 §14.5.2 的隔离方式（独立测试库 + Redis DB 15 + 独立 bucket）。任何 PR 的测试文件中出现 `use testcontainers` 都视为不合格，必须返工。

16. **生产部署用腾讯云 COS（不是 MinIO）+ 带宽红线** — dev 用 MinIO（compose 里那个）；生产**必须**用腾讯云 COS（用户服务器只 10 Mbps 上行，自建对象存储不可行）。COS S3 兼容，`aws-sdk-s3` 客户端代码 100% 复用，只改 4 个 env：
    ```bash
    APP_S3__ENDPOINT=https://cos.ap-shanghai.myqcloud.com   # 不要带 bucket 前缀
    APP_S3__REGION=ap-shanghai
    APP_S3__BUCKET=gathered-light-1234567890                  # COS 命名约定 <name>-<APPID>
    APP_S3__ACCESS_KEY=<SecretId>
    APP_S3__SECRET_KEY=<SecretKey>
    ```
    **带宽红线**：image worker（PR-M2-2 引入）会"下载原图 → 本地解码 → 上传 4 个变体"，单张 5 MB 原图约消耗 5x 上行流量、6 秒带宽时间。低频上传（< 100 张/天）OK。高频请考虑改用腾讯 CI（Cloud Infinite）让 COS 自己出缩略图——见 §10 路线图未来项。
    其它路径（presigned PUT 浏览器直传 / presigned GET 浏览器直读）**不**走我们服务器带宽，无影响。

---

## 24. M2 真上传链路（M2 第一砍 → PR-M2-1）

> **目标**：让 admin 能真上传图片到 MinIO（生产同 S3）。本 PR 只做"签 + 收"两步，**不做** 图片处理（缩略图等），那是 PR-M2-2。
>
> **前提**：M1 已 ✅；docker MinIO 容器已起或会起；`media_assets` 表已存在（PR-2 建好）。

### 24.1 PR-M2-1：aws-sdk-s3 真签名 + complete 回调

#### 24.1.1 改动文件清单（12 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `Cargo.toml`（workspace） | 加 `aws-sdk-s3 = "1"`、`aws-config = { version = "1", features = ["behavior-version-latest"] }`、`aws-credential-types = "1"` |
| 2 | `crates/cms-api/Cargo.toml` | 引入上述三个 |
| 3 | `crates/cms-api/src/config.rs` | `S3Cfg` 加 `access_key: String` + `secret_key: String` 字段（**无 default**，必须通过 env 设） |
| 4 | `.env.example` | 加 `APP_S3__ACCESS_KEY=minioadmin` + `APP_S3__SECRET_KEY=minioadmin` |
| 5 | `crates/cms-api/src/bootstrap.rs` | `AppState` 加 `pub s3: aws_sdk_s3::Client`；`init` 中按 §24.1.3 装配 + 自动 `head_bucket` / `create_bucket` |
| 6 | `crates/cms-api/src/repositories/media_repo.rs`（**新建**） | `insert_ready(db, asset)` —— 入库 media_assets（status='ready'，PR-M2-2 才改 'pending'） |
| 7 | `crates/cms-api/src/repositories/mod.rs` | 加 `pub mod media_repo;` |
| 8 | `crates/cms-api/src/dto/media_dto.rs` | 加 `CompleteReq` + `CompleteResp`（保留现有 `PresignReq` / `PresignResp` 形状） |
| 9 | `crates/cms-api/src/services/media_service.rs` | `presign_upload` 用真 `aws-sdk-s3` 签 PUT；新增 `complete_upload`（HEAD object 校验 → 落库） |
| 10 | `crates/cms-api/src/handlers/media_handler.rs` | 新增 `complete` handler |
| 11 | `crates/cms-api/src/routes.rs` | `protected_api` 加 `POST /admin/media/complete` |
| 12 | `scripts/dev-up.sh` | `docker compose up -d postgres redis` 改成 `postgres redis minio`；`mkdir -p ./data/minio` |

#### 24.1.2 路由调整说明

当前 `routes.rs` 把 `/media/presign` 挂在 `protected_api` 下作为 `/api/v1/media/presign`。本 PR 改一下命名一致性：

- `/api/v1/media/presign` → **`/api/v1/admin/media/presign`**（更清晰，admin 专属）
- 新增 **`/api/v1/admin/media/complete`**

前端 `client.ts` 的 `presign(...)` 调用路径同步从 `/api/v1/media/presign` 改成 `/api/v1/admin/media/presign`（H-10：路径有变就同步前端）。

#### 24.1.3 S3 客户端装配（MinIO 兼容）

```rust
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_s3::{Client as S3Client, config::Region};

async fn init_s3(cfg: &S3Cfg) -> anyhow::Result<S3Client> {
    let creds = Credentials::new(
        cfg.access_key.clone(),
        cfg.secret_key.clone(),
        None, None, "gathered-light-static",
    );
    let aws_cfg = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(cfg.region.clone()))
        .endpoint_url(cfg.endpoint.clone())
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .load().await;
    let s3_cfg = aws_sdk_s3::config::Builder::from(&aws_cfg)
        .force_path_style(true)            // ★ MinIO 必须 path-style
        .build();
    let client = S3Client::from_conf(s3_cfg);

    // 自动 head_bucket / create_bucket
    match client.head_bucket().bucket(&cfg.bucket).send().await {
        Ok(_) => tracing::info!(bucket = %cfg.bucket, "s3 bucket exists"),
        Err(e) if e.into_service_error().is_not_found() => {
            client.create_bucket().bucket(&cfg.bucket).send().await
                .map_err(|e| anyhow::anyhow!("create bucket: {e}"))?;
            tracing::info!(bucket = %cfg.bucket, "s3 bucket created");
        }
        Err(e) => anyhow::bail!("head_bucket failed: {e}"),
    }
    Ok(client)
}
```

`AppState::init` 在 PG/Redis 之后调 `init_s3(&cfg.s3).await?`。

#### 24.1.4 presign_upload 真实现

```rust
use aws_sdk_s3::presigning::PresigningConfig;

pub async fn presign_upload(state: &AppState, req: PresignReq) -> AppResult<PresignResp> {
    // 1. 校验 mime + size 上限
    if !req.mime_type.starts_with("image/") {
        return Err(AppError::Validation("only image uploads".into()));
    }
    let max = state.config.s3.upload_max_bytes;
    if req.byte_size == 0 || req.byte_size > max {
        return Err(AppError::Validation(format!("file size 1..={max}")));
    }

    // 2. 生成 storage_key（按月分目录，UUID 防冲突）
    let safe = sanitize_file_name(&req.file_name);
    let storage_key = format!("uploads/{}/{}-{}",
        chrono::Utc::now().format("%Y/%m"),
        uuid::Uuid::new_v4(),
        safe);

    // 3. 真签 15 min PUT URL
    let presigning = PresigningConfig::expires_in(std::time::Duration::from_secs(900))
        .map_err(|e| AppError::Internal(Box::leak(format!("presign: {e}").into_boxed_str())))?;
    let req_built = state.s3.put_object()
        .bucket(&state.config.s3.bucket)
        .key(&storage_key)
        .content_type(&req.mime_type)
        .content_length(req.byte_size as i64)
        .presigned(presigning).await
        .map_err(|e| { tracing::error!(error = ?e, "presign"); AppError::Internal("presign failed") })?;

    let upload_url = req_built.uri().to_string();
    let public_url = format!("{}/{}/{}",
        state.config.s3.endpoint.trim_end_matches('/'),
        state.config.s3.bucket,
        storage_key);

    let mut headers = std::collections::BTreeMap::new();
    headers.insert("content-type".into(), req.mime_type.clone());

    Ok(PresignResp {
        method: "PUT",
        upload_url,
        public_url,
        storage_key,
        headers,
        max_bytes: max,
        expires_in: 900,
    })
}

fn sanitize_file_name(input: &str) -> String {
    let s: String = input.chars().filter(|c|
        c.is_ascii_alphanumeric() || matches!(c, '.'|'-'|'_')
    ).collect();
    if s.is_empty() { "upload.jpg".into() } else { s }
}
```

#### 24.1.5 complete_upload 实现

```rust
pub async fn complete_upload(state: &AppState, req: CompleteReq) -> AppResult<CompleteResp> {
    // 1. HEAD object —— 不信前端，问 S3 拿权威元数据
    let head = state.s3.head_object()
        .bucket(&state.config.s3.bucket)
        .key(&req.storage_key)
        .send().await
        .map_err(|e| {
            tracing::warn!(error = ?e, key = %req.storage_key, "head_object failed");
            AppError::NotFound
        })?;

    let mime_type = head.content_type().unwrap_or("application/octet-stream").to_string();
    let byte_size = head.content_length().unwrap_or(0);

    // 2. 落库（status='ready'，PR-M2-2 引入 worker 后改 'pending'）
    let asset = media_repo::insert_ready(&state.db, NewAsset {
        storage_key: req.storage_key.clone(),
        mime_type,
        byte_size,
    }).await?;

    Ok(CompleteResp {
        asset_id: asset.id,
        storage_key: req.storage_key,
        public_url: format!("{}/{}/{}",
            state.config.s3.endpoint.trim_end_matches('/'),
            state.config.s3.bucket,
            asset.storage_key),
    })
}
```

#### 24.1.6 DTO 增量

```rust
// dto/media_dto.rs 追加
#[derive(Debug, Deserialize)]
pub struct CompleteReq {
    pub storage_key: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteResp {
    pub asset_id: i64,
    pub storage_key: String,
    pub public_url: String,
}
```

#### 24.1.7 media_repo 最小集

```rust
use cms_entity::media_assets::{ActiveModel, Column, Entity, Model};
use sea_orm::*;

pub struct NewAsset {
    pub storage_key: String,
    pub mime_type: String,
    pub byte_size: i64,
}

pub async fn insert_ready(db: &DatabaseConnection, a: NewAsset) -> Result<Model, DbErr> {
    ActiveModel {
        storage_key: Set(a.storage_key),
        mime_type: Set(a.mime_type),
        byte_size: Set(Some(a.byte_size)),
        status: Set("ready".into()),
        ..Default::default()
    }.insert(db).await
}
```

#### 24.1.8 dev-up.sh 改一行

```diff
- docker compose up -d postgres redis
+ docker compose up -d postgres redis minio
```

加在 `mkdir -p` 那行：

```diff
- mkdir -p ./data/postgres ./data/redis
+ mkdir -p ./data/postgres ./data/redis ./data/minio
```

#### 24.1.9 不能动

- `frontend/` 任何文件（前端的 upload 按钮已经调 `api.presign`，本 PR 只确保后端真返签名 URL；前端文件直传 + 自动 complete UI 留给 PR-M2-3）
- `photo_service` / `auth_service` / `user_service`（如有）业务代码
- `media_assets` 表 schema（PR-M2-2 才会引入 'pending' / worker）
- `routes.rs` 公开路由分组（仅在 protected_api 增 `/admin/media/complete`，并把 `/media/presign` 改名 `/admin/media/presign`）

#### DoD（贴 PR 描述）

```sh
# 0) 前提：起完整依赖（含 MinIO）
bash scripts/dev-up.sh --skip-dod    # 应当看到 minio 也起来了

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3
# 期望：Finished `dev` profile

# 2) 起 cms-api，启动日志应显示 bucket 自动创建（首次）或 exists（后续）
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api > /tmp/cms.log 2>&1 &
PID=$!; sleep 5
grep -E "s3 bucket" /tmp/cms.log
# 期望：1 行 "s3 bucket created" 或 "s3 bucket exists"

# 3) 拿 token
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

# 4) presign（路径已迁到 /admin/media/presign）
PRES=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"file_name":"test.jpg","mime_type":"image/jpeg","byte_size":12345}' \
  http://127.0.0.1:18080/api/v1/admin/media/presign)
echo "$PRES" | jq
URL=$(echo "$PRES" | jq -r .upload_url)
KEY=$(echo "$PRES" | jq -r .storage_key)
echo "got upload_url: ${URL:0:80}..."
echo "$URL" | grep -q "X-Amz-Signature" && echo "✅ 真 v4 签名" || echo "❌ 不是真签名"

# 5) 真 PUT 一个测试文件到 MinIO
echo "fake-jpeg-content-$(date +%s)" > /tmp/test.jpg
curl -s -o /dev/null -w "PUT to S3: HTTP %{http_code}\n" \
  -X PUT -H "content-type: image/jpeg" --data-binary @/tmp/test.jpg "$URL"
# 期望：200

# 6) MinIO 真有这个文件
docker exec gathered-minio mc alias set local http://localhost:9000 minioadmin minioadmin > /dev/null
docker exec gathered-minio mc ls local/gathered-light/ -r | grep "$(basename $KEY)"
# 期望：1 行命中

# 7) complete 回调 → media_assets 入库
COMP=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d "{\"storage_key\":\"$KEY\"}" \
  http://127.0.0.1:18080/api/v1/admin/media/complete)
echo "$COMP" | jq
ASSET_ID=$(echo "$COMP" | jq -r .asset_id)
[ -n "$ASSET_ID" ] && [ "$ASSET_ID" != "null" ] && echo "✅ asset id=$ASSET_ID" || echo "❌"

# 8) 数据库真有这条 asset
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT id, storage_key, mime_type, byte_size, status FROM media_assets WHERE id=$ASSET_ID;"
# 期望：1 行；status=ready

# 9) 校验 mime 不通必 422
curl -s -o /dev/null -w "wrong mime: %{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"file_name":"x.exe","mime_type":"application/x-msdownload","byte_size":100}' \
  http://127.0.0.1:18080/api/v1/admin/media/presign
# 期望：400 (BAD_REQUEST，"only image uploads")

# 10) 超尺寸必 422
curl -s -o /dev/null -w "too big: %{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"file_name":"x.jpg","mime_type":"image/jpeg","byte_size":99999999}' \
  http://127.0.0.1:18080/api/v1/admin/media/presign
# 期望：400

# 11) 无 token 必 401
curl -s -o /dev/null -w "no token: %{http_code}\n" -X POST \
  -H 'content-type: application/json' \
  -d '{"file_name":"x.jpg","mime_type":"image/jpeg","byte_size":100}' \
  http://127.0.0.1:18080/api/v1/admin/media/presign
# 期望：401

# 12) 收工
kill -INT $PID; wait $PID 2>/dev/null
```

> **预期回归**：前端 admin form 的 upload 按钮调用路径要更新（`/api/v1/admin/media/presign`）才能正常工作；frontend `client.ts` 改一行就行（**算 H-10 同步改前端**，不是破坏契约）。
>
> **延后到 PR-M2-2**：image worker 出 thumb/medium/full/webp variants；status 改成 'pending' 触发流转。

---

### 24.2 PR-M2-2：image worker 出 4 variants

> **目标**：tokio 后台 worker 监听 `media_assets.status = 'pending'`，下载原图 → image crate 解码 → 生成 4 种尺寸/格式 variants → 回传 S3 → 写 `media_variants` → 更新 `status = 'ready'`。
>
> **副作用**：`complete_upload` 改为写 `status = 'pending'`（PR-M2-1 写的 'ready' 在本 PR 改）。worker 每 3 秒轮询一次。
>
> **前提**：PR-M2-1 已 ✅；MinIO 在线；可上传一张真 JPEG 入库。

#### 24.2.1 改动文件清单（7 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `Cargo.toml`（workspace） | 加 `image = { version = "0.25", features = ["jpeg", "png", "webp"] }`；如 image 0.25 不带 WebP 编码，再加独立 `webp = "0.3"` |
| 2 | `crates/cms-api/Cargo.toml` | 引入 `image`（必要时含 `webp`） |
| 3 | `crates/cms-api/src/repositories/media_repo.rs` | 扩展：`claim_pending(db, n)` / `update_status(db, id, status)` / `insert_variant(db, asset_id, variant, key, w, h)` / `find_variant_by_key(db, key)` |
| 4 | `crates/cms-api/src/services/media_service.rs::complete_upload` | 状态从 `"ready"` 改为 `"pending"`（让 worker 接管） |
| 5 | `crates/cms-api/src/workers/image_processor.rs`（**新建**） | 完整 worker（轮询 + 处理 + 错误兜底） |
| 6 | `crates/cms-api/src/workers/mod.rs` | 加 `pub mod image_processor;` |
| 7 | `crates/cms-api/src/main.rs` | `bootstrap` 后追加 `tokio::spawn(workers::image_processor::run(state.clone()));` |

#### 24.2.2 4 个 variants 规格

| variant | 长边 | 格式 | 质量 | storage_key 命名 |
| --- | --- | --- | --- | --- |
| `thumb_400` | 400 px | JPEG | 75 | `variants/{asset_id}/thumb_400.jpg` |
| `medium_900` | 900 px | JPEG | 80 | `variants/{asset_id}/medium_900.jpg` |
| `full_1800` | 1800 px | JPEG | 85 | `variants/{asset_id}/full_1800.jpg` |
| `webp_900` | 900 px | WebP | 80 | `variants/{asset_id}/webp_900.webp` |

resize 算法：`image::imageops::FilterType::Lanczos3`，**保持纵横比**——传入"目标长边"，按 `max(w, h)` 等比缩到目标。短边小于目标长边时**不放大**（避免无谓压缩失真）。

#### 24.2.3 worker 流程

```rust
// crates/cms-api/src/workers/image_processor.rs
use std::time::Duration;
use crate::bootstrap::AppState;

pub async fn run(state: AppState) {
    tracing::info!("🖼  image_processor worker started");
    loop {
        match tick(&state).await {
            Ok(processed) if processed == 0 => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            Ok(n) => tracing::debug!(processed = n, "image_processor tick"),
            Err(e) => {
                tracing::error!(error = ?e, "image_processor tick failed; backing off");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

#[tracing::instrument(skip(state), fields(asset_id))]
async fn tick(state: &AppState) -> anyhow::Result<usize> {
    // 1. claim N pending（原子：UPDATE ... WHERE status='pending' RETURNING *）
    let claimed = media_repo::claim_pending(&state.db, 5).await?;
    let n = claimed.len();
    for asset in claimed {
        tracing::Span::current().record("asset_id", asset.id);
        if let Err(e) = process_one(state, &asset).await {
            tracing::error!(error = ?e, asset_id = asset.id, "process failed");
            // 仍在 'processing'，置 'failed'
            let _ = media_repo::update_status(&state.db, asset.id, "failed").await;
        } else {
            media_repo::update_status(&state.db, asset.id, "ready").await?;
        }
    }
    Ok(n)
}

async fn process_one(state: &AppState, asset: &Model) -> anyhow::Result<()> {
    // 1. 从 S3 下载原图
    let body = state.s3.get_object()
        .bucket(&state.config.s3.bucket)
        .key(&asset.storage_key)
        .send().await?;
    let bytes = body.body.collect().await?.into_bytes();

    // 2. 解码
    let img = image::load_from_memory(&bytes)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    // 3. 4 变体
    for v in [
        Variant { name: "thumb_400",  long_side: 400,  fmt: Fmt::Jpeg(75) },
        Variant { name: "medium_900", long_side: 900,  fmt: Fmt::Jpeg(80) },
        Variant { name: "full_1800",  long_side: 1800, fmt: Fmt::Jpeg(85) },
        Variant { name: "webp_900",   long_side: 900,  fmt: Fmt::Webp(80) },
    ] {
        let resized = resize_keep_ratio(&img, v.long_side);
        let (w, h) = (resized.width(), resized.height());
        let (encoded, ext) = encode(resized, &v.fmt)?;
        let key = format!("variants/{}/{}.{}", asset.id, v.name, ext);

        state.s3.put_object()
            .bucket(&state.config.s3.bucket)
            .key(&key)
            .content_type(v.fmt.mime())
            .body(encoded.into())
            .send().await?;

        media_repo::insert_variant(&state.db, asset.id, v.name, &key, w as i32, h as i32).await?;
    }

    tracing::info!(asset_id = asset.id, orig_w, orig_h, "variants generated");
    Ok(())
}
```

辅助类型与函数：

```rust
struct Variant { name: &'static str, long_side: u32, fmt: Fmt }

enum Fmt { Jpeg(u8), Webp(u8) }
impl Fmt {
    fn mime(&self) -> &'static str {
        match self { Fmt::Jpeg(_) => "image/jpeg", Fmt::Webp(_) => "image/webp" }
    }
}

fn resize_keep_ratio(img: &image::DynamicImage, long_side: u32) -> image::DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w.max(h) <= long_side { return img.clone(); }   // 不放大
    img.resize(long_side, long_side, image::imageops::FilterType::Lanczos3)
}

fn encode(img: image::DynamicImage, fmt: &Fmt) -> anyhow::Result<(Vec<u8>, &'static str)> {
    let mut buf = Vec::new();
    match fmt {
        Fmt::Jpeg(q) => {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, *q);
            enc.encode_image(&img)?;
            Ok((buf, "jpg"))
        }
        Fmt::Webp(q) => {
            // image 0.25 的 WebP encoder 走 image-webp 子 crate；如不可用，
            // fallback 到独立 webp crate：let enc = webp::Encoder::from_image(&img)?;
            //                              let mem = enc.encode(*q as f32);
            //                              buf = mem.to_vec();
            let mut enc = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
            enc.encode(img.as_bytes(), img.width(), img.height(), img.color().into())?;
            Ok((buf, "webp"))
        }
    }
}
```

> ⚠️ image 0.25 的 WebPEncoder 在某些版本只支持 lossless（不接 quality 参数）。如果遇到接口不匹配，**改用独立 `webp = "0.3"` crate**（注释里已给 fallback 代码）。这是 §22 接力速查可以加一条的踩坑点。

#### 24.2.4 media_repo 增量

```rust
/// 原子认领：把 N 个 pending 改成 processing 并返回。
/// 后续支持多 worker 时可以加 FOR UPDATE SKIP LOCKED。
pub async fn claim_pending(db: &DatabaseConnection, n: u64) -> Result<Vec<Model>, DbErr> {
    // SeaORM 没有原生 RETURNING + 批量 update，用事务兜：
    let txn = db.begin().await?;
    let candidates: Vec<Model> = Entity::find()
        .filter(Column::Status.eq("pending"))
        .order_by_asc(Column::Id)
        .limit(n)
        .all(&txn).await?;
    for m in &candidates {
        let mut a: ActiveModel = m.clone().into();
        a.status = Set("processing".into());
        a.update(&txn).await?;
    }
    txn.commit().await?;
    Ok(candidates)
}

pub async fn update_status(db: &DatabaseConnection, id: i64, status: &str) -> Result<(), DbErr> {
    let m = Entity::find_by_id(id).one(db).await?
        .ok_or(DbErr::RecordNotFound("media_asset".into()))?;
    let mut a: ActiveModel = m.into();
    a.status = Set(status.into());
    a.update(db).await?;
    Ok(())
}

pub async fn insert_variant(
    db: &DatabaseConnection,
    asset_id: i64, variant: &str, storage_key: &str,
    width: i32, height: i32,
) -> Result<(), DbErr> {
    use cms_entity::media_variants::ActiveModel as VAM;
    use cms_entity::media_variants::*;
    VAM {
        asset_id: Set(asset_id),
        variant: Set(variant.into()),
        storage_key: Set(storage_key.into()),
        width: Set(width),
        height: Set(height),
        ..Default::default()
    }.insert(db).await?;
    Ok(())
}
```

#### 24.2.5 main.rs 改一处

```rust
// 在 axum::serve 之前追加：
tokio::spawn(cms_api::workers::image_processor::run(state.clone()));
```

#### 24.2.6 不能动

- `frontend/` 任何文件（cover_url 改用 variant URL 是 PR-M2-3 的事）
- `photo_service` / `auth_service` / `user_service`（如有）
- `routes.rs` 路由（worker 不暴露 HTTP）
- `media_handler` 函数体
- `media_assets` 表 schema（`media_variants` 表 PR-2 已建好）

#### DoD（贴 PR 描述）

```sh
# 0) 前提：bash scripts/dev-up.sh --skip-dod 起好 PG/Redis/MinIO
docker ps --filter "name=gathered-" --format "{{.Names}} {{.Status}}"

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3
# 期望：Finished `dev` profile

# 2) 起 cms-api，确认 worker 启动
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info,cms_api::workers=debug \
  cargo run -p cms-api > /tmp/cms.log 2>&1 &
PID=$!; sleep 5
grep "image_processor worker started" /tmp/cms.log
# 期望：1 行

# 3) 拿 token + 准备一张真 JPEG（curl 一张可重现的 1200x800 测试图）
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)
curl -sL -o /tmp/t.jpg "https://picsum.photos/seed/m22/1200/800"
SIZE=$(stat -f%z /tmp/t.jpg 2>/dev/null || stat -c%s /tmp/t.jpg)
echo "test image size: $SIZE bytes"
file /tmp/t.jpg | head -1   # 期望：JPEG image data

# 4) presign + PUT + complete
PRES=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d "{\"file_name\":\"m2-test.jpg\",\"mime_type\":\"image/jpeg\",\"byte_size\":$SIZE}" \
  http://127.0.0.1:18080/api/v1/admin/media/presign)
URL=$(echo "$PRES" | jq -r .upload_url)
KEY=$(echo "$PRES" | jq -r .storage_key)

curl -s -o /dev/null -w "PUT: %{http_code}\n" \
  -X PUT -H "content-type: image/jpeg" --data-binary @/tmp/t.jpg "$URL"

COMP=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d "{\"storage_key\":\"$KEY\"}" \
  http://127.0.0.1:18080/api/v1/admin/media/complete)
ASSET_ID=$(echo "$COMP" | jq -r .asset_id)
echo "asset_id = $ASSET_ID"

# 5) 立即查 status 应是 pending
docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
  -c "SELECT status FROM media_assets WHERE id=$ASSET_ID;"
# 期望：pending

# 6) 等 worker 处理（最多 15 秒）
for i in $(seq 1 15); do
  st=$(docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
    -c "SELECT status FROM media_assets WHERE id=$ASSET_ID;" | tr -d ' ')
  echo "[t=${i}s] status = $st"
  [ "$st" = "ready" ] && break
  [ "$st" = "failed" ] && { echo "❌ worker failed"; break; }
  sleep 1
done

# 7) media_variants 必须 4 行
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT variant, width, height FROM media_variants WHERE asset_id=$ASSET_ID ORDER BY variant;"
# 期望：thumb_400 / medium_900 / full_1800 / webp_900 共 4 行；尺寸符合长边规则

# 8) MinIO 真有 4 个 variant 文件
docker exec gathered-minio mc alias set local http://localhost:9000 minioadmin minioadmin > /dev/null
docker exec gathered-minio mc ls local/gathered-light/variants/$ASSET_ID/
# 期望：4 个文件 thumb_400.jpg / medium_900.jpg / full_1800.jpg / webp_900.webp

# 9) tracing 日志能看到 variants generated
grep "variants generated" /tmp/cms.log
# 期望：1 行 + asset_id + orig_w + orig_h

# 10) 失败兜底（可选）：用一个非图片测试
PRES2=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"file_name":"bad.jpg","mime_type":"image/jpeg","byte_size":100}' \
  http://127.0.0.1:18080/api/v1/admin/media/presign)
URL2=$(echo "$PRES2" | jq -r .upload_url)
KEY2=$(echo "$PRES2" | jq -r .storage_key)
echo "this is not a real jpeg" | curl -s -o /dev/null -X PUT \
  -H "content-type: image/jpeg" --data-binary @- "$URL2"
COMP2=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d "{\"storage_key\":\"$KEY2\"}" \
  http://127.0.0.1:18080/api/v1/admin/media/complete)
BAD_ID=$(echo "$COMP2" | jq -r .asset_id)
sleep 5
docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
  -c "SELECT status FROM media_assets WHERE id=$BAD_ID;"
# 期望：failed（worker 解码失败兜住，没 crash）

# 11) 收工
kill -INT $PID; wait $PID 2>/dev/null
```

> **预期回归**：photo 列表的 `src` 字段还是 `media_assets.storage_key`（直接外部 URL），不会自动用新生成的 thumb_400。**这是 PR-M2-3 的工作**——cover_url 算法改用 variant 的 presigned GET。
>
> **可观测性**：worker 启动会打 `🖼  image_processor worker started`；每次处理完打 `variants generated`；空闲时 debug 级 `processed = N tick`。
>
> **冷知识**：本 PR 是项目第一个 `tokio::spawn` 长任务，AGENTS.md 第 2 章的 "Background workers via tokio::spawn" 第一次落地。

---

### 24.3 PR-M2-3：cover_url 走 variant + 前端文件直传 UI（M2 收官）

> **目标**：
> 1. 后端 `assemble_dto` 改用真"封面 URL 算法"：外部 URL 原样返；S3 key 选最佳 variant 签 GET URL
> 2. `photo_service::create` 复用已存在的 asset（按 storage_key），避免上传后再创建 photo 时重复落库
> 3. 前端 admin form：文件直传 UI（presign → PUT → complete → 自动填 src）
>
> **前提**：PR-M2-1 / PR-M2-2 已 ✅；MinIO 在线；至少有 1 张 `status=ready` 的新上传 asset（带 4 个 variants）可联调。

#### 24.3.1 改动文件清单（8 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/repositories/media_repo.rs` | 新增 `find_variants_for_assets(db, ids) -> HashMap<i64, Vec<Variant>>`、`find_by_storage_key(db, key) -> Option<Asset>` |
| 2 | `crates/cms-api/src/repositories/photo_repo.rs::insert_asset` | 改为 **upsert** 语义：先 `find_by_storage_key`，命中则复用，未命中再 insert（避免重复落库） |
| 3 | `crates/cms-api/src/services/photo_service.rs` | 新增 `cover_url_for(state, asset, variants) -> AppResult<String>` 算法；`assemble_dto` / `find_dto` / `list_public` / `list_admin` 全部改走 cover_url |
| 4 | `crates/cms-api/src/services/media_service.rs` | 抽出 `presign_get(s3, cfg, key, ttl) -> AppResult<String>` 给 cover_url 复用 |
| 5 | `frontend/src/types.ts` | 新增 `CompleteRequest` / `CompleteResponse` 类型 |
| 6 | `frontend/src/api/client.ts` | 新增 `api.complete(payload)`；保持 `api.presign` 不变 |
| 7 | `frontend/src/components/AdminApp.tsx` | 替换"申请上传 URL"按钮为**文件选择器 + 自动上传**；保留手动 URL 输入框（供外部 URL 场景） |
| 8 | `frontend/src/i18n.ts` | 加 `uploading / uploadFailed / chooseFile` 等 i18n key |

#### 24.3.2 cover_url 算法（最关键）

```rust
// services/photo_service.rs
async fn cover_url_for(
    state: &AppState,
    asset: &media_assets::Model,
    variants: &[media_variants::Model],
) -> AppResult<String> {
    // 1. 外部 URL（demo / 历史数据）：原样返
    if asset.storage_key.starts_with("http://") || asset.storage_key.starts_with("https://") {
        return Ok(asset.storage_key.clone());
    }
    // 2. S3 key：选最佳 variant，没有就回退到原图，签 1h presigned GET
    let pick = variants.iter().find(|v| v.variant == "medium_900")
        .or_else(|| variants.iter().find(|v| v.variant == "thumb_400"))
        .map(|v| v.storage_key.as_str())
        .unwrap_or(asset.storage_key.as_str());
    media_service::presign_get(&state.s3, &state.config.s3, pick, 3600).await
}
```

`presign_get` 抽到 `media_service`：

```rust
// services/media_service.rs
pub async fn presign_get(
    s3: &S3Client, cfg: &S3Cfg, key: &str, ttl_secs: u64,
) -> AppResult<String> {
    let presigning = PresigningConfig::expires_in(Duration::from_secs(ttl_secs))
        .map_err(|_| AppError::Internal("presign cfg"))?;
    let req = s3.get_object()
        .bucket(&cfg.bucket).key(key)
        .presigned(presigning).await
        .map_err(|e| { tracing::error!(error = ?e, key, "presign GET"); AppError::Internal("presign GET") })?;
    Ok(req.uri().to_string())
}
```

> 注：presign 是**本地签名**（不是网络调用），22 张图签 22 次 < 10ms。无需缓存。

#### 24.3.3 list_public / list_admin 改 batch 取 variants

```rust
// services/photo_service.rs
pub async fn list_public(state: &AppState, q: PhotoQuery) -> AppResult<Vec<PhotoDto>> {
    let fulls = photo_repo::list_public(&state.db, q.category.as_deref()).await.map_err(db_err)?;
    let asset_ids: Vec<i64> = fulls.iter().map(|f| f.asset.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids)
        .await.map_err(db_err)?;

    let mut out = Vec::with_capacity(fulls.len());
    for f in fulls {
        let v = variants_map.get(&f.asset.id).cloned().unwrap_or_default();
        let cover = cover_url_for(state, &f.asset, &v).await?;
        out.push(assemble_dto_with_cover(f, cover));
    }
    Ok(out)
}
```

`assemble_dto_with_cover` ≈ 现有 `assemble_dto` 但 `src: cover` 而不是 `full.asset.storage_key`。

#### 24.3.4 photo_repo::upsert_asset

```rust
// repositories/photo_repo.rs
async fn insert_asset(
    txn: &DatabaseTransaction,
    input: &NewPhoto,
) -> Result<media_assets::Model, DbErr> {
    // ★ 复用：如果同 storage_key 已经入库（前端先 complete 再 create photo 的场景），
    // 不要再插一份；直接拿来挂 photo.primary_asset_id。
    if let Some(existing) = media_assets::Entity::find()
        .filter(media_assets::Column::StorageKey.eq(&input.src_url))
        .one(txn).await?
    {
        return Ok(existing);
    }
    // 否则按现有逻辑插入（外部 URL 场景：storage_key 是 https://...）
    media_assets::ActiveModel {
        storage_key: Set(input.src_url.clone()),
        mime_type: Set(input.mime_type.clone()),
        exif: Set(json!({})),
        status: Set("ready".to_owned()),
        ..Default::default()
    }.insert(txn).await
}
```

#### 24.3.5 media_repo 增量

```rust
use std::collections::HashMap;
use cms_entity::media_variants;

pub async fn find_by_storage_key(
    db: &DatabaseConnection, key: &str,
) -> Result<Option<Model>, DbErr> {
    Entity::find().filter(Column::StorageKey.eq(key)).one(db).await
}

pub async fn find_variants_for_assets(
    db: &DatabaseConnection, asset_ids: &[i64],
) -> Result<HashMap<i64, Vec<media_variants::Model>>, DbErr> {
    if asset_ids.is_empty() { return Ok(HashMap::new()); }
    let rows = media_variants::Entity::find()
        .filter(media_variants::Column::AssetId.is_in(asset_ids.to_vec()))
        .all(db).await?;
    let mut by: HashMap<i64, Vec<_>> = HashMap::new();
    for v in rows { by.entry(v.asset_id).or_default().push(v); }
    Ok(by)
}
```

#### 24.3.6 frontend 文件直传 UI

`frontend/src/types.ts`：

```ts
export type CompleteRequest = { storage_key: string };
export type CompleteResponse = { asset_id: number; storage_key: string; public_url: string };
```

`frontend/src/api/client.ts`（仅追加，不动 presign）：

```ts
export const api = {
  // ... 现有方法 ...
  complete: (payload: CompleteRequest) =>
    request<CompleteResponse>("/api/v1/admin/media/complete", { method: "POST", body: payload }),
};
```

`frontend/src/components/AdminApp.tsx` 中 PhotoForm 部分——把现有"申请上传 URL"按钮替换为：

```tsx
const [uploadState, setUploadState] = useState<"idle"|"uploading"|"failed">("idle");
const fileInputRef = useRef<HTMLInputElement>(null);

const onFileSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
  const file = e.target.files?.[0];
  if (!file) return;
  setUploadState("uploading");
  try {
    // 1. presign
    const p = await api.presign({
      file_name: file.name, mime_type: file.type, byte_size: file.size,
    });
    // 2. PUT 直传 S3
    const put = await fetch(p.upload_url, {
      method: "PUT",
      headers: { "content-type": file.type },
      body: file,
    });
    if (!put.ok) throw new Error(`upload failed: HTTP ${put.status}`);
    // 3. complete 回调
    const c = await api.complete({ storage_key: p.storage_key });
    // 4. 把 storage_key 填到 src 字段（**不是** public_url；
    //    backend 按 storage_key dedup 并选 variant 出 cover_url）
    update("src", c.storage_key);
    setUploadState("idle");
    onNotice(t.uploadDone as string);
  } catch (err) {
    setUploadState("failed");
    onNotice(`${t.uploadFailed}: ${err}`);
  } finally {
    if (fileInputRef.current) fileInputRef.current.value = "";
  }
};
```

UI 渲染（伪代码）：

```tsx
<label>
  {t.src}
  <input value={form.src} onChange={(e) => update("src", e.target.value)}
         placeholder="https://... 或上传后自动填" />
  <input type="file" accept="image/*" ref={fileInputRef}
         onChange={onFileSelected} disabled={uploadState === "uploading"} />
  {uploadState === "uploading" && <span>{t.uploading}</span>}
</label>
```

i18n（`frontend/src/i18n.ts`）补几条：

```ts
zh: { ..., uploading: "上传中...", uploadDone: "上传完成", uploadFailed: "上传失败", chooseFile: "选择文件" }
en: { ..., uploading: "Uploading...", uploadDone: "Uploaded", uploadFailed: "Upload failed", chooseFile: "Choose file" }
```

#### 24.3.7 不能动

- 后端 DTO 字段名 / 类型（`PhotoDto.src` 仍 `String`，语义略变："URL 或 S3 key"）
- routes.rs（PR-M2-1 已添过 `/admin/media/complete`，本 PR 不动路由）
- `cms-entity` 任何文件
- `media_assets` / `media_variants` 表 schema
- worker 代码（PR-M2-2 已稳定）
- frontend `PublicGallery` 组件（前台访问 cover URL 时是签名 URL，浏览器自动跟随）

#### DoD（贴 PR 描述）

```sh
# 0) 前提：PG/Redis/MinIO 在线，并已有至少 1 张 status=ready 的 asset 含 4 个 variants
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT a.id, a.storage_key, a.status, count(v.id) AS variants
      FROM media_assets a LEFT JOIN media_variants v ON v.asset_id=a.id
      WHERE a.status='ready' AND a.storage_key NOT LIKE 'http%'
      GROUP BY a.id ORDER BY a.id DESC LIMIT 3;"
# 期望：≥1 行，variants=4

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api > /tmp/cms.log 2>&1 &
PID=$!; sleep 3

# 3) 公开列表 demo 图（外部 Unsplash URL）src 应原样返
curl -s "http://127.0.0.1:18080/api/v1/photos?category=street" \
  | jq '.[0].src' | head -c 80
echo ""
# 期望：以 "https://images.unsplash.com/" 开头

# 4) 用上传过的真实 S3 asset 创建一张 photo（手动构造 PhotoReq，src=storage_key）
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

REAL_KEY=$(docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
  -c "SELECT storage_key FROM media_assets WHERE status='ready' AND storage_key NOT LIKE 'http%' ORDER BY id DESC LIMIT 1;")
echo "using S3 key: $REAL_KEY"

NEW=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d "{\"src\":\"$REAL_KEY\",\"cat\":\"street\",\"date\":\"2026.05\",\"privacy\":\"public\",
       \"title\":{\"zh\":\"上传测试\",\"en\":\"Upload Test\"},
       \"loc\":{\"zh\":\"测试\",\"en\":\"Test\"}}" \
  http://127.0.0.1:18080/api/v1/admin/photos)
echo "$NEW" | jq

# 5) 这张新 photo 的 src 应是 presigned GET URL（含 X-Amz-Signature）
NEW_ID=$(echo "$NEW" | jq -r .id)
SRC=$(curl -s "http://127.0.0.1:18080/api/v1/photos" | jq -r ".[] | select(.id==$NEW_ID) | .src")
echo "$SRC" | head -c 200
echo ""
echo "$SRC" | grep -q "X-Amz-Signature" && echo "✅ presigned" || echo "❌ 不是签名 URL"

# 6) URL 真能 GET（MinIO 接受签名）
curl -s -o /tmp/cover.bin -w "GET cover: %{http_code} bytes=%{size_download}\n" "$SRC"
file /tmp/cover.bin | head -1
# 期望：HTTP 200 + JPEG/WebP 文件类型（应是 medium_900 variant 而非原图）

# 7) upsert：用同一个 storage_key 再创建一张 photo，asset 不应重复落库
COUNT_BEFORE=$(docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
  -c "SELECT count(*) FROM media_assets WHERE storage_key='$REAL_KEY';")
curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d "{\"src\":\"$REAL_KEY\",\"cat\":\"life\",\"date\":\"2026.05\",\"privacy\":\"public\",
       \"title\":{\"zh\":\"复用测试\",\"en\":\"Reuse Test\"},
       \"loc\":{\"zh\":\"测试\",\"en\":\"Test\"}}" \
  http://127.0.0.1:18080/api/v1/admin/photos > /dev/null
COUNT_AFTER=$(docker exec gathered-pg psql -U postgres -d gathered_light -t -A \
  -c "SELECT count(*) FROM media_assets WHERE storage_key='$REAL_KEY';")
[ "$COUNT_BEFORE" = "$COUNT_AFTER" ] && echo "✅ asset 复用 (count=$COUNT_AFTER)" || echo "❌ asset 重复 ($COUNT_BEFORE→$COUNT_AFTER)"

# 8) 前端联调（手动）
# - 打开 http://localhost:5173/admin
# - 编辑某张 photo / 新建 photo
# - 应看到 "选择文件" input
# - 选一张本地 JPEG，几秒内 src 自动填上 uploads/2026/05/... 路径
# - 保存，前台 / 应能看到这张图（cover 是 presigned URL）

# 9) 收工
kill -INT $PID; wait $PID 2>/dev/null
```

> **预期回归**：演示分类树 + 老 Unsplash demo 图 100% 兼容（外部 URL 原样返）；新建图（含上传）走 variant 签名 URL；进程重启后 cover URL 重新签发（每次 list 都是新签名，1h 内有效）。
>
> **延后到 M3 / M5**：
> - cover URL CDN 化（生产应走 CloudFront/Akamai EdgeAuth，不直发 S3 签名）
> - locked photo 的 unlock 成功后返短期 JWT（§17）
> - cover 缓存（list 整体缓存 < 30 min，TTL 短于 cover 寿命）

---

## 23. M3-CMS-Full 后台增强（M2 已完成，本节 ACTIVE）

> **状态**：✅ M2 真上传链路已通；本节作为 M3-CMS-Full 第一砍激活。
>
> §23.1 = **PR-M3-1**（用户管理 + 登录表单 email），spec 内容继续有效。
> §23.2 / §23.3 占位，PR-M3-4 / PR-M3-5 时再细化。

### 23.1 PR-CMS-0：登录可用 + 用户管理

> **目标**：
> 1. 修登录表单——前端硬编码 `admin@gathered.local` 改成 email 输入框
> 2. 后端新增 5 个 `admin/users` 端点
> 3. 前端 admin 加 Users tab，可增 / 改 / 重置密码 / 删 admin
>
> **范围有意收窄**：本 PR **不做** 角色权限收紧（owner/editor/viewer 强制），所有登录用户都能调 users 端点；仅做 self-protect（不能删自己 / 改自己角色）。完整 RBAC 是 PR-CMS-2。

#### 23.1.1 改动文件清单

| 步骤 | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/dto/user_dto.rs`（**新建**） | `UserListItem` / `NewUserReq` / `UpdateUserReq` / `ResetPasswordReq` |
| 2 | `crates/cms-api/src/dto/mod.rs` | 加 `pub mod user_dto;` |
| 3 | `crates/cms-api/src/repositories/user_repo.rs`（**扩展**） | 增加 `list / count / update / delete / update_password` |
| 4 | `crates/cms-api/src/services/user_service.rs`（**新建**） | 5 个业务函数；create / reset_password 走 argon2 |
| 5 | `crates/cms-api/src/services/mod.rs` | 加 `pub mod user_service;` |
| 6 | `crates/cms-api/src/handlers/user_handler.rs`（**新建**） | 5 个 handler，Extension<Claims> 取 current user 做 self-protect |
| 7 | `crates/cms-api/src/handlers/mod.rs` | 加 `pub mod user_handler;` |
| 8 | `crates/cms-api/src/routes.rs` | `protected_api` 加 5 条 `/admin/users*` 路由 |
| 9 | `frontend/src/types.ts` | 加 `User` / `UserListResponse` 等类型 |
| 10 | `frontend/src/api/client.ts` | `login(password)` 改 `login(email, password)`；新增 `users.list/create/update/resetPassword/delete` |
| 11 | `frontend/src/components/AdminApp.tsx` | LoginForm 加 email 输入；顶部加 Photos/Users 切 tab；新增 Users 视图组件（表格 + 3 个模态） |

#### 23.1.2 API 设计

| 方法 | 路径 | 入参 | 返回 | 备注 |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/admin/users?page=1&page_size=20` | Query | `{ items: [UserListItem], total: i64, page, page_size }` | |
| POST | `/api/v1/admin/users` | `NewUserReq` | `UserListItem` | argon2 hash 密码 |
| PATCH | `/api/v1/admin/users/:id` | `UpdateUserReq` | `UserListItem` | 仅可改 `display_name` / `role` |
| POST | `/api/v1/admin/users/:id/password` | `ResetPasswordReq` | `204 No Content` | argon2 重新 hash |
| DELETE | `/api/v1/admin/users/:id` | — | `204 No Content` | **禁止删自己**（403） |

全部走 protected_api，自动经过 jwt_guard。

#### 23.1.3 DTO 形状（精确）

```rust
// dto/user_dto.rs
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct UserListItem {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,                      // "owner" | "editor" | "viewer"
    pub created_at: String,                // ISO 8601
}

#[derive(Debug, Serialize)]
pub struct UserListResp {
    pub items: Vec<UserListItem>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    #[serde(default = "default_page")]      pub page: u32,
    #[serde(default = "default_page_size")] pub page_size: u32,
}
fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 20 }

#[derive(Debug, Deserialize, Validate)]
pub struct NewUserReq {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub display_name: Option<String>,
    #[serde(default = "default_role")]
    pub role: String,                       // 默认 "editor"
}
fn default_role() -> String { "editor".into() }

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserReq {
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordReq {
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}
```

#### 23.1.4 self-protect 规则

`user_service` 必须在以下情况返 `AppError::Forbidden`：

| 操作 | 触发条件 |
| --- | --- |
| `update_user` | `id == current.sub.parse()? && req.role.is_some()` 时拒（不能改自己 role） |
| `delete_user` | `id == current.sub.parse()?` 时拒（不能删自己） |

`reset_password` 允许给自己改密码（这是常见需求）。

`role` 字段只接受 `"owner"` / `"editor"` / `"viewer"`，其它值返 `AppError::Validation`。

#### 23.1.5 user_repo 新增函数

```rust
pub async fn list(db: &DatabaseConnection, page: u32, page_size: u32) -> Result<Vec<Model>, DbErr> {
    Users::find()
        .order_by_desc(Column::Id)
        .paginate(db, page_size as u64)
        .fetch_page((page - 1) as u64).await
}
pub async fn count(db: &DatabaseConnection) -> Result<i64, DbErr> {
    Users::find().count(db).await.map(|n| n as i64)
}
pub async fn update(
    db: &DatabaseConnection, id: i64,
    display_name: Option<Option<String>>,    // double-Option：外层=改不改，内层=改成什么
    role: Option<String>,
) -> Result<Model, DbErr> {
    let mut m: ActiveModel = Users::find_by_id(id).one(db).await?
        .ok_or(DbErr::RecordNotFound("user".into()))?.into();
    if let Some(dn) = display_name { m.display_name = Set(dn); }
    if let Some(r)  = role         { m.role = Set(r); }
    m.update(db).await
}
pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    Users::delete_by_id(id).exec(db).await?;
    Ok(())
}
pub async fn update_password(db: &DatabaseConnection, id: i64, hash: String) -> Result<(), DbErr> {
    let mut m: ActiveModel = Users::find_by_id(id).one(db).await?
        .ok_or(DbErr::RecordNotFound("user".into()))?.into();
    m.password_hash = Set(hash);
    m.update(db).await?;
    Ok(())
}
```

#### 23.1.6 frontend 改动要点

`frontend/src/api/client.ts`：

```ts
// 旧：export async function login(password: string) { ... body: { email: "admin@gathered.local", password } }
// 新：
export async function login(email: string, password: string) {
  const pair = await request<TokenPair>("/api/v1/auth/login", {
    method: "POST",
    body: { email, password }
  }, false);
  saveTokens(pair);
  return pair.user;
}

// 新增：
export const adminUsers = {
  list: (page = 1, pageSize = 20) =>
    request<UserListResp>(`/api/v1/admin/users?page=${page}&page_size=${pageSize}`),
  create: (req: NewUserReq) =>
    request<UserListItem>("/api/v1/admin/users", { method: "POST", body: req }),
  update: (id: number, req: UpdateUserReq) =>
    request<UserListItem>(`/api/v1/admin/users/${id}`, { method: "PATCH", body: req }),
  resetPassword: (id: number, newPassword: string) =>
    request<void>(`/api/v1/admin/users/${id}/password`, {
      method: "POST", body: { new_password: newPassword }
    }),
  remove: (id: number) =>
    request<void>(`/api/v1/admin/users/${id}`, { method: "DELETE" }),
};
```

`frontend/src/components/AdminApp.tsx`：
- LoginForm：把单 password 输入改成 email + password 两个 input。
- AdminApp 顶栏 / 侧栏加 tab 切换：`Photos` (默认) | `Users`
- Users 视图：表格 + "+ 新建用户" 按钮 + 三个模态（New / Edit / ResetPassword）+ Delete confirm
- 参考现有 PhotosTab 视觉风格保持一致

#### 23.1.7 不能动

- 已有 photo / category / media / auth 业务代码
- `bootstrap.rs` / `infra/jwt.rs` / `middleware/auth.rs`
- `routes.rs` 公开路由分组
- frontend `PublicGallery` 组件
- `docs/` 任何文件

#### DoD（贴 PR 描述）

```sh
# 1) 静态
ls crates/cms-api/src/handlers/user_handler.rs \
   crates/cms-api/src/services/user_service.rs \
   crates/cms-api/src/dto/user_dto.rs
# 期望：3 个文件齐全

cargo check --workspace --all-targets 2>&1 | tail -3
# 期望：Finished `dev` profile

# 2) seed 第一个 owner
cargo run -p cms-api --bin seed_admin -- owner@local.dev "OwnerPass1!"

# 3) 起后端
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api > /tmp/cms.log 2>&1 &
PID=$!; sleep 3

# 4) 拿 token
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"owner@local.dev","password":"OwnerPass1!"}' | jq -r .access_token)

# 5) 列用户（应至少 1 条）
curl -s -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/admin/users | jq

# 6) 创建 editor
NEW=$(curl -s -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"email":"editor@local.dev","password":"EditorPass1!","display_name":"Editor","role":"editor"}' \
  http://127.0.0.1:18080/api/v1/admin/users)
echo "$NEW" | jq
EID=$(echo "$NEW" | jq -r .id)

# 7) editor 也能登录
ETOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"editor@local.dev","password":"EditorPass1!"}' | jq -r .access_token)
[ -n "$ETOK" ] && echo "✅ editor login" || echo "❌"

# 8) 改 display_name
curl -s -X PATCH -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"display_name":"Senior Editor"}' \
  http://127.0.0.1:18080/api/v1/admin/users/$EID | jq -r .display_name
# 期望：Senior Editor

# 9) 重置密码
curl -s -o /dev/null -w "reset pwd HTTP %{http_code}\n" \
  -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"new_password":"NewPass1!"}' \
  http://127.0.0.1:18080/api/v1/admin/users/$EID
# 期望：204

# 10) 用新密码登录验证
NEW_ETOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"editor@local.dev","password":"NewPass1!"}' | jq -r .access_token)
[ -n "$NEW_ETOK" ] && echo "✅ new password works" || echo "❌"

# 11) self-protect：不能删自己
SELF_ID=$(curl -s -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/auth/me | jq -r .user.id)
curl -s -o /dev/null -w "delete self HTTP %{http_code}\n" \
  -X DELETE -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/admin/users/$SELF_ID
# 期望：403

# 12) 删 editor
curl -s -o /dev/null -w "delete editor HTTP %{http_code}\n" \
  -X DELETE -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/admin/users/$EID
# 期望：204

# 13) frontend 联调（手动）
# - 浏览器开 http://localhost:5173/admin
# - 登录页应有 email + password 两个输入框（不再硬编码）
# - 输入 owner@local.dev / OwnerPass1! 登录成功
# - 顶部能看到 Photos / Users 两个 tab
# - Users tab：能看到表格、能新建、能编辑、能改密、能删（删自己时禁掉或报错）

kill -INT $PID; wait $PID 2>/dev/null
```

---

### 23.2 PR-M3-2：Tags 子系统（CRUD + 给 photo 打标）

> **目标**：管理后台能加 / 改 / 删标签；photo 编辑时可勾选已有标签；前后端契约打通。
>
> **前提**：M2 + PR-M3-1 已 ✅；`tags` + `photo_tags` 表已在 PR-2 迁移建好（空表）。

#### 23.2.1 改动文件清单（11 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/dto/tag_dto.rs`（**新建**） | `TagDto` / `TagListResp` / `NewTagReq` / `UpdateTagReq` / `TagListQuery` |
| 2 | `crates/cms-api/src/dto/mod.rs` | 加 `pub mod tag_dto;` |
| 3 | `crates/cms-api/src/dto/photo_dto.rs` | `PhotoDto` 加 `pub tags: Vec<TagSummary>`；`PhotoReq` 加 `#[serde(default)] pub tag_ids: Vec<i64>` |
| 4 | `crates/cms-api/src/repositories/tag_repo.rs`（**新建**） | `list / count / find_by_id / find_by_slug / insert / update / delete / find_for_photos / set_for_photo` |
| 5 | `crates/cms-api/src/repositories/mod.rs` | 加 `pub mod tag_repo;` |
| 6 | `crates/cms-api/src/repositories/photo_repo.rs::create / update` | 同事务里调 `tag_repo::set_for_photo` 把 `NewPhoto.tag_ids` 落库 |
| 7 | `crates/cms-api/src/services/tag_service.rs`（**新建**） | `list_admin_tags / list_public_tags / create_tag / update_tag / delete_tag` |
| 8 | `crates/cms-api/src/services/mod.rs` | 加 `pub mod tag_service;` |
| 9 | `crates/cms-api/src/services/photo_service.rs::list_*` | batch 取每个 photo 的 tags（`tag_repo::find_for_photos`），assemble 时填入 `PhotoDto.tags` |
| 10 | `crates/cms-api/src/handlers/tag_handler.rs`（**新建**） | 5 个 handler |
| 11 | `crates/cms-api/src/handlers/mod.rs` + `routes.rs` | 加路由：`GET /api/v1/tags`（公开）+ `GET/POST /api/v1/admin/tags` + `PATCH/DELETE /api/v1/admin/tags/:id` |

**前端追加**（计入同 PR）：
| 文件 | 动作 |
| --- | --- |
| `frontend/src/types.ts` | 加 `Tag` / `TagSummary` / `TagListResp` 等类型 |
| `frontend/src/api/client.ts` | 加 `adminTags.list/create/update/remove` + `api.publicTags()` |
| `frontend/src/components/AdminApp.tsx` | 顶栏 tab 加 `Tags`；新增 `TagsView` 组件（表格 + 新建/编辑/删除模态）；`PhotoForm` 加"标签"区块（多选 checkbox 列出所有 tags，绑定 `tag_ids`） |
| `frontend/src/i18n.ts` | 加 `tags / addTag / tagSlug / tagNameZh / tagNameEn` 等 key |

#### 23.2.2 API 设计

| 方法 | 路径 | 入参 | 返回 | 鉴权 |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/tags` | — | `Vec<TagDto>` | 无（公开） |
| GET | `/api/v1/admin/tags?page=&page_size=` | Query | `TagListResp` | jwt_guard |
| POST | `/api/v1/admin/tags` | `NewTagReq` | `TagDto` | jwt_guard |
| PATCH | `/api/v1/admin/tags/:id` | `UpdateTagReq` | `TagDto` | jwt_guard |
| DELETE | `/api/v1/admin/tags/:id` | — | `204` | jwt_guard |

打标动作**复用** `POST /admin/photos` 与 `PUT /admin/photos/:id` —— `PhotoReq.tag_ids` 字段：
- create 时：含 tag_ids 就同事务挂上
- update 时：tag_ids 含啥就**全量替换** photo_tags（先 DELETE 再 INSERT）；不传 = 不动
- 但因为 `#[serde(default)]` 默认是空 `Vec`，前端如果不发字段或发 `[]` 都会被解释成"清空"。**前端必须每次都发完整的 tag_ids 列表**——这是契约。

#### 23.2.3 DTO 形状

```rust
// dto/tag_dto.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::dto::photo_dto::I18nText;

#[derive(Debug, Clone, Serialize)]
pub struct TagDto {
    pub id: i64,
    pub slug: String,
    pub name: I18nText,
}

#[derive(Debug, Serialize)]
pub struct TagListResp {
    pub items: Vec<TagDto>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct TagListQuery {
    #[serde(default = "default_page")]      pub page: u32,
    #[serde(default = "default_page_size")] pub page_size: u32,
}
fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 50 }

#[derive(Debug, Deserialize, Validate)]
pub struct NewTagReq {
    #[validate(regex(path = "*SLUG_RE", message = "slug must match [a-z0-9-]{1,40}"))]
    pub slug: String,
    pub name: I18nText,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTagReq {
    pub slug: Option<String>,
    pub name: Option<I18nText>,
}

// 用 lazy_static 或者 once_cell 拿出 SLUG_RE
// 简化：service 层手动 regex 校验更直接
```

> **slug 校验**用 service 层手写（不引 lazy_static / regex 依赖）：`slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && (1..=40).contains(&slug.len())`。

`PhotoDto` 增量：

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TagSummary {
    pub id: i64,
    pub slug: String,
    pub name: I18nText,
}

pub struct PhotoDto {
    // ... 现有字段不动 ...
    pub tags: Vec<TagSummary>,    // ★ 新增；空数组也要返
}

pub struct PhotoReq {
    // ... 现有字段不动 ...
    #[serde(default)]
    pub tag_ids: Vec<i64>,        // ★ 新增；create / update 都接受，全量替换
}
```

#### 23.2.4 tag_repo 关键 API

```rust
pub async fn list(db: &DatabaseConnection, page: u32, page_size: u32) -> Result<Vec<Model>, DbErr>;
pub async fn count(db: &DatabaseConnection) -> Result<i64, DbErr>;
pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<Model>, DbErr>;
pub async fn find_by_slug(db: &DatabaseConnection, slug: &str) -> Result<Option<Model>, DbErr>;
pub async fn insert(db: &DatabaseConnection, slug: String, name_zh: String, name_en: String)
    -> Result<Model, DbErr>;
pub async fn update(db: &DatabaseConnection, id: i64,
    slug: Option<String>, name_zh: Option<String>, name_en: Option<String>) -> Result<Model, DbErr>;
pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr>;

/// batch：`SELECT t.*, pt.photo_id FROM tags t JOIN photo_tags pt ON pt.tag_id = t.id
///        WHERE pt.photo_id IN (...)` → group by photo_id
pub async fn find_for_photos(db: &DatabaseConnection, photo_ids: &[i64])
    -> Result<HashMap<i64, Vec<Model>>, DbErr>;

/// 全量替换：DELETE FROM photo_tags WHERE photo_id = ? + bulk INSERT
pub async fn set_for_photo<C: ConnectionTrait>(c: &C, photo_id: i64, tag_ids: &[i64])
    -> Result<(), DbErr>;
```

`name_i18n` 在表上是 `JSONB` —— `insert` 时构造 `serde_json::json!({"zh": name_zh, "en": name_en})`；查询时反过来 unwrap。

#### 23.2.5 photo_service 改动

list 函数跟 PR-M2-3 的 variants batch fetch 同套路：

```rust
pub async fn list_public(state: &AppState, q: PhotoQuery) -> AppResult<Vec<PhotoDto>> {
    let fulls = photo_repo::list_public(&state.db, q.category.as_deref()).await.map_err(db_err)?;

    let asset_ids:  Vec<i64> = fulls.iter().map(|f| f.asset.id).collect();
    let photo_ids:  Vec<i64> = fulls.iter().map(|f| f.photo.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids).await.map_err(db_err)?;
    let tags_map     = tag_repo::find_for_photos(&state.db, &photo_ids).await.map_err(db_err)?;

    let mut out = Vec::with_capacity(fulls.len());
    for f in fulls {
        let v = variants_map.get(&f.asset.id).cloned().unwrap_or_default();
        let t = tags_map.get(&f.photo.id).cloned().unwrap_or_default();
        let cover = cover_url_for(state, &f.asset, &v).await?;
        out.push(assemble_dto(f, cover, t));
    }
    Ok(out)
}
```

`photo_repo::create` / `update` 在事务里追加：
```rust
tag_repo::set_for_photo(&txn, photo.id, &input.tag_ids).await?;
```

#### 23.2.6 frontend Tags tab + photo 标签选择

`AdminApp.tsx`：
- `activeTab` union 改为 `"photos" | "users" | "tags"`
- 顶栏加 `<button onClick={() => setActiveTab("tags")}>{t.tags}</button>`
- `<TagsView />` 跟 UsersView 一个套路（表格 + 新建/编辑/删除模态）
- `PhotoForm` 加一个区块"标签"：
  - 加载所有 tags（一次 `adminTags.list(1, 200)`）
  - 渲染为多选 checkbox 列表，预勾上 `photo.tags` 里那些
  - 用户改动 → 维护本地 `selectedTagIds`
  - Save 时 `tag_ids: selectedTagIds` 一起提交

#### 23.2.7 不能动

- 现有 `auth_service / user_service / media_service / cover_url_for / jwt_guard`
- 现有 `media_repo / user_repo` 函数（仅扩展 photo_repo / 新增 tag_repo）
- `tags` / `photo_tags` 表 schema（已建好）
- `bootstrap.rs::AppState`
- 前端 `PublicGallery` 组件（标签的公开过滤 UI 不在本 PR）

#### DoD（贴 PR 描述）

```sh
# 0) 前提
docker ps --filter "name=gathered-" --format "{{.Names}}"  # gathered-pg/redis/minio Up

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3

# 2) 起 cms-api（main.rs 已有 dotenvy::dotenv 自动 load .env）
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api > /tmp/m32.log 2>&1 &
PID=$!; sleep 3

# 3) 拿 token
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

# 4) 公开 tags 列表（应空数组）
curl -s http://127.0.0.1:18080/api/v1/tags | jq
# 期望：[]

# 5) 创建 3 个 tag
for slug in "morning" "tokyo" "film-grain"; do
  curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
    -d "{\"slug\":\"$slug\",\"name\":{\"zh\":\"$slug\",\"en\":\"$slug\"}}" \
    http://127.0.0.1:18080/api/v1/admin/tags | jq -c
done

# 6) admin list（应 3 行 + total=3）
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/tags | jq

# 7) slug 校验：错 slug 必 400
curl -s -o /dev/null -w "bad slug: %{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"slug":"BAD SLUG WITH CAPS","name":{"zh":"x","en":"x"}}' \
  http://127.0.0.1:18080/api/v1/admin/tags
# 期望：400

# 8) PATCH 改 name
TID=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/tags | jq -r '.items[0].id')
curl -s -X PATCH -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"name":{"zh":"早晨","en":"Morning"}}' \
  http://127.0.0.1:18080/api/v1/admin/tags/$TID | jq

# 9) 给一张 photo 打标（取 demo 第一张）
PID_FIRST=$(curl -s "http://127.0.0.1:18080/api/v1/photos" | jq -r '.[0].id')
ALL_TAG_IDS=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/tags | jq '[.items[].id]')
# 用 PUT 全量更新（保持其它字段不变 → 拉一份原值）
ORIG=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos | jq ".[] | select(.id==$PID_FIRST)")
PAYLOAD=$(echo "$ORIG" | jq --argjson tids "$ALL_TAG_IDS" '. + {tag_ids: $tids}')
curl -s -X PUT -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d "$PAYLOAD" http://127.0.0.1:18080/api/v1/admin/photos/$PID_FIRST | jq '.tags'
# 期望：3 个 tag 对象数组

# 10) 公开列表带 tags 字段
curl -s "http://127.0.0.1:18080/api/v1/photos" | jq ".[] | select(.id==$PID_FIRST) | {id, tags: .tags|length}"
# 期望：tags: 3

# 11) 删一个 tag → photo_tags 级联删
curl -s -o /dev/null -w "delete tag: %{http_code}\n" \
  -X DELETE -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/tags/$TID
# 期望：204
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT count(*) FROM photo_tags WHERE photo_id=$PID_FIRST;"
# 期望：2（原来 3 个 - 删了 1 个 = 2）

# 12) 前端联调（手动）
# - 浏览器开 http://localhost:5173/admin
# - 顶栏多了 Tags tab；点进去能加/改/删
# - 编辑某张 photo，下面多了"标签"区块带 checkbox
# - 勾选保存后，公开列表的 photo.tags 反映出来

kill -INT $PID; wait $PID 2>/dev/null
```

#### 23.2.8 反作弊扫描（自证）

```sh
# 1. 关键文件落盘
ls crates/cms-api/src/{handlers/tag_handler,services/tag_service,repositories/tag_repo,dto/tag_dto}.rs

# 2. 5 个 service 函数
grep -c "^pub async fn" crates/cms-api/src/services/tag_service.rs
# 期望：5+ （list_admin/list_public/create/update/delete）

# 3. 路由
grep -E '"/admin/tags|"/tags' crates/cms-api/src/routes.rs

# 4. PhotoDto 含 tags 字段
grep "pub tags:" crates/cms-api/src/dto/photo_dto.rs

# 5. PhotoReq 含 tag_ids
grep "pub tag_ids:" crates/cms-api/src/dto/photo_dto.rs

# 6. cargo check + tsc
cargo check --workspace --all-targets 2>&1 | tail -3
cd frontend && npx tsc --noEmit 2>&1 | tail -3 && cd ..
```

---

### 23.3 PR-M3-3：Categories 后台 CRUD

> **目标**：把当前 3 条种子分类（street / landscape / life）变成可在 admin 增 / 删 / 改 / 排序的实体；前台公开 `/categories` 端点保持兼容。
>
> **前提**：PR-M3-1 / PR-M3-2 已 ✅；`categories` 表（PR-2 建）已有 3 条种子。

#### 23.3.1 改动文件清单（10 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/dto/category_dto.rs`（**新建**） | `CategoryAdminDto` / `NewCategoryReq` / `UpdateCategoryReq` （保留 `CategoryDto` 用于公开） |
| 2 | `crates/cms-api/src/dto/mod.rs` | 加 `pub mod category_dto;` |
| 3 | `crates/cms-api/src/repositories/category_repo.rs`（**新建**） | `list` / `count` / `find_by_id` / `find_by_slug` / `insert` / `update` / `delete` / `count_photos_in(id)` |
| 4 | `crates/cms-api/src/repositories/mod.rs` | 加 `pub mod category_repo;` |
| 5 | `crates/cms-api/src/services/category_service.rs`（**新建** 或扩展现有） | `list_admin_categories` / `create_category` / `update_category` / `delete_category`；保留 list_public 的现有逻辑 |
| 6 | `crates/cms-api/src/services/mod.rs` | 加 `pub mod category_service;`（如未挂） |
| 7 | `crates/cms-api/src/handlers/category_handler.rs` | 新增 4 个 admin handler（list_admin / create / update / delete）；保留现有公开 list |
| 8 | `crates/cms-api/src/routes.rs` | `protected_api` 加 `GET/POST /admin/categories` + `PATCH/DELETE /admin/categories/:id` |
| 9 | 前端 `frontend/src/types.ts` + `frontend/src/api/client.ts` | 加 `CategoryAdminDto` / `NewCategoryReq` / `UpdateCategoryReq` 类型 + `adminCategories.list/create/update/remove` |
| 10 | 前端 `frontend/src/components/AdminApp.tsx` + `frontend/src/i18n.ts` | 顶栏加 `Categories` tab；`CategoriesView` 组件（表格 + 新建/编辑/删除模态）；i18n key |

#### 23.3.2 API 设计

| 方法 | 路径 | 入参 | 返回 |
| --- | --- | --- | --- |
| GET | `/api/v1/categories` | — | `Vec<CategoryDto>`（**公开保持不变**） |
| GET | `/api/v1/admin/categories` | — | `Vec<CategoryAdminDto>`（含 sort_order + photo_count） |
| POST | `/api/v1/admin/categories` | `NewCategoryReq` | `CategoryAdminDto` |
| PATCH | `/api/v1/admin/categories/:id` | `UpdateCategoryReq` | `CategoryAdminDto` |
| DELETE | `/api/v1/admin/categories/:id` | — | `204` 或 `409` 如果有 photos 引用 |

categories 总量预期 < 100，不分页（list_admin 一把返回）。

#### 23.3.3 DTO 形状

```rust
// dto/category_dto.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::dto::photo_dto::I18nText;

#[derive(Debug, Clone, Serialize)]
pub struct CategoryAdminDto {
    pub id: i64,
    pub slug: String,
    pub name: I18nText,
    pub sort_order: i32,
    pub photo_count: i64,         // ★ admin 用，让用户决定能否安全删
}

#[derive(Debug, Deserialize, Validate)]
pub struct NewCategoryReq {
    pub slug: String,             // service 层手写 [a-z0-9-]{1,40} 校验
    pub name: I18nText,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCategoryReq {
    pub slug: Option<String>,
    pub name: Option<I18nText>,
    pub sort_order: Option<i32>,
}
```

保留现有 `CategoryDto`（公开 GET /categories 用的形状）不动；admin 用 `CategoryAdminDto`。

#### 23.3.4 删除策略（重要）

`photos.category_id` 是 FK 无 CASCADE 行为（PR-2 DDL `REFERENCES categories(id)` 不带 ON DELETE）。直接删会被 PG 拒绝并报 FK 错误。

**service 层主动校验**：

```rust
pub async fn delete_category(state: &AppState, id: i64) -> AppResult<()> {
    let n = category_repo::count_photos_in(&state.db, id).await.map_err(db_err)?;
    if n > 0 {
        return Err(AppError::Conflict("category still has photos; reassign or delete them first"));
    }
    category_repo::delete(&state.db, id).await.map_err(db_err)?;
    Ok(())
}
```

不要去改 schema（不动 FK）。这样删除前 admin UI 必能看到 photo_count，决定是否清理。

#### 23.3.5 slug 唯一与变更约束

- create：service 先 `find_by_slug` 命中就返 `Conflict("slug already exists")`
- update：同上；外加**不阻止改 slug**（admin 知情决定），但前端 UI 应在编辑弹窗加一行警告"改 slug 会让旧 URL `/photos?category=<old>` 失效"

#### 23.3.6 photo 引用迁移（不在本 PR）

如果未来要"把 A 类下所有 photo 改成 B 类"，建议另开端点 `POST /admin/categories/:from_id/move?to=:to_id`。本 PR **不做**。

#### 23.3.7 不能动

- `frontend/PublicGallery` 组件、`PublicGallery` 的分类过滤逻辑
- 现有 `GET /api/v1/categories` 的响应形状（H-10）
- `photos` 表 schema / FK 约束
- 任何 `auth / user / tag / media / photo` 业务代码
- `routes.rs` 公开路由 / `bootstrap.rs / jwt_guard / cover_url_for`

#### DoD（贴 PR 描述）

```sh
# 0) 前提
docker ps --filter "name=gathered-" --format "{{.Names}}"  # gathered-pg/redis/minio Up

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api > /tmp/m33.log 2>&1 &
PID=$!; sleep 3
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

# 3) 公开 /categories 形状保持（H-10）
curl -s http://127.0.0.1:18080/api/v1/categories | jq '.[0]'
# 期望：与本次改动前完全一致（不能多/少字段）

# 4) /admin/categories 应有 3 条 + photo_count 字段
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/categories | jq
# 期望：3 行，每行含 id/slug/name/sort_order/photo_count，其中 street/landscape/life 的 photo_count > 0

# 5) 创建一个新分类
NEW=$(curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"slug":"portrait","name":{"zh":"人像","en":"Portrait"},"sort_order":40}' \
  http://127.0.0.1:18080/api/v1/admin/categories)
echo "$NEW" | jq
CID=$(echo "$NEW" | jq -r .id)

# 6) 重复 slug 必 409
echo -n "duplicate slug: "
curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"slug":"portrait","name":{"zh":"x","en":"x"}}' \
  http://127.0.0.1:18080/api/v1/admin/categories

# 7) bad slug 必 400
echo -n "bad slug: "
curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' \
  -d '{"slug":"BAD SLUG","name":{"zh":"x","en":"x"}}' \
  http://127.0.0.1:18080/api/v1/admin/categories

# 8) PATCH 改 name + sort_order
curl -s -X PATCH -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"name":{"zh":"肖像","en":"Portraits"},"sort_order":15}' \
  http://127.0.0.1:18080/api/v1/admin/categories/$CID | jq

# 9) delete 空分类 → 204
echo -n "delete empty: "
curl -s -o /dev/null -w "%{http_code}\n" \
  -X DELETE -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/admin/categories/$CID

# 10) delete 非空分类 → 409
STREET_ID=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/categories \
  | jq -r '.[] | select(.slug=="street") | .id')
echo -n "delete non-empty: "
curl -s -o /dev/null -w "%{http_code}\n" \
  -X DELETE -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/admin/categories/$STREET_ID
# 期望：409，并且分类仍在表里
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/categories \
  | jq '[.[].slug] | contains(["street"])'
# 期望：true

# 11) 前端联调（手动）
# - 浏览器开 http://localhost:5173/admin
# - 顶栏多了 Categories tab；点进去能加/改/删
# - 删非空分类时看到清晰的 409 提示

# 收工
kill -INT $PID; wait $PID 2>/dev/null
```

#### 23.3.8 自证扫描

```sh
ls crates/cms-api/src/{handlers/category_handler,services/category_service,repositories/category_repo,dto/category_dto}.rs

grep -c "^pub async fn" crates/cms-api/src/services/category_service.rs
# 期望：>= 5（含原 list_public）

grep -E '"/admin/categories' crates/cms-api/src/routes.rs | wc -l
# 期望：2（一行 list+create，一行 update+delete）

cargo check --workspace --all-targets 2>&1 | tail -3
cd frontend && npx tsc --noEmit 2>&1 | tail -3 && cd ..
```

---

### 23.4 PR-M3-4：Photo 元数据增强 + 分页 + 搜索

> **目标**：
> 1. `PhotoDto` 加 **slug / caption / alt_text** 字段（schema 早就有，只是没暴露到 API）
> 2. `GET /api/v1/admin/photos` 支持 `?page=&page_size=&q=&category=` 分页 + 搜索
> 3. 前端 admin 表格加分页器 + 搜索框
>
> **前提**：M2 + PR-M3-1..3 已 ✅；`photos.slug` 与 `photo_translations.caption/alt_text` 列在 PR-2 已建好（**不动 schema**）。

#### 23.4.1 改动文件清单（8 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/dto/photo_dto.rs` | `PhotoDto` 加 `slug` / `caption: Option<I18nText>` / `alt_text: Option<I18nText>`；`PhotoReq` 加同名可选字段；新增 `PhotoListQuery` / `PhotoListResp` |
| 2 | `crates/cms-api/src/repositories/photo_repo.rs` | `list_admin` 重命名为 `list_admin_paginated`（含 q+page），`list_public` 保持 flat 返回；`PhotoFull` 字段透传 caption/alt_text（已在 translations join）；create/update 写 photo.slug / caption / alt_text |
| 3 | `crates/cms-api/src/services/photo_service.rs` | `list_admin` 改返 `PhotoListResp`；新增 service 端 slug 校验 `[a-z0-9-]{1,80}` + 不空时唯一性检查；assemble_dto 填新字段 |
| 4 | `crates/cms-api/src/handlers/photo_handler.rs` | `list_admin` handler 加 `Query<PhotoListQuery>`，返 `Json<PhotoListResp>`；`list_public` 不动 |
| 5 | `frontend/src/types.ts` | `Photo` 加 `slug / caption / alt_text`；新增 `PhotoListResp` |
| 6 | `frontend/src/api/client.ts` | `adminPhotos.list(page, pageSize, q?, cat?)` 改签名返 PhotoListResp |
| 7 | `frontend/src/components/AdminApp.tsx` | 表格上方加搜索框（300ms debounce）+ 表格下方加分页器（上一页/下一页/页码/总数）；PhotoForm 加 3 个新字段（slug 输入 + caption 双语 + alt_text 双语） |
| 8 | `frontend/src/i18n.ts` | 加 `searchPlaceholder / nextPage / prevPage / pageOf / slug / caption / altText` |

#### 23.4.2 API 设计

**`GET /api/v1/admin/photos?page=1&page_size=20&q=tokyo&category=street`**

| Query | 类型 | 默认 | 含义 |
| --- | --- | --- | --- |
| `page` | u32 | 1 | 1-based |
| `page_size` | u32 | 20 | 1..=100 |
| `q` | string | none | 全文：`slug` OR `title.{zh,en}` OR `location.{zh,en}` OR `caption.{zh,en}` ILIKE `%q%` |
| `category` | string slug | none | category slug 过滤 |
| `privacy` | string | none | "public" / "locked" / "private" 过滤 |

**返回**：

```json
{
  "items": [PhotoDto, ...],
  "total": 123,
  "page": 1,
  "page_size": 20
}
```

公开 `GET /api/v1/photos`（前台瀑布流）**保持 flat array 返回不变**（H-10）。前端瀑布流不分页。

#### 23.4.3 DTO 形状

```rust
// dto/photo_dto.rs
#[derive(Debug, Clone, Serialize)]
pub struct PhotoDto {
    pub id: i64,
    pub slug: String,                       // ★ 新增
    pub src: String,
    pub cat: String,
    pub title: I18nText,
    pub loc: I18nText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<I18nText>,          // ★ 新增（NULL → null）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<I18nText>,         // ★ 新增
    pub date: String,
    pub privacy: Privacy,
    pub tags: Vec<TagSummary>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PhotoReq {
    pub src: String,
    pub cat: String,
    pub title: I18nText,
    pub loc: I18nText,
    pub date: String,
    pub privacy: Privacy,
    #[serde(default)] pub slug: Option<String>,         // ★ 不传则用 photo-{uuid}
    #[serde(default)] pub caption: Option<I18nText>,    // ★
    #[serde(default)] pub alt_text: Option<I18nText>,   // ★
    #[serde(default)] pub tag_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoListQuery {
    #[serde(default = "default_page")]      pub page: u32,
    #[serde(default = "default_page_size")] pub page_size: u32,
    #[serde(default)] pub q: Option<String>,
    #[serde(default)] pub category: Option<String>,
    #[serde(default)] pub privacy: Option<String>,
}
fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 20 }

#[derive(Debug, Serialize)]
pub struct PhotoListResp {
    pub items: Vec<PhotoDto>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}
```

#### 23.4.4 search 查询实现要点

```rust
// repositories/photo_repo.rs
pub async fn list_admin_paginated(
    db: &DatabaseConnection,
    q: Option<&str>,
    category_slug: Option<&str>,
    privacy: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PhotoFull>, i64), DbErr> {
    let mut sel = photos::Entity::find();

    if let Some(p) = privacy {
        sel = sel.filter(photos::Column::Privacy.eq(p));
    }
    if let Some(slug) = category_slug.filter(|s| !s.is_empty() && *s != "all") {
        let cat_id = category_repo::find_by_slug(db, slug).await?
            .ok_or_else(|| DbErr::RecordNotFound("category".into()))?.id;
        sel = sel.filter(photos::Column::CategoryId.eq(cat_id));
    }
    if let Some(query) = q.map(str::trim).filter(|s| !s.is_empty()) {
        let pat = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        // JOIN photo_translations，title/location/caption 任一命中
        // 注意 DISTINCT 防止一张图两个 translation 都命中重复
        sel = sel
            .left_join(photo_translations::Entity)
            .filter(
                Condition::any()
                    .add(photos::Column::Slug.like(&pat))
                    .add(photo_translations::Column::Title.like(&pat))
                    .add(photo_translations::Column::Location.like(&pat))
                    .add(photo_translations::Column::Caption.like(&pat))
            )
            .distinct();
    }

    let total = sel.clone().count(db).await? as i64;

    let rows = sel
        .order_by_desc(photos::Column::SortOrder)
        .order_by_desc(photos::Column::TakenAtDate)
        .order_by_desc(photos::Column::Id)
        .limit(page_size as u64)
        .offset(((page.saturating_sub(1)) as u64) * page_size as u64)
        .find_also_related(media_assets::Entity)
        .all(db).await?;

    let fulls = assemble_photo_fulls(db, rows).await?;
    Ok((fulls, total))
}
```

注意 `pat` 转义：用户输入 `%` 或 `_` 不能误当通配符。

#### 23.4.5 service 层

```rust
pub async fn list_admin(state: &AppState, q: PhotoListQuery) -> AppResult<PhotoListResp> {
    let page_size = q.page_size.clamp(1, 100);
    let page      = q.page.max(1);

    let (fulls, total) = photo_repo::list_admin_paginated(
        &state.db,
        q.q.as_deref(), q.category.as_deref(), q.privacy.as_deref(),
        page, page_size,
    ).await.map_err(db_err)?;

    let photo_ids: Vec<i64> = fulls.iter().map(|f| f.photo.id).collect();
    let asset_ids: Vec<i64> = fulls.iter().map(|f| f.asset.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids).await.map_err(db_err)?;
    let tags_map     = tag_repo::find_for_photos(&state.db, &photo_ids).await.map_err(db_err)?;

    let mut items = Vec::with_capacity(fulls.len());
    for f in fulls {
        let v = variants_map.get(&f.asset.id).cloned().unwrap_or_default();
        let t = tags_map.get(&f.photo.id).cloned().unwrap_or_default();
        let cover = cover_url_for(state, &f.asset, &v).await?;
        items.push(assemble_dto(f, cover, t));
    }
    Ok(PhotoListResp { items, total, page, page_size })
}
```

slug 校验 + 唯一性：

```rust
pub async fn create(state: &AppState, req: PhotoReq) -> AppResult<PhotoDto> {
    validate(&req)?;
    let slug = match req.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(user_slug) => {
            validate_slug(user_slug)?;
            if photo_repo::find_by_slug(&state.db, user_slug).await.map_err(db_err)?.is_some() {
                return Err(AppError::Conflict("slug already exists"));
            }
            user_slug.to_string()
        }
        None => format!("photo-{}", Uuid::new_v4()),
    };
    /* ... 接现有逻辑 ... */
}

fn validate_slug(s: &str) -> AppResult<()> {
    if !(1..=80).contains(&s.len()) ||
       !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(AppError::Validation("slug must match [a-z0-9-]{1,80}".into()));
    }
    Ok(())
}
```

caption / alt_text 写入：在事务里的 translation insert 时附加列；`Option::None` 对应 NULL。

#### 23.4.6 frontend 改动

- `client.ts`：
  ```ts
  adminPhotos: {
    list: (page = 1, pageSize = 20, q = "", category = "") =>
      request<PhotoListResp>(
        `/api/v1/admin/photos?page=${page}&page_size=${pageSize}` +
        (q ? `&q=${encodeURIComponent(q)}` : "") +
        (category ? `&category=${encodeURIComponent(category)}` : "")
      ),
    /* ... */
  }
  ```

- `AdminApp.tsx::PhotosView`（或主体）：
  ```tsx
  const [page, setPage] = useState(1);
  const [q, setQ] = useState("");
  const debouncedQ = useDebounce(q, 300);
  const [resp, setResp] = useState<PhotoListResp | null>(null);

  useEffect(() => {
    api.adminPhotos.list(page, 20, debouncedQ).then(setResp);
  }, [page, debouncedQ]);
  ```
  - 搜索框 + 分页器（上一页/下一页/页码/共 N 张）
  - PhotoForm 加 slug 输入 + 双语 caption + 双语 alt_text 输入

- 简单 useDebounce hook（< 10 行）可写在 hooks/useDebounce.ts

#### 23.4.7 不能动

- 公开 `GET /api/v1/photos` 响应（仍 flat array，H-10）
- `categories / tags / users / auth / media` 业务代码
- schema / migrations
- `cover_url_for` / worker / jwt_guard / bootstrap
- frontend PublicGallery 组件
- `docs/` 任何文件

#### DoD（贴 PR 描述）

```sh
# 0) 前提
docker ps --filter "name=gathered-" --format "{{.Names}}"

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api > /tmp/m34.log 2>&1 &
PID=$!; sleep 3
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

# 3) admin list 默认分页（应返 {items, total, page, page_size}）
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos | jq '{total, page, page_size, items_len: (.items|length)}'
# 期望：total>=22, page=1, page_size=20, items_len<=20

# 4) page=2 应返不同图
curl -s -H "Authorization: Bearer $TOK" "http://127.0.0.1:18080/api/v1/admin/photos?page=2&page_size=10" | jq '.items[0].id, .total'

# 5) 搜索测试（demo 数据里"东京"是其中一张图的 location）
curl -s -H "Authorization: Bearer $TOK" "http://127.0.0.1:18080/api/v1/admin/photos?q=tokyo&page_size=50" | jq '.total, [.items[].id]'
# 期望：total >= 1
curl -s -H "Authorization: Bearer $TOK" "http://127.0.0.1:18080/api/v1/admin/photos?q=%E4%B8%9C%E4%BA%AC&page_size=50" | jq '.total'
# 期望：和 tokyo 同（中文 / 英文 都能命中）

# 6) 组合 q + category
curl -s -H "Authorization: Bearer $TOK" "http://127.0.0.1:18080/api/v1/admin/photos?q=tokyo&category=street" | jq '.total'

# 7) privacy 过滤
curl -s -H "Authorization: Bearer $TOK" "http://127.0.0.1:18080/api/v1/admin/photos?privacy=locked&page_size=50" | jq '.total'

# 8) 公开 /photos 形状保持 flat（H-10）
curl -s "http://127.0.0.1:18080/api/v1/photos" | jq 'type'
# 期望："array"

# 9) PhotoDto 新字段（slug + 可选 caption/alt_text）
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos | jq '.items[0] | keys'
# 期望：含 slug、tags；caption/alt_text 可能不在（serde skip_none）

# 10) create 时 slug 校验
echo -n "bad slug: "
curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"slug":"BAD SLUG","src":"https://x","cat":"street","date":"2026.05","privacy":"public","title":{"zh":"x","en":"x"},"loc":{"zh":"x","en":"x"}}' \
  http://127.0.0.1:18080/api/v1/admin/photos

# 11) create 时 slug 重复
echo -n "duplicate slug: "
EXISTING_SLUG=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos | jq -r '.items[0].slug')
curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d "{\"slug\":\"$EXISTING_SLUG\",\"src\":\"https://x\",\"cat\":\"street\",\"date\":\"2026.05\",\"privacy\":\"public\",\"title\":{\"zh\":\"x\",\"en\":\"x\"},\"loc\":{\"zh\":\"x\",\"en\":\"x\"}}" \
  http://127.0.0.1:18080/api/v1/admin/photos
# 期望：409

# 12) create 时带 caption + alt_text
NEW=$(curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"slug":"m34-test","src":"https://images.unsplash.com/photo-test","cat":"street","date":"2026.05","privacy":"public","title":{"zh":"测试","en":"Test"},"loc":{"zh":"x","en":"x"},"caption":{"zh":"测试说明","en":"Test caption"},"alt_text":{"zh":"alt","en":"alt"}}' \
  http://127.0.0.1:18080/api/v1/admin/photos)
echo "$NEW" | jq '.slug, .caption, .alt_text'
# 期望：slug=m34-test, caption / alt_text 都返双语对象

# 13) 前端联调（手动）
# - 浏览器开 /admin
# - 表格上方有搜索框 + 下方有分页器
# - PhotoForm 新建/编辑时多了 slug + caption + alt_text 输入
# - 搜索"tokyo"实时过滤

# 收工
NEW_ID=$(echo "$NEW" | jq -r .id)
curl -s -X DELETE -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/photos/$NEW_ID > /dev/null
kill -INT $PID; wait $PID 2>/dev/null
```

#### 23.4.8 自证扫描

```sh
grep -nE "pub slug:|pub caption:|pub alt_text:" crates/cms-api/src/dto/photo_dto.rs
# 期望：>= 6 行（PhotoDto + PhotoReq 各 3）

grep -E "fn list_admin_paginated|fn list_admin\(" crates/cms-api/src/repositories/photo_repo.rs
# 期望：list_admin_paginated 函数存在

grep -nE "PhotoListResp|PhotoListQuery" crates/cms-api/src/dto/photo_dto.rs | wc -l
# 期望：>= 2

grep -E "type=\"search\"|placeholder=.*search|searchPlaceholder" frontend/src/components/AdminApp.tsx | head -3
# 期望：搜索框存在

cargo check --workspace --all-targets 2>&1 | tail -3
cd frontend && npx tsc --noEmit 2>&1 | tail -3 && cd ..
```

---

### 23.5 PR-M3-5：Dashboard 真统计

> **目标**：给 admin 一个 overview 端点 + 前端 Dashboard tab（默认页），把"现在系统有多少图、什么分布、最近上传啥"一屏说清楚。
>
> **范围有意收窄**：本 PR **不做** 批量操作（多选删/改隐私/打标）—— 留给 PR-M3-6。本 PR 只读不写。
>
> **不引入图表库**：手写 CSS `<div>` 横向 bar（width 百分比即可），不引 recharts / chart.js / d3。

#### 23.5.1 改动文件清单（13 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/dto/dashboard_dto.rs`（**新建**） | `DashboardResp` 类型集（PhotoStats / PrivacyCount / CategoryCount / PhotoSummary / MediaStats） |
| 2 | `crates/cms-api/src/dto/mod.rs` | 加 `pub mod dashboard_dto;` |
| 3 | `crates/cms-api/src/services/dashboard_service.rs`（**新建**） | `overview(state) -> DashboardResp` 一个函数聚合所有统计 |
| 4 | `crates/cms-api/src/services/mod.rs` | 加 `pub mod dashboard_service;` |
| 5 | `crates/cms-api/src/handlers/dashboard_handler.rs`（**新建**） | `get_overview` handler |
| 6 | `crates/cms-api/src/handlers/mod.rs` | 加 `pub mod dashboard_handler;` |
| 7 | `crates/cms-api/src/routes.rs` | `protected_api` 加 `GET /admin/dashboard` |
| 8 | `crates/cms-api/src/repositories/photo_repo.rs` | 新增 `count_by_privacy / count_by_category / list_recent(n)` |
| 9 | `crates/cms-api/src/repositories/media_repo.rs` | 新增 `count_by_status` |
| 10 | `frontend/src/types.ts` | 加 `DashboardResp` 类型集 |
| 11 | `frontend/src/api/client.ts` | 加 `api.dashboard()` |
| 12 | `frontend/src/components/AdminApp.tsx` | 顶栏加 `Dashboard` tab（**设为默认 active**）；新增 `DashboardView` 组件（4 stat 卡 + 2 个 bar 图 + recent 缩略图 strip） |
| 13 | `frontend/src/i18n.ts` | 加 `dashboard / overview / recentUploads / byPrivacy / byCategory / mediaQueue` 等 key |

#### 23.5.2 API 设计

`GET /api/v1/admin/dashboard`（需鉴权）返回：

```json
{
  "photos": {
    "total": 28,
    "by_privacy": { "public": 22, "locked": 3, "private": 3 },
    "by_category": [
      { "slug": "street",    "name": { "zh": "街拍", "en": "Street" },    "count": 8 },
      { "slug": "landscape", "name": { "zh": "风景", "en": "Landscape" }, "count": 11 },
      { "slug": "life",      "name": { "zh": "生活", "en": "Life" },      "count": 9 }
    ],
    "recent": [
      { "id": 30, "slug": "demo-1", "title": {"zh":"...", "en":"..."},
        "src": "http://localhost:9000/...signed...", "created_at": "2026-05-15T..." },
      /* 共 5 条 */
    ]
  },
  "media": {
    "total": 30, "pending": 0, "processing": 0, "ready": 29, "failed": 1
  },
  "users_total": 3,
  "tags_total": 5,
  "categories_total": 4
}
```

#### 23.5.3 DTO 形状

```rust
// dto/dashboard_dto.rs
use serde::Serialize;
use crate::dto::photo_dto::I18nText;

#[derive(Debug, Serialize)]
pub struct DashboardResp {
    pub photos: PhotoStats,
    pub media: MediaStats,
    pub users_total: i64,
    pub tags_total: i64,
    pub categories_total: i64,
}

#[derive(Debug, Serialize)]
pub struct PhotoStats {
    pub total: i64,
    pub by_privacy: PrivacyCount,
    pub by_category: Vec<CategoryCount>,
    pub recent: Vec<PhotoSummary>,
}

#[derive(Debug, Serialize)]
pub struct PrivacyCount {
    pub public: i64,
    pub locked: i64,
    pub private: i64,
}

#[derive(Debug, Serialize)]
pub struct CategoryCount {
    pub slug: String,
    pub name: I18nText,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct PhotoSummary {
    pub id: i64,
    pub slug: String,
    pub title: I18nText,
    pub src: String,                   // cover_url（走 §18 同款变体选择）
    pub created_at: String,            // ISO 8601
}

#[derive(Debug, Serialize)]
pub struct MediaStats {
    pub total: i64,
    pub pending: i64,
    pub processing: i64,
    pub ready: i64,
    pub failed: i64,
}
```

#### 23.5.4 repo 新增函数（SQL 提示）

```rust
// photo_repo
pub async fn count_by_privacy(db: &DatabaseConnection) -> Result<PrivacyCount, DbErr> {
    // SELECT privacy, count(*) FROM photos GROUP BY privacy
    // 用 sea_query 或者 statement_from_string 都行
}

pub async fn count_by_category(db: &DatabaseConnection)
    -> Result<Vec<(String, serde_json::Value, i64)>, DbErr>
{
    // SELECT c.slug, c.name_i18n, count(p.id)
    // FROM categories c LEFT JOIN photos p ON p.category_id=c.id
    // GROUP BY c.id, c.slug, c.name_i18n
    // ORDER BY c.sort_order
}

pub async fn list_recent(db: &DatabaseConnection, n: u32) -> Result<Vec<PhotoFull>, DbErr> {
    // SELECT * FROM photos ORDER BY created_at DESC LIMIT n
    // 复用现有的 PhotoFull join 模式
}

// media_repo
pub async fn count_by_status(db: &DatabaseConnection) -> Result<MediaStats, DbErr> {
    // SELECT status, count(*) FROM media_assets GROUP BY status
    // 转成 MediaStats { total, pending, processing, ready, failed }
}
```

#### 23.5.5 service 聚合

```rust
pub async fn overview(state: &AppState) -> AppResult<DashboardResp> {
    let total          = photo_repo::count(&state.db).await.map_err(db_err)?;
    let by_privacy     = photo_repo::count_by_privacy(&state.db).await.map_err(db_err)?;
    let by_cat_raw     = photo_repo::count_by_category(&state.db).await.map_err(db_err)?;
    let by_category    = by_cat_raw.into_iter().map(|(slug, name_json, count)| {
        let name = serde_json::from_value::<I18nText>(name_json).unwrap_or_default();
        CategoryCount { slug, name, count }
    }).collect();

    // recent 5：用 PhotoFull 组装，src 复用 cover_url_for 选 variant 签 URL
    let recent_fulls = photo_repo::list_recent(&state.db, 5).await.map_err(db_err)?;
    let asset_ids: Vec<i64> = recent_fulls.iter().map(|f| f.asset.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids)
        .await.map_err(db_err)?;
    let mut recent = Vec::with_capacity(recent_fulls.len());
    for f in recent_fulls {
        let v = variants_map.get(&f.asset.id).cloned().unwrap_or_default();
        let cover = photo_service::cover_url_for(state, &f.asset, &v).await?;
        recent.push(PhotoSummary {
            id: f.photo.id,
            slug: f.photo.slug.clone(),
            title: I18nText {
                zh: f.title_zh.clone(),
                en: f.title_en.clone(),
            },
            src: cover,
            created_at: f.photo.created_at.to_rfc3339(),
        });
    }

    let media = media_repo::count_by_status(&state.db).await.map_err(db_err)?;
    let users_total = user_repo::count(&state.db).await.map_err(db_err)?;
    let tags_total = tag_repo::count(&state.db).await.map_err(db_err)?;
    let categories_total = category_repo::count(&state.db).await.map_err(db_err)?;

    Ok(DashboardResp {
        photos: PhotoStats { total, by_privacy, by_category, recent },
        media,
        users_total, tags_total, categories_total,
    })
}
```

> `cover_url_for` 需要从 photo_service 暴露成 `pub`（或抽到 cover 模块）让 dashboard_service 调。

#### 23.5.6 frontend Dashboard 布局

```
┌─────────────────────────────────────────────────────────────┐
│  📊 Dashboard                                                │
├─────────────┬─────────────┬─────────────┬───────────────────┤
│  Photos     │  Users      │  Tags       │  Categories       │
│   28        │   3         │   5         │   4               │
├─────────────┴─────────────┼─────────────┴───────────────────┤
│  按隐私分布                │  按分类分布                       │
│  ▓▓▓▓▓▓▓▓▓ Public  22    │  ▓▓▓▓▓ Street    8               │
│  ▓▓ Locked  3            │  ▓▓▓▓▓▓▓ Landscape 11            │
│  ▓▓ Private 3            │  ▓▓▓▓▓▓ Life     9               │
├──────────────────────────┴──────────────────────────────────┤
│  最近上传                                                     │
│  [thumb][thumb][thumb][thumb][thumb]                         │
├──────────────────────────────────────────────────────────────┤
│  媒资队列：pending 0 · processing 0 · ready 29 · failed 1     │
└──────────────────────────────────────────────────────────────┘
```

CSS bar 实现：

```tsx
function Bar({ label, value, max }: { label: string; value: number; max: number }) {
  const pct = max > 0 ? (value / max) * 100 : 0;
  return (
    <div className="dashboard-bar-row">
      <span className="dashboard-bar-label">{label}</span>
      <div className="dashboard-bar-track">
        <div className="dashboard-bar-fill" style={{ width: `${pct}%` }} />
      </div>
      <span className="dashboard-bar-value">{value}</span>
    </div>
  );
}
```

CSS 加几行 .dashboard-bar-* 进 styles.css 即可。**禁止引入图表库**。

#### 23.5.7 不能动

- `frontend/PublicGallery` 组件
- 任何业务路由（photo / tag / category / user / auth / media）
- schema / migrations
- `bootstrap.rs` / `jwt_guard` / `worker`
- 引入新 deps（chart 类、recharts、d3、chartjs、moment、lodash 等一律禁）

#### DoD（贴 PR 描述）

```sh
# 0) 前提
docker ps --filter "name=gathered-" --format "{{.Names}}"

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3
cd frontend && npx tsc --noEmit 2>&1 | tail -3 && cd ..

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api > /tmp/m35.log 2>&1 &
PID=$!; sleep 3
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

# 3) /admin/dashboard 形状完整
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/dashboard | jq

# 4) keys 全在
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/dashboard | jq 'keys'
# 期望：["categories_total","media","photos","tags_total","users_total"]

# 5) photos.by_privacy 三个键齐
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/dashboard | jq '.photos.by_privacy | keys'
# 期望：["locked","private","public"]

# 6) photos.by_category 不空
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/dashboard | jq '.photos.by_category | length'
# 期望：>= 3

# 7) photos.recent.src 必含 X-Amz-Signature 或 https://（cover_url 算法对了）
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/dashboard \
  | jq -r '.photos.recent[0].src' | head -c 100
echo ""

# 8) media stats sum 一致
curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/dashboard \
  | jq '.media | { sum: (.pending + .processing + .ready + .failed), total }'
# 期望：sum == total

# 9) 无 token 必 401
curl -s -o /dev/null -w "no-token: %{http_code}\n" http://127.0.0.1:18080/api/v1/admin/dashboard
# 期望：401

# 10) 前端联调（手动）
# - 浏览器开 http://localhost:5173/admin
# - 登录后默认进 Dashboard tab
# - 看到 4 stat 卡 + 2 bar 区块 + recent 缩略图条
# - 切到 Photos / Users / Tags / Categories tab 仍正常

kill -INT $PID; wait $PID 2>/dev/null
```

#### 23.5.8 自证扫描

```sh
ls crates/cms-api/src/{handlers/dashboard_handler,services/dashboard_service,dto/dashboard_dto}.rs

grep -E '"/admin/dashboard' crates/cms-api/src/routes.rs

grep -E "pub async fn (count_by_privacy|count_by_category|list_recent)" \
  crates/cms-api/src/repositories/photo_repo.rs

grep -E "pub async fn count_by_status" crates/cms-api/src/repositories/media_repo.rs

grep -E "DashboardView|api\.dashboard|dashboard-bar" frontend/src/components/AdminApp.tsx frontend/src/styles.css | head -5

# ★ 反作弊：禁止引入图表库
grep -E "recharts|chart\.js|chartjs|d3|nivo|echarts|apexcharts" frontend/package.json && echo "❌ 引入了图表库" || echo "✅ 无图表库"
```

---

### 23.6 PR-M3-6：批量操作（多选 + 批删 + 批改隐私 + 批打标）

> **目标**：Photos 表格行级 checkbox 多选 → 顶部"已选 N 张"工具栏 → 三种批量动作：删除 / 改隐私 / 打标。
>
> **范围**：3 个 POST 端点 + 前端表格 multi-select + 3 个批操作模态。

#### 23.6.1 改动文件清单（11 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/dto/photo_dto.rs` | 新增 `BulkDeleteReq` / `BulkPrivacyReq` / `BulkTagsReq` / `BulkResp` |
| 2 | `crates/cms-api/src/repositories/photo_repo.rs` | 新增 `find_existing_ids` / `delete_many` / `update_privacy_many` |
| 3 | `crates/cms-api/src/repositories/tag_repo.rs` | 新增 `assign_many(c, photo_ids, tag_ids, replace)` |
| 4 | `crates/cms-api/src/services/photo_service.rs` | 新增 `bulk_delete` / `bulk_update_privacy` / `bulk_set_tags` |
| 5 | `crates/cms-api/src/handlers/photo_handler.rs` | 新增 3 个 bulk handler |
| 6 | `crates/cms-api/src/routes.rs` | `protected_api` 加 `POST /admin/photos/bulk/{delete,privacy,tags}` |
| 7 | `frontend/src/types.ts` | 加 `BulkDeleteReq` / `BulkPrivacyReq` / `BulkTagsReq` / `BulkResp` |
| 8 | `frontend/src/api/client.ts` | `adminPhotos.bulkDelete / bulkPrivacy / bulkSetTags` |
| 9 | `frontend/src/components/AdminApp.tsx::PhotosView` | 行 checkbox + 全选/反选/清空 + 工具栏（已选 N 张）+ 3 个批操作按钮 + 3 个模态 |
| 10 | `frontend/src/styles.css` | 加 `.bulk-toolbar` `.row-checkbox` 等少量样式 |
| 11 | `frontend/src/i18n.ts` | 加 `bulkSelected / bulkDelete / bulkPrivacy / bulkTags / selectAll / clearSelection / appendMode / replaceMode` 等 |

#### 23.6.2 API 设计

| 方法 | 路径 | 入参 | 返回 |
| --- | --- | --- | --- |
| POST | `/api/v1/admin/photos/bulk/delete` | `BulkDeleteReq` | `BulkResp` |
| POST | `/api/v1/admin/photos/bulk/privacy` | `BulkPrivacyReq` | `BulkResp` |
| POST | `/api/v1/admin/photos/bulk/tags` | `BulkTagsReq` | `BulkResp` |

`BulkResp` 形状：

```json
{ "affected": 5, "skipped": [42, 99] }
```

`affected` = 实际成功操作的数；`skipped` = 在数据库中不存在的 ID（视为已删/无效）。

#### 23.6.3 DTO 形状

```rust
// dto/photo_dto.rs 追加
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct BulkDeleteReq {
    #[validate(length(min = 1, max = 100))]
    pub ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BulkPrivacyReq {
    #[validate(length(min = 1, max = 100))]
    pub ids: Vec<i64>,
    pub privacy: Privacy,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BulkTagsReq {
    #[validate(length(min = 1, max = 100))]
    pub ids: Vec<i64>,
    #[serde(default)]
    pub tag_ids: Vec<i64>,
    #[serde(default = "default_bulk_mode")]
    pub mode: BulkTagMode,    // "replace" or "append"
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BulkTagMode {
    #[default]
    Replace,
    Append,
}

fn default_bulk_mode() -> BulkTagMode { BulkTagMode::Replace }

#[derive(Debug, Serialize)]
pub struct BulkResp {
    pub affected: i64,
    pub skipped: Vec<i64>,
}
```

#### 23.6.4 service 行为

**bulk_delete**:
```rust
pub async fn bulk_delete(state: &AppState, req: BulkDeleteReq) -> AppResult<BulkResp> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let existing = photo_repo::find_existing_ids(&state.db, &req.ids).await.map_err(db_err)?;
    let existing_set: std::collections::HashSet<i64> = existing.iter().copied().collect();
    let skipped: Vec<i64> = req.ids.iter().filter(|id| !existing_set.contains(id)).copied().collect();
    let affected = photo_repo::delete_many(&state.db, &existing).await.map_err(db_err)?;
    Ok(BulkResp { affected, skipped })
}
```

photo_tags / photo_translations 走 ON DELETE CASCADE 自动清；media_assets 不删（可能被其它 photo 引用）。

**bulk_update_privacy**:
```rust
pub async fn bulk_update_privacy(state: &AppState, req: BulkPrivacyReq) -> AppResult<BulkResp> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let existing = photo_repo::find_existing_ids(&state.db, &req.ids).await.map_err(db_err)?;
    let existing_set: std::collections::HashSet<i64> = existing.iter().copied().collect();
    let skipped: Vec<i64> = req.ids.iter().filter(|id| !existing_set.contains(id)).copied().collect();

    // 切到 locked：所有命中 photo 设默认 passcode "1234" 的 argon2 hash
    // 切到 public/private：清空 passcode_hash
    let passcode_hash = if matches!(req.privacy, Privacy::Locked) {
        Some(argon2_hash_passcode("1234")?)
    } else {
        None
    };
    let affected = photo_repo::update_privacy_many(
        &state.db, &existing, req.privacy, passcode_hash
    ).await.map_err(db_err)?;
    Ok(BulkResp { affected, skipped })
}
```

> 切 locked 时 batch 全部用 `"1234"` 默认口令，与单条接口（PR-2）行为一致。admin 后续可单独改单张口令。

**bulk_set_tags**:
```rust
pub async fn bulk_set_tags(state: &AppState, req: BulkTagsReq) -> AppResult<BulkResp> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let existing = photo_repo::find_existing_ids(&state.db, &req.ids).await.map_err(db_err)?;
    let existing_set: std::collections::HashSet<i64> = existing.iter().copied().collect();
    let skipped: Vec<i64> = req.ids.iter().filter(|id| !existing_set.contains(id)).copied().collect();

    let replace = matches!(req.mode, BulkTagMode::Replace);
    let txn = state.db.begin().await.map_err(db_err)?;
    tag_repo::assign_many(&txn, &existing, &req.tag_ids, replace).await.map_err(db_err)?;
    txn.commit().await.map_err(db_err)?;
    Ok(BulkResp { affected: existing.len() as i64, skipped })
}
```

#### 23.6.5 repo 关键函数

```rust
// photo_repo.rs
pub async fn find_existing_ids(db: &DatabaseConnection, ids: &[i64]) -> Result<Vec<i64>, DbErr> {
    if ids.is_empty() { return Ok(vec![]); }
    photos::Entity::find()
        .filter(photos::Column::Id.is_in(ids.to_vec()))
        .select_only().column(photos::Column::Id)
        .into_tuple::<i64>().all(db).await
}

pub async fn delete_many(db: &DatabaseConnection, ids: &[i64]) -> Result<i64, DbErr> {
    if ids.is_empty() { return Ok(0); }
    let res = photos::Entity::delete_many()
        .filter(photos::Column::Id.is_in(ids.to_vec()))
        .exec(db).await?;
    Ok(res.rows_affected as i64)
}

pub async fn update_privacy_many(
    db: &DatabaseConnection, ids: &[i64],
    privacy: Privacy, passcode_hash: Option<String>,
) -> Result<i64, DbErr> {
    if ids.is_empty() { return Ok(0); }
    let res = photos::Entity::update_many()
        .col_expr(photos::Column::Privacy, Expr::value(privacy.as_str()))
        .col_expr(photos::Column::PasscodeHash, Expr::value(passcode_hash))
        .filter(photos::Column::Id.is_in(ids.to_vec()))
        .exec(db).await?;
    Ok(res.rows_affected as i64)
}
```

```rust
// tag_repo.rs
pub async fn assign_many<C: ConnectionTrait>(
    c: &C,
    photo_ids: &[i64],
    tag_ids: &[i64],
    replace: bool,
) -> Result<(), DbErr> {
    if photo_ids.is_empty() { return Ok(()); }
    if replace {
        // 全量替换：先删现有
        photo_tags::Entity::delete_many()
            .filter(photo_tags::Column::PhotoId.is_in(photo_ids.to_vec()))
            .exec(c).await?;
    }
    if tag_ids.is_empty() { return Ok(()); }
    // bulk INSERT；append 模式用 ON CONFLICT DO NOTHING 防主键冲突
    let mut rows = Vec::new();
    for pid in photo_ids {
        for tid in tag_ids {
            rows.push(photo_tags::ActiveModel {
                photo_id: Set(*pid),
                tag_id: Set(*tid),
            });
        }
    }
    photo_tags::Entity::insert_many(rows)
        .on_conflict(sea_query::OnConflict::columns([
            photo_tags::Column::PhotoId, photo_tags::Column::TagId,
        ]).do_nothing().to_owned())
        .exec_without_returning(c).await?;
    Ok(())
}
```

#### 23.6.6 frontend 多选 UX

`PhotosView` 改动：

```tsx
const [selected, setSelected] = useState<Set<number>>(new Set());

const toggle = (id: number) => setSelected(prev => {
  const next = new Set(prev); next.has(id) ? next.delete(id) : next.add(id);
  return next;
});

const selectAll  = () => setSelected(new Set(items.map(p => p.id)));
const clearAll   = () => setSelected(new Set());
const invertSel  = () => setSelected(new Set(items.filter(p => !selected.has(p.id)).map(p => p.id)));

return (
  <>
    {selected.size > 0 && (
      <div className="bulk-toolbar">
        <span>已选 {selected.size} 张</span>
        <button onClick={selectAll}>全选当前页</button>
        <button onClick={invertSel}>反选</button>
        <button onClick={clearAll}>清空</button>
        <span className="bulk-sep" />
        <button className="danger" onClick={() => setBulkAction("delete")}>批删</button>
        <button onClick={() => setBulkAction("privacy")}>改隐私</button>
        <button onClick={() => setBulkAction("tags")}>打标</button>
      </div>
    )}
    <table>...
      <thead>...
        <th><input type="checkbox" checked={selected.size === items.length} onChange={e => e.target.checked ? selectAll() : clearAll()} /></th>
        ...
      </thead>
      <tbody>
        {items.map(p => (
          <tr key={p.id}>
            <td><input type="checkbox" checked={selected.has(p.id)} onChange={() => toggle(p.id)} /></td>
            ...
          </tr>
        ))}
      </tbody>
    </table>

    {bulkAction === "delete"  && <BulkDeleteModal ids={[...selected]} ... />}
    {bulkAction === "privacy" && <BulkPrivacyModal ids={[...selected]} ... />}
    {bulkAction === "tags"    && <BulkTagsModal ids={[...selected]} ... />}
  </>
);
```

3 个模态各自实现（小组件）：
- **BulkDeleteModal**：仅一行 "确认批量删除 N 张？" + 取消/确认
- **BulkPrivacyModal**：单选 radio public/locked/private + 确认
- **BulkTagsModal**：多选 tags checkbox + replace/append 二选一 radio + 确认

操作成功后：刷新表格 + 清空 selected + 显示 toast "已处理 N 张，跳过 M 张"。

#### 23.6.7 不能动

- 公开 `GET /api/v1/photos` 形状（仍 flat array）
- 任何 `GET` 端点行为
- schema / migrations / FK 行为
- `auth / user / category / media` 业务代码（仅 tag_repo + photo_repo + photo_service 加新函数）
- `bootstrap.rs / jwt_guard / cover_url_for / worker / dashboard_service`
- 引入新依赖（不要 use-debounce / lodash / immer 等）

#### DoD（贴 PR 描述）

```sh
# 0) 前提
docker ps --filter "name=gathered-" --format "{{.Names}}"

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3
cd frontend && npx tsc --noEmit 2>&1 | tail -3 && cd ..

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 cargo run -p cms-api > /tmp/m36.log 2>&1 &
PID=$!; sleep 3
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)

# 3) 创建 3 张测试 photo
declare -a TIDS
for i in 1 2 3; do
  R=$(curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
    -d "{\"slug\":\"bulk-test-$i\",\"src\":\"https://example.com/$i.jpg\",\"cat\":\"street\",\"date\":\"2026.05\",\"privacy\":\"public\",\"title\":{\"zh\":\"批量测试 $i\",\"en\":\"Bulk Test $i\"},\"loc\":{\"zh\":\"x\",\"en\":\"x\"}}" \
    http://127.0.0.1:18080/api/v1/admin/photos)
  TIDS+=("$(echo $R | jq -r .id)")
done
IDS_JSON=$(printf '%s\n' "${TIDS[@]}" | jq -R . | jq -s 'map(tonumber)')
echo "created ids: $IDS_JSON"

# 4) bulk privacy → locked
curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d "{\"ids\":$IDS_JSON,\"privacy\":\"locked\"}" \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/privacy | jq
# 期望：{affected:3, skipped:[]}
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT id, privacy, length(passcode_hash) FROM photos WHERE id IN ($(IFS=,; echo "${TIDS[*]}"));"
# 期望：3 行都是 locked + passcode_hash 长度 > 60（argon2 hash）

# 5) 创建 2 个 tag
TAG1=$(curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"slug":"bulk-a","name":{"zh":"批a","en":"Bulk A"}}' http://127.0.0.1:18080/api/v1/admin/tags | jq -r .id)
TAG2=$(curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"slug":"bulk-b","name":{"zh":"批b","en":"Bulk B"}}' http://127.0.0.1:18080/api/v1/admin/tags | jq -r .id)
TAGS_JSON="[$TAG1,$TAG2]"

# 6) bulk tags replace
curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d "{\"ids\":$IDS_JSON,\"tag_ids\":$TAGS_JSON,\"mode\":\"replace\"}" \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/tags | jq
# 期望：{affected:3, skipped:[]}
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT photo_id, count(*) FROM photo_tags WHERE photo_id IN ($(IFS=,; echo "${TIDS[*]}")) GROUP BY photo_id;"
# 期望：3 行每行 count=2

# 7) bulk tags append（再加同 2 个，应去重不报错，count 仍 2）
curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d "{\"ids\":$IDS_JSON,\"tag_ids\":$TAGS_JSON,\"mode\":\"append\"}" \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/tags | jq
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT photo_id, count(*) FROM photo_tags WHERE photo_id IN ($(IFS=,; echo "${TIDS[*]}")) GROUP BY photo_id;"
# 期望：3 行每行 count 仍 2（去重生效）

# 8) bulk delete 含一个不存在的 id (99999)
DEL_JSON=$(echo "$IDS_JSON" | jq '. + [99999]')
curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d "{\"ids\":$DEL_JSON}" \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/delete | jq
# 期望：{affected:3, skipped:[99999]}
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT count(*) FROM photos WHERE id IN ($(IFS=,; echo "${TIDS[*]}"));"
# 期望：0（CASCADE 也清了 photo_tags）
docker exec gathered-pg psql -U postgres -d gathered_light \
  -c "SELECT count(*) FROM photo_tags WHERE photo_id IN ($(IFS=,; echo "${TIDS[*]}"));"
# 期望：0

# 9) 空 ids 必 400
echo -n "empty ids: "
curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' -d '{"ids":[]}' \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/delete

# 10) 超 100 ids 必 400
echo -n "too many ids: "
BIG=$(seq 1 101 | jq -s .)
curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Authorization: Bearer $TOK" \
  -H 'content-type: application/json' -d "{\"ids\":$BIG}" \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/delete

# 11) 无 token 必 401
curl -s -o /dev/null -w "no-token: %{http_code}\n" -X POST \
  http://127.0.0.1:18080/api/v1/admin/photos/bulk/delete \
  -H 'content-type: application/json' -d '{"ids":[1]}'

# 12) 清理 tags
for tid in $TAG1 $TAG2; do
  curl -s -X DELETE -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/tags/$tid > /dev/null
done

# 13) 前端联调（手动）
# - /admin Photos tab 表格行有 checkbox + 顶部"已选 N"工具栏
# - 选 2 张 → 批删，确认 → 表格刷新少 2 行
# - 选 3 张 → 批改隐私 locked，提示成功
# - 选 N 张 → 打标，多选 tags + replace/append 单选

kill -INT $PID; wait $PID 2>/dev/null
```

#### 23.6.8 自证扫描

```sh
grep -E "fn bulk_delete|fn bulk_update_privacy|fn bulk_set_tags" \
  crates/cms-api/src/services/photo_service.rs | wc -l
# 期望：3

grep -E "fn (delete_many|update_privacy_many|find_existing_ids)" \
  crates/cms-api/src/repositories/photo_repo.rs | wc -l
# 期望：3

grep "fn assign_many" crates/cms-api/src/repositories/tag_repo.rs

grep -E '"/admin/photos/bulk' crates/cms-api/src/routes.rs | wc -l
# 期望：3

grep -E "BulkDeleteReq|BulkPrivacyReq|BulkTagsReq|BulkResp" frontend/src/types.ts | wc -l
# 期望：>= 4

grep -E "bulkDelete|bulkPrivacy|bulkSetTags|bulk-toolbar" frontend/src/components/AdminApp.tsx | head -5

cargo check --workspace --all-targets 2>&1 | tail -3
cd frontend && npx tsc --noEmit 2>&1 | tail -3 && cd ..
```

---

### 23.7 PR-M3-7：photo 列表 cache-aside（M3-CMS-Full 收官）

> **目标**：把高频公开端点（photo list / categories / tags）走 Redis cache-aside；任何写操作后主动失效。**Redis 终于真用上**（之前只用于 JWT 黑名单）。
>
> **范围有意收窄**：
> - **只缓存 3 个公开 GET**：`/api/v1/photos`、`/api/v1/categories`、`/api/v1/tags`
> - admin endpoints **不缓存**（admin 总要读最新）
> - 单图详情 `/api/v1/photos/:id/unlock` **不缓存**（已限流）
> - 验证用 redis-cli 看 key 真在，不依赖日志级别

#### 23.7.1 改动文件清单（9 步）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `crates/cms-api/src/infra/cache.rs`（**新建**） | `get<T>` / `set<T>` / `del_one` / `invalidate_prefix` 4 个通用函数 |
| 2 | `crates/cms-api/src/infra/mod.rs` | 加 `pub mod cache;` |
| 3 | `crates/cms-api/src/services/photo_service.rs::list_public` | cache-aside 包装（key=`cms:photo:list:cat={x}`，TTL=60s） |
| 4 | `crates/cms-api/src/services/photo_service.rs::所有写函数` | 末尾 `cache::invalidate_prefix(&state.redis, "cms:photo:list:*")`（create/update/update_privacy/delete/reset/bulk_*） |
| 5 | `crates/cms-api/src/services/category_service.rs::list_public` | cache-aside 单 key `cms:taxonomy:categories`，TTL=600s |
| 6 | `crates/cms-api/src/services/category_service.rs::create/update/delete` | 末尾失效 `cms:taxonomy:categories` + `cms:photo:list:*` |
| 7 | `crates/cms-api/src/services/tag_service.rs::list_public_tags` | cache-aside 单 key `cms:taxonomy:tags`，TTL=600s |
| 8 | `crates/cms-api/src/services/tag_service.rs::create/update/delete` 与 `photo_service::bulk_set_tags` | 失效 `cms:taxonomy:tags` + `cms:photo:list:*`（因为 photo.tags 字段嵌在 list 缓存里） |
| 9 | (无前端改动) | 缓存对前端透明 |

#### 23.7.2 缓存 key 设计

| Key 模板 | 内容 | TTL |
| --- | --- | --- |
| `cms:photo:list:cat={all\|street\|landscape\|life\|...}` | `serde_json::to_string(&Vec<PhotoDto>)` | 60s |
| `cms:taxonomy:categories` | `serde_json::to_string(&Vec<CategoryDto>)` | 600s（10 min；分类不常变） |
| `cms:taxonomy:tags` | `serde_json::to_string(&Vec<TagDto>)` | 600s |

写后失效矩阵严格按 §15。

#### 23.7.3 infra/cache.rs 完整模板

```rust
//! 通用 cache-aside 工具（≈ Spring Cache 的辅助层）。
//! 所有 cache 错误**不抛出 500**——降级为日志 + 回源（保证 cache 故障不让业务挂）。

use fred::clients::RedisPool;
use fred::interfaces::KeysInterface;
use fred::types::Expiration;
use futures::StreamExt;
use serde::{Serialize, de::DeserializeOwned};

/// 读：JSON 反序列化；任何错误返 None 走回源
pub async fn get<T: DeserializeOwned>(redis: &RedisPool, key: &str) -> Option<T> {
    match redis.get::<Option<String>, _>(key).await {
        Ok(Some(raw)) => match serde_json::from_str::<T>(&raw) {
            Ok(v) => Some(v),
            Err(e) => { tracing::warn!(error = ?e, key, "cache decode failed"); None }
        },
        Ok(None) => None,
        Err(e) => { tracing::warn!(error = ?e, key, "cache get failed"); None }
    }
}

/// 写：JSON 序列化 + EX TTL；失败仅日志（不影响业务）
pub async fn set<T: Serialize>(redis: &RedisPool, key: &str, value: &T, ttl_secs: i64) {
    let json = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => { tracing::warn!(error = ?e, key, "cache encode failed"); return; }
    };
    if let Err(e) = redis
        .set::<(), _, _>(key, json, Some(Expiration::EX(ttl_secs)), None, false)
        .await
    {
        tracing::warn!(error = ?e, key, "cache set failed");
    }
}

/// 删单 key（用于精确失效，如 cms:taxonomy:categories）
pub async fn del_one(redis: &RedisPool, key: &str) {
    if let Err(e) = redis.del::<i64, _>(key).await {
        tracing::warn!(error = ?e, key, "cache del failed");
    }
}

/// SCAN + UNLINK 清前缀（不用 KEYS 防阻塞主线程）
pub async fn invalidate_prefix(redis: &RedisPool, pattern: &str) {
    let client = redis.next();
    let mut scan = client.scan(pattern, Some(100), None);
    let mut keys: Vec<String> = Vec::new();
    while let Some(page) = scan.next().await {
        match page {
            Ok(mut p) => {
                if let Some(results) = p.take_results() {
                    keys.extend(results.into_iter().filter_map(|v| v.into_string()));
                }
            }
            Err(e) => { tracing::warn!(error = ?e, pattern, "cache scan failed"); return; }
        }
    }
    if !keys.is_empty() {
        let count = keys.len();
        if let Err(e) = redis.unlink::<i64, _>(keys).await {
            tracing::warn!(error = ?e, pattern, "cache unlink failed");
        } else {
            tracing::debug!(pattern, count, "cache invalidated");
        }
    }
}
```

> fred 9.x `scan` API 可能略不同——如果 `take_results()` / `into_string()` 名字对不上，参考 fred 文档微调；**逻辑保持不变**。如果实在不行，回退到先用 `KEYS pattern` 拿全列表（dev 阶段 keys 很少不会阻塞），生产再迭代。

#### 23.7.4 service 改造样例

**photo_service::list_public**：

```rust
pub async fn list_public(state: &AppState, q: PhotoQuery) -> AppResult<Vec<PhotoDto>> {
    let cat = q.category.as_deref().unwrap_or("all");
    let key = format!("cms:photo:list:cat={cat}");

    // 1) 查 cache
    if let Some(hit) = cache::get::<Vec<PhotoDto>>(&state.redis, &key).await {
        tracing::debug!(cache = "hit", %key);
        return Ok(hit);
    }
    tracing::debug!(cache = "miss", %key);

    // 2) 回源（既有逻辑保留）
    let fulls = photo_repo::list_public(&state.db, q.category.as_deref()).await.map_err(db_err)?;
    let asset_ids: Vec<i64> = fulls.iter().map(|f| f.asset.id).collect();
    let photo_ids: Vec<i64> = fulls.iter().map(|f| f.photo.id).collect();
    let variants_map = media_repo::find_variants_for_assets(&state.db, &asset_ids).await.map_err(db_err)?;
    let tags_map     = tag_repo::find_for_photos(&state.db, &photo_ids).await.map_err(db_err)?;

    let mut out = Vec::with_capacity(fulls.len());
    for f in fulls {
        let v = variants_map.get(&f.asset.id).cloned().unwrap_or_default();
        let t = tags_map.get(&f.photo.id).cloned().unwrap_or_default();
        let cover = cover_url_for(state, &f.asset, &v).await?;
        out.push(assemble_dto(f, cover, t));
    }

    // 3) 回写
    cache::set(&state.redis, &key, &out, 60).await;
    Ok(out)
}
```

**所有写函数末尾追加**（photo_service create/update/update_privacy/delete/reset/bulk_delete/bulk_update_privacy/bulk_set_tags）：

```rust
cache::invalidate_prefix(&state.redis, "cms:photo:list:*").await;
```

**category_service / tag_service 同理**，按 §23.7.2 的 key + §15 失效矩阵。

#### 23.7.5 不能动

- `frontend/` 任何文件（cache 对前端透明）
- admin 端点（不缓存）
- `auth / user / media / dashboard` service / handler 业务代码
- `photo_repo / tag_repo / category_repo` 任何函数
- schema / migrations / routes.rs
- `bootstrap.rs / jwt_guard / worker / cover_url_for`

#### DoD（贴 PR 描述）

```sh
# 0) 前提
docker ps --filter "name=gathered-" --format "{{.Names}}"

# 1) 编译
cargo check --workspace --all-targets 2>&1 | tail -3

# 2) 起 cms-api
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info cargo run -p cms-api > /tmp/m37.log 2>&1 &
PID=$!; sleep 3

# 3) 清缓存基线
docker exec gathered-redis redis-cli FLUSHDB | head
docker exec gathered-redis redis-cli KEYS 'cms:*' | wc -l
# 期望：0

# 4) 第一次 GET /photos → 写缓存
curl -s "http://127.0.0.1:18080/api/v1/photos" > /dev/null
docker exec gathered-redis redis-cli KEYS 'cms:photo:list:*'
# 期望：1 行 cms:photo:list:cat=all

# 5) 第二次 GET 同 URL → 应当命中（验证 key 仍在 + TTL > 0）
curl -s "http://127.0.0.1:18080/api/v1/photos" > /dev/null
docker exec gathered-redis redis-cli TTL 'cms:photo:list:cat=all'
# 期望：1..60 秒

# 6) 带 cat 参数 → 另一 key
curl -s "http://127.0.0.1:18080/api/v1/photos?category=street" > /dev/null
docker exec gathered-redis redis-cli KEYS 'cms:photo:list:*' | sort
# 期望：2 行 cat=all + cat=street

# 7) categories 同理
curl -s "http://127.0.0.1:18080/api/v1/categories" > /dev/null
docker exec gathered-redis redis-cli KEYS 'cms:taxonomy:*'
# 期望：cms:taxonomy:categories

# 8) tags 同理
curl -s "http://127.0.0.1:18080/api/v1/tags" > /dev/null
docker exec gathered-redis redis-cli KEYS 'cms:taxonomy:*' | sort
# 期望：cms:taxonomy:categories + cms:taxonomy:tags

# 9) 写后失效：admin 改 photo → cms:photo:list:* 必清
TOK=$(curl -s -X POST http://127.0.0.1:18080/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@gathered.local","password":"Passw0rd!"}' | jq -r .access_token)
PHOTO_ID=$(curl -s -H "Authorization: Bearer $TOK" "http://127.0.0.1:18080/api/v1/admin/photos?page_size=1" | jq -r '.items[0].id')
curl -s -X PATCH -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"privacy":"public"}' \
  http://127.0.0.1:18080/api/v1/admin/photos/$PHOTO_ID/privacy > /dev/null
echo -n "after photo write, list keys: "
docker exec gathered-redis redis-cli KEYS 'cms:photo:list:*' | wc -l | tr -d ' '
# 期望：0

# 10) 写后失效：admin 改 category → categories + photo:list:* 都清
curl -s "http://127.0.0.1:18080/api/v1/categories" > /dev/null
curl -s "http://127.0.0.1:18080/api/v1/photos" > /dev/null  # 重新填两个 cache
CID=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/categories | jq -r '.[0].id')
curl -s -X PATCH -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"sort_order":99}' http://127.0.0.1:18080/api/v1/admin/categories/$CID > /dev/null
echo -n "after category write, taxonomy:categories: "
docker exec gathered-redis redis-cli EXISTS 'cms:taxonomy:categories' | tr -d ' '
# 期望：0
echo -n "after category write, photo:list keys: "
docker exec gathered-redis redis-cli KEYS 'cms:photo:list:*' | wc -l | tr -d ' '
# 期望：0

# 11) 写后失效：admin 加 tag → taxonomy:tags + photo:list 都清
curl -s "http://127.0.0.1:18080/api/v1/tags" > /dev/null
curl -s "http://127.0.0.1:18080/api/v1/photos" > /dev/null
curl -s -X POST -H "Authorization: Bearer $TOK" -H 'content-type: application/json' \
  -d '{"slug":"m37-cache-test","name":{"zh":"缓存测试","en":"Cache Test"}}' \
  http://127.0.0.1:18080/api/v1/admin/tags > /dev/null
echo -n "after tag write, taxonomy:tags: "
docker exec gathered-redis redis-cli EXISTS 'cms:taxonomy:tags' | tr -d ' '
# 期望：0
echo -n "after tag write, photo:list keys: "
docker exec gathered-redis redis-cli KEYS 'cms:photo:list:*' | wc -l | tr -d ' '
# 期望：0

# 12) cache 故障降级：暂停 Redis 仍能 GET /photos（200，仅回源不写缓存）
docker compose stop redis > /dev/null 2>&1
sleep 1
curl -s -o /dev/null -w "redis down GET /photos: %{http_code}\n" http://127.0.0.1:18080/api/v1/photos
# 期望：HTTP 200（cache get failed → 回源 → 写缓存 fail 也仅 warn）
docker compose start redis > /dev/null 2>&1; sleep 2

# 13) 清理测试 tag
TID=$(curl -s -H "Authorization: Bearer $TOK" http://127.0.0.1:18080/api/v1/admin/tags \
  | jq -r '.items[] | select(.slug=="m37-cache-test") | .id')
[ -n "$TID" ] && curl -s -X DELETE -H "Authorization: Bearer $TOK" \
  http://127.0.0.1:18080/api/v1/admin/tags/$TID > /dev/null

kill -INT $PID; wait $PID 2>/dev/null
```

#### 23.7.6 自证扫描

```sh
ls crates/cms-api/src/infra/cache.rs
grep -E "pub async fn (get|set|del_one|invalidate_prefix)" crates/cms-api/src/infra/cache.rs

# 至少 8 处 invalidate_prefix（photo 写函数 + category + tag）
grep -c "cache::invalidate_prefix\|cache::del_one\|cache::invalidate" \
  crates/cms-api/src/services/{photo,category,tag}_service.rs

# 3 处 cache::get + 3 处 cache::set（photo list + categories + tags）
grep -c "cache::get\|cache::set" \
  crates/cms-api/src/services/{photo,category,tag}_service.rs

# cache 错误不抛 500（grep 不应见 AppError::Internal 在 cache 模块内）
grep "AppError::Internal" crates/cms-api/src/infra/cache.rs || echo "✅ cache 内零 AppError"

cargo check --workspace --all-targets 2>&1 | tail -3
```

---

## 23.99 M3-CMS-Full 收官清单（PR-M3-7 完成后填）

| PR | 主题 | 状态 |
| --- | --- | --- |
| PR-M3-1 | 用户管理 + 登录 email | ✅ |
| PR-M3-2 | Tags 子系统 | ✅ |
| PR-M3-3 | Categories 后台 CRUD | ✅ |
| PR-M3-4 | Photo 元数据 + 分页 + 搜索 | ✅ |
| PR-M3-5 | Dashboard 真统计 | ✅ |
| PR-M3-6 | 批量操作 | ✅ |
| PR-M3-7 | photo 列表 cache-aside | 🟡 in progress |

---

## 附：心智速查

> 第二天回头打开这份稿子时，你只需要记住这 5 件事：

1. **Axum = Spring MVC，但参数靠 Extractor 类型注入**；中间件 = Tower Layer，洋葱模型。
2. **AppState 是 Spring 的 IoC 容器**——只不过手动 `Arc::clone` 传，没反射。
3. **缓存不是注解，是 service 层显式三步**：查 → 回源 → 回写；写后 SCAN+UNLINK。
4. **OSS 永远直传**，后端只签名 + 校验 + 异步处理；图片 worker 单独 `tokio::spawn`。
5. **JWT 双失效**：`auth:blacklist:{jti}` 拉黑单 token；`auth:user-rev:{uid}` 拉黑全设备。

—— 完。
