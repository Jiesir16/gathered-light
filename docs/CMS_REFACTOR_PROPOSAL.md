# 拾光集 · 从「图片 CMS」到「仿 WordPress Headless CMS」改造方案

> 读者：资深 Java/Spring 后端、正在实战 Rust 的全栈架构师。
> 关系：本文是 `docs/ARCHITECTURE.md`（现状 SSOT）的**演进提案**，不替代它。文中所有新表/新接口/新目录都标注了与现有代码的衔接点；凡会破坏现有前端契约（§9 of ARCHITECTURE）之处都显式标红说明。
> 目标：在不推翻现有 Axum + SeaORM + Redis + S3 骨架的前提下，补齐三件事——
> 1. **真正的内容模型**（文章 / 页面 / 图片统一为 Post，仿 WordPress）；
> 2. **富文本文章编辑**（TipTap，前台可读、后台可编、可版本回溯）；
> 3. **可插拔主题系统**（主题包 + 设计令牌，集成 [awesome-design-md](https://github.com/VoltAgent/awesome-design-md) 的 `DESIGN.md` 作为主题来源）。
>
> **配套文档**：用户中心 / 身份认证 / 权限 / 多租户隔离见 [`docs/USER_CENTER_AND_IAM_DESIGN.md`](USER_CENTER_AND_IAM_DESIGN.md)。由于已确认走**多租户硬隔离**，本文 §2 的 `posts`/`terms`/`media_assets`/`site_settings` 等表在落地时都会增加 `tenant_id`/`site_id` 维度并启用 Postgres RLS（详见 IAM 文档 §2.4），建表时一次带上、避免二次加列回填。

---

## 目录

- [§0 现状评估（诚实版）](#0-现状评估诚实版)
- [§1 目标：对标 WordPress 的能力地图](#1-目标对标-wordpress-的能力地图)
- [§2 内容模型改造：三种策略对比与推荐](#2-内容模型改造三种策略对比与推荐)
- [§3 富文本文章编辑（TipTap）链路](#3-富文本文章编辑tiptap链路)
- [§4 可插拔主题引擎 + awesome-design-md 集成](#4-可插拔主题引擎--awesome-design-md-集成)
- [§5 API / 缓存 / 安全增量](#5-api--缓存--安全增量)
- [§6 前端架构改造](#6-前端架构改造)
- [§7 实施路线图（M2 → M6）](#7-实施路线图m2--m6)
- [§8 新增依赖清单](#8-新增依赖清单)
- [附录 A：核心代码骨架索引](#附录-a核心代码骨架索引)

---

## §0 现状评估（诚实版）

当前仓库是一套**以「图片」为唯一内容形态**的 CMS，工程质量不错，但内容模型被「一张图 + 元数据」这个假设焊死了：

| 维度 | 现状 | 对「仿 WordPress」的缺口 |
| --- | --- | --- |
| 内容主体 | `photos` 表 = 一张图卡（slug / 分类 / 主图 asset / privacy / 拍摄日期） | 没有「正文 body」概念，无法写文章/页面 |
| 文本字段 | `photo_translations`（title / location / caption / alt_text，均为短文本 i18n） | 没有长文正文、摘要、富文本 |
| 分类法 | `categories`（FK 挂在 photos 上）+ `tags`（`photo_tags` 多对多），且 service 里 `["street","landscape","life"]` **硬编码校验** | 分类法与「图片」强耦合，不能给文章复用；非层级、不可扩展为自定义分类 |
| 主题 | `site_settings` 里一个 `theme` 键，值仅 `warm/cool/bold`（`settings_dto.rs::validate_theme` 写死） | 不是主题系统，只是 3 个配色开关；无法新增主题、无法换版式 |
| 站点设置 | `site_settings`（key/value 文本表）已存在，存了 theme/range/hero/brand_effect | ✅ 这张表是好底子，正好对应 WP `wp_options`，主题系统可复用 |
| 媒体库 | `media_assets` + `media_variants`（OSS 直传 + 异步生成 4 变体 + EXIF） | ✅ 这套是亮点，远超 WP 原生；保留并复用 |
| 鉴权 | JWT + Redis 黑名单 + owner/editor/viewer | ✅ 够用，文章/页面权限沿用 |

**一句话结论**：底座（Axum 分层、SeaORM、Redis cache-aside、OSS 直传、tracing）**全部保留**；要动的是「内容模型 + 分类法 + 主题」这三块上层抽象。下面逐块给方案。

> Spring 类比：相当于你有一套成熟的 `spring-boot` 工程，Controller/Service/Mapper 都在，但 domain 只有一个 `Photo` 聚合根。现在要把它抽象成 `Post` 聚合根 + 可扩展 `PostType`，并把 `Theme` 从一个枚举字段升级成一个可插拔的「starter 模块」。

---

## §1 目标：对标 WordPress 的能力地图

把 WordPress 的核心对象映射到本项目，明确「要做」「已具备」「暂缓」：

| WordPress 概念 | 本项目落点 | 状态 |
| --- | --- | --- |
| `wp_posts`（post/page/attachment 统一表） | 新 `posts` 表（`post_type` 区分 post/page/photo） | 🔨 §2 新建 |
| `post_content`（正文） | `post_translations.body_json` + `body_html`（TipTap） | 🔨 §3 新建 |
| `post_status`（draft/publish/future/private/trash） | `posts.status` | 🔨 §2 新建 |
| `post_password`（密码保护文章） | 复用现有 `passcode_hash`（locked 语义） | ✅ 已有，平移 |
| `wp_postmeta`（EAV 扩展字段） | `post_meta`（可选，放 EXIF/相机等非结构化字段） | 🔨 §2 可选 |
| `wp_terms`/`term_taxonomy`/`term_relationships`（统一分类法） | `terms` + `term_taxonomies` + `post_terms` | 🔨 §2 新建（替代 categories/tags 硬编码） |
| `wp_options`（站点设置） | 现有 `site_settings` | ✅ 已有，扩展 |
| 主题 `wp-content/themes/<slug>/` | `themes/<slug>/`（theme.toml + DESIGN.md + tokens.json） | 🔨 §4 新建 |
| 主题切换 / 定制器 Customizer | `active_theme` + `theme_mods:<slug>`（存 site_settings） | 🔨 §4 新建 |
| 媒体库 | `media_assets`/`media_variants` | ✅ 已有，更强 |
| 评论 `wp_comments` | `comments`（暂缓到 M6） | ⏸ 暂缓 |
| 菜单 `nav_menu` | `terms` 的一种 taxonomy 或独立 `menus` | ⏸ M4 |
| 固定链接 Permalink | `posts.slug` + 路由规则 | 🔨 §2 |
| 插件 Hooks | 暂不做（Rust 无运行时插件，靠 trait + feature 编译期扩展） | ⏸ 不做 |

**不照搬 WordPress 的地方（刻意取舍）**：

- **不做运行时插件/钩子系统**。WP 的 `add_action/add_filter` 依赖 PHP 动态加载；Rust 是编译期语言，强行做会引入 wasm 沙箱等重型复杂度。扩展点用 **trait + 编译期注册** 实现（见 §4 模板主题）。
- **不做 EAV 为主的数据模型**。WP `postmeta` 什么都往 EAV 塞导致查询地狱。本项目**结构化字段优先**，EAV（`post_meta`）只作非检索的补充。

---

## §2 内容模型改造：三种策略对比与推荐

这是整个改造**最核心的架构岔路**。先给结论，再展开。

### 2.1 三种策略对比

| 维度 | A. 统一 Post 模型（仿 WP） | B. 文章/照片并存 | C. 统一抽象 + 渐进迁移 |
| --- | --- | --- | --- |
| 做法 | 新建 `posts`，`post_type ∈ {post,page,photo}`，把现有 photos 数据迁进来 | 保留 `photos` 不动，另起 `articles` 表 | 目标是 A，但用「绞杀者」分阶段迁移：先并行、再切流、最后下线旧表 |
| 代码路径 | 一套（PostService 泛化） | 两套（PhotoService + ArticleService 长期并存） | 过渡期两套，终态一套 |
| 扩展性（加 post_type） | 强：新增类型只加枚举 + 模板 | 弱：每种内容都要新表新 service | 强（终态同 A） |
| 与 WP 心智一致 | 高 | 低（不像 WP） | 高 |
| 迁移风险 | 中高（一次性动 photos 主流程） | 低（不碰旧表） | 低→可控（每阶段可回滚，photos 接口始终可用） |
| 前端契约冲击 | 中（需 photo 兼容 DTO） | 无 | 低（兼容层保留旧 `/photos`） |
| 一次性工作量 | 大 | 小（但长期债大） | 中（分摊到多个 PR） |

### 2.2 推荐：**A 的终态 + C 的迁移路径**

理由：

1. **B 是技术债陷阱**。两套内容、两套分类法、两套缓存失效、两套 i18n 折叠逻辑，半年后必然分叉到无法统一。你要的是「仿 WordPress」，而 WP 的精髓恰恰是 *Everything is a post*。
2. **A 是正确终态**，但「一次性把 photos 主流程换掉」风险高——现有 OSS ACL 同步、variants URL 拼装、unlock 限流都绑在 photo 流程上。
3. 所以用 **C 的绞杀者模式**落地 A：
   - 阶段 1：新建 `posts`/`post_translations`/统一分类法，**把 photos 作为 `post_type='photo'` 的数据写入**，旧 `photos` 表保留为视图或只读兼容。
   - 阶段 2：`/api/v1/photos`（前台）改为「查 posts where post_type='photo'」，DTO 形状保持不变（前端零改动）。
   - 阶段 3：新增 `/api/v1/posts`（文章）走同一套 PostService。
   - 阶段 4：确认无回归后，下线旧 `photos` 表（或永久留作历史）。

> Spring 类比：把 `PhotoRepository` 重构为 `PostRepository<PhotoView>`，对外 `/photos` 接口签名不变（适配器模式），内部数据源切到统一 `posts` 表。调用方无感。

### 2.3 目标 Schema（DDL 草案，新迁移 `m20260701_000004_content_model`）

> 写法沿用现有约定：SQL 放 `migrations/src/sql/`，用 `include_str!` 包进 `.rs`，注册到 `migrations/src/lib.rs` 的 `Migrator`（见现有 `m20260529_000003_site_settings`）。
>
> **多租户增量**（已确认走硬隔离）：下方每张内容表都应再加 `tenant_id BIGINT NOT NULL` 与（内容表）`site_id BIGINT NOT NULL`，且 `posts` 的唯一约束改为按站点 `UNIQUE (site_id, post_type, slug)`、列表索引前缀加 `tenant_id`，并对各表 `ENABLE ROW LEVEL SECURITY` + 租户隔离策略。完整 DDL/RLS/迁移见 [`USER_CENTER_AND_IAM_DESIGN.md`](USER_CENTER_AND_IAM_DESIGN.md) §2.4 / §2.6 / §12。为聚焦内容模型本身，下文 DDL 暂以单租户形态展示。

```sql
-- ─────────────────────────────────────────────────────────────
-- posts：统一内容主体（对标 wp_posts）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE posts (
    id                 BIGSERIAL PRIMARY KEY,
    post_type          TEXT NOT NULL DEFAULT 'post'
                       CHECK (post_type IN ('post','page','photo')),     -- 可扩展
    slug               TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'draft'
                       CHECK (status IN ('draft','published','scheduled','private','trash')),
    -- 可见性：沿用现有 privacy 语义，public/locked/private 与前端契约一致
    visibility         TEXT NOT NULL DEFAULT 'public'
                       CHECK (visibility IN ('public','locked','private')),
    passcode_hash      TEXT,                       -- locked 时用（平移自 photos.passcode_hash）
    author_id          BIGINT REFERENCES users(id),
    parent_id          BIGINT REFERENCES posts(id) ON DELETE SET NULL,   -- page 层级 / 附属关系
    -- 图片型内容的主图（photo 专用，文章可空）
    primary_asset_id   BIGINT REFERENCES media_assets(id),
    featured_asset_id  BIGINT REFERENCES media_assets(id),               -- 文章封面图
    -- 排序与时间
    menu_order         BIGINT NOT NULL DEFAULT 0,                         -- 对标 wp menu_order
    taken_at_label     TEXT NOT NULL DEFAULT '',                          -- photo 专用，兼容旧字段
    taken_at_date      DATE,
    published_at       TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- 同一类型下 slug 唯一（允许 photo 与 page 同 slug，靠 type 区分路由）
    UNIQUE (post_type, slug)
);
CREATE INDEX idx_posts_listing
    ON posts (post_type, status, visibility, menu_order DESC, published_at DESC);
CREATE INDEX idx_posts_published
    ON posts (post_type, published_at DESC) WHERE status = 'published' AND visibility = 'public';

-- ─────────────────────────────────────────────────────────────
-- post_translations：i18n 内容（对标 photo_translations，增加正文字段）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE post_translations (
    id          BIGSERIAL PRIMARY KEY,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    locale      TEXT NOT NULL,                      -- zh / en / ...
    title       TEXT NOT NULL DEFAULT '',
    excerpt     TEXT,                               -- 摘要（可由正文自动截取）
    body_json   JSONB,                              -- ★ TipTap/ProseMirror 文档（编辑真源）
    body_html   TEXT,                               -- ★ 服务端清洗后的渲染 HTML（前台直接用）
    -- photo 专用短字段（文章可空，保持与旧 photo_translations 对齐）
    location    TEXT NOT NULL DEFAULT '',
    caption     TEXT,
    alt_text    TEXT,
    word_count  INT NOT NULL DEFAULT 0,             -- 阅读时长估算用
    UNIQUE (post_id, locale)
);
CREATE INDEX idx_post_tr_locale ON post_translations(locale);

-- ─────────────────────────────────────────────────────────────
-- 统一分类法（对标 wp_terms + term_taxonomy + term_relationships）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE terms (
    id          BIGSERIAL PRIMARY KEY,
    slug        TEXT UNIQUE NOT NULL,
    name_i18n   JSONB NOT NULL                      -- {"zh":"街拍","en":"Street"}
);

CREATE TABLE term_taxonomies (
    id          BIGSERIAL PRIMARY KEY,
    term_id     BIGINT NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
    taxonomy    TEXT NOT NULL                       -- 'category' | 'tag' | 'series' | 自定义
                CHECK (taxonomy IN ('category','tag','series')),
    parent_id   BIGINT REFERENCES term_taxonomies(id) ON DELETE SET NULL,  -- 层级分类
    sort_order  INT NOT NULL DEFAULT 0,
    UNIQUE (term_id, taxonomy)
);
CREATE INDEX idx_tt_taxonomy ON term_taxonomies(taxonomy);

CREATE TABLE post_terms (
    post_id            BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    term_taxonomy_id   BIGINT NOT NULL REFERENCES term_taxonomies(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, term_taxonomy_id)
);

-- ─────────────────────────────────────────────────────────────
-- post_revisions：TipTap 正文版本快照（对标 wp revision）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE post_revisions (
    id          BIGSERIAL PRIMARY KEY,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    locale      TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    body_json   JSONB,
    author_id   BIGINT REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_revisions_post ON post_revisions(post_id, created_at DESC);

-- ─────────────────────────────────────────────────────────────
-- post_meta：非检索型扩展字段（对标 wp_postmeta，按需用）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE post_meta (
    id        BIGSERIAL PRIMARY KEY,
    post_id   BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    meta_key  TEXT NOT NULL,
    meta_value JSONB,
    UNIQUE (post_id, meta_key)
);
```

### 2.4 photos → posts 数据迁移（同一迁移内的 DML）

```sql
-- 1) 旧 photos 平移为 post_type='photo'
INSERT INTO posts (id, post_type, slug, status, visibility, passcode_hash,
                   primary_asset_id, menu_order, taken_at_label, taken_at_date,
                   published_at, created_at, updated_at)
SELECT id, 'photo', slug,
       CASE WHEN published_at IS NULL THEN 'draft' ELSE 'published' END,
       privacy, passcode_hash, primary_asset_id, sort_order,
       taken_at_label, taken_at_date, published_at, created_at, updated_at
FROM photos;
SELECT setval('posts_id_seq', (SELECT COALESCE(MAX(id),1) FROM posts));

-- 2) 翻译平移
INSERT INTO post_translations (post_id, locale, title, location, caption, alt_text)
SELECT photo_id, locale, title, location, caption, alt_text FROM photo_translations;

-- 3) 分类迁移：categories → terms + term_taxonomies(category)
INSERT INTO terms (slug, name_i18n) SELECT slug, name_i18n FROM categories;
INSERT INTO term_taxonomies (term_id, taxonomy, sort_order)
SELECT t.id, 'category', c.sort_order
FROM categories c JOIN terms t ON t.slug = c.slug;

-- 4) tags → terms + term_taxonomies(tag)，photo_tags → post_terms
INSERT INTO terms (slug, name_i18n) SELECT slug, name_i18n FROM tags
ON CONFLICT (slug) DO NOTHING;
-- （photo 与 category 的关联：photos.category_id → post_terms，略，见迁移脚本完整版）
```

> ⚠️ **前端契约影响**：迁移后 `/api/v1/photos` 仍要返回原 `PhotoDto` 形状（`src/cat/title/loc/date/privacy/variants/...`，见 `frontend/src/types.ts::Photo`）。做法是 PhotoService 内部从 posts 取数后**组装成旧 DTO**，对前端零改动。新 `/api/v1/posts` 才用新的 PostDto。

### 2.5 SeaORM Entity 骨架（`cms-entity/src/posts.rs` 等，sea-orm-cli 可直出）

```rust
// crates/cms-entity/src/posts.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub post_type: String,       // service 层用 cms_domain::PostType 强类型化
    pub slug: String,
    pub status: String,          // cms_domain::PostStatus
    pub visibility: String,      // cms_domain::Privacy（复用现有枚举！）
    pub passcode_hash: Option<String>,
    pub author_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub primary_asset_id: Option<i64>,
    pub featured_asset_id: Option<i64>,
    pub menu_order: i64,
    pub taken_at_label: String,
    pub taken_at_date: Option<Date>,
    pub published_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post_translations::Entity")]
    Translations,
    #[sea_orm(belongs_to = "super::media_assets::Entity",
              from = "Column::PrimaryAssetId", to = "super::media_assets::Column::Id")]
    PrimaryAsset,
    #[sea_orm(has_many = "super::post_terms::Entity")]
    PostTerms,
}
impl ActiveModelBehavior for ActiveModel {}
```

```rust
// crates/cms-domain/src/post.rs —— 强类型壳，复用现有 Privacy
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostType { Post, Page, Photo }

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus { Draft, Published, Scheduled, Private, Trash }

// Privacy 已存在于 cms-domain/src/photo.rs，visibility 直接复用：
// pub enum Privacy { Public, Locked, Private }
```

> 注册：在 `cms-entity/src/lib.rs` 增加 `pub mod posts; pub mod post_translations; pub mod terms; pub mod term_taxonomies; pub mod post_terms; pub mod post_revisions; pub mod post_meta;`，并在 `prelude.rs` 同步导出。

完整的 PostService / PostRepository 骨架见 [§3.4](#34-后端postservice-骨架) 与附录 A。

---

## §3 富文本文章编辑（TipTap）链路

你选了 **TipTap**（ProseMirror 内核）。下面把「存什么、怎么清洗、怎么渲染、怎么版本化」一次说清。

### 3.1 存储格式：JSON 是真源，HTML 是缓存

TipTap 编辑器产出 ProseMirror 文档 JSON。**双存**策略（业界标准）：

| 字段 | 内容 | 谁写 | 谁读 |
| --- | --- | --- | --- |
| `body_json` (JSONB) | TipTap 文档 JSON，**编辑的唯一真源** | 编辑器保存时提交 | 后台再次编辑时回填 |
| `body_html` (TEXT) | 服务端从 JSON 渲染并**清洗**后的 HTML | 后端保存时生成 | 前台直接 `v-html`/`dangerouslySetInnerHTML` 渲染 |

为什么不直接存 HTML？因为：① 前台渲染不该信任客户端 HTML（XSS）；② JSON 结构化便于后续做全文检索（抽纯文本喂 Meilisearch）、生成 TOC、做内容迁移。

> Spring 类比：`body_json` 像存「领域对象的序列化」，`body_html` 像一张「物化视图」——写时算好，读时零成本。

### 3.2 服务端清洗（关键安全点）：`ammonia`

**绝不能把前端来的 HTML 直接落库渲染**。新增 `ammonia` crate（基于 html5ever 的 allowlist 清洗器），在保存时把 TipTap JSON → HTML → 过一遍白名单。当前 `Cargo.toml` 没有 HTML 清洗依赖，这是必须补的（见 §8）。

```rust
// crates/cms-api/src/services/content_render.rs
use ammonia::Builder;
use std::collections::HashSet;

/// 把 TipTap JSON 渲染为安全 HTML。
/// 路线：JSON --(自写 walker / prosemirror-model 等价)--> 原始 HTML --(ammonia)--> 安全 HTML
pub fn render_body(body_json: &serde_json::Value) -> Result<RenderedBody, AppError> {
    let raw_html = tiptap_json_to_html(body_json)?;     // 见 3.3，纯函数、可单测
    let plain_text = strip_tags(&raw_html);

    // allowlist：只放行排版/媒体必要标签；img 只允许我们自己的 OSS 域
    let mut allowed_tags: HashSet<&str> = [
        "p","h1","h2","h3","h4","blockquote","pre","code","ul","ol","li",
        "strong","em","s","a","img","figure","figcaption","hr","br","table",
        "thead","tbody","tr","th","td","span",
    ].into_iter().collect();

    let clean_html = Builder::default()
        .tags(allowed_tags.drain().collect())
        .link_rel(Some("noopener nofollow"))
        .url_schemes(["http","https","mailto"].into_iter().collect())
        // data-* 给前端做锚点/代码高亮；class 仅放行白名单（防 CSS 注入式钓鱼）
        .add_generic_attributes(["data-id","data-lang"])
        .clean(&raw_html)
        .to_string();

    Ok(RenderedBody {
        html: clean_html,
        word_count: count_words(&plain_text),
        excerpt: make_excerpt(&plain_text, 160),
    })
}

pub struct RenderedBody { pub html: String, pub word_count: i32, pub excerpt: String }
```

### 3.3 TipTap JSON → HTML 的 walker

TipTap 文档是 `{ type:"doc", content:[ {type:"paragraph", content:[...]}, ... ] }`。写一个纯函数递归渲染（**纯逻辑、易单测**，对标你在 Java 里写的 `Jackson` 树遍历）：

```rust
// 摘要版；完整 node/mark 表见附录 A
fn tiptap_json_to_html(doc: &serde_json::Value) -> Result<String, AppError> {
    let mut out = String::new();
    if let Some(content) = doc.get("content").and_then(|c| c.as_array()) {
        for node in content { render_node(node, &mut out); }
    }
    Ok(out)
}

fn render_node(node: &serde_json::Value, out: &mut String) {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("paragraph") => wrap(node, "p", out),
        Some("heading")   => {
            let lvl = node.pointer("/attrs/level").and_then(|l| l.as_i64()).unwrap_or(2).clamp(1,4);
            wrap_tag(node, &format!("h{lvl}"), out);
        }
        Some("bulletList") => wrap(node, "ul", out),
        Some("orderedList") => wrap(node, "ol", out),
        Some("listItem")   => wrap(node, "li", out),
        Some("blockquote") => wrap(node, "blockquote", out),
        Some("codeBlock")  => render_code_block(node, out),
        Some("image")      => render_image(node, out),   // src 必须指向我们的 media_assets
        Some("text")       => render_text_with_marks(node, out),
        _ => {}
    }
}
```

> 工程取舍：也可以让**前端**用 TipTap 的 `generateHTML()` 出 HTML 一并提交，后端只跑 ammonia 清洗。优点是省一套 Rust walker；缺点是后端无法独立从 JSON 重建（如批量重渲染换主题时）。**推荐后端持有 walker**，前端提交 JSON 即可，渲染权威在后端。

### 3.4 后端 PostService 骨架

```rust
// crates/cms-api/src/services/post_service.rs（节选：创建/更新文章）
pub async fn upsert_article(state: &AppState, author_id: i64, req: ArticleReq)
    -> AppResult<PostDto>
{
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let txn = state.db.begin().await.map_err(db_err)?;

    // 1) 落 posts（status: draft/published/scheduled）
    let post_id = post_repo::upsert_post(&txn, &req).await.map_err(db_err)?;

    // 2) 每个 locale：清洗正文 + 写 translations + 存一份 revision
    for tr in &req.translations {
        let rendered = content_render::render_body(&tr.body_json)?;   // ammonia 清洗
        post_repo::upsert_translation(&txn, post_id, &tr.locale, tr, &rendered)
            .await.map_err(db_err)?;
        post_repo::insert_revision(&txn, post_id, &tr.locale, &tr.title, &tr.body_json, author_id)
            .await.map_err(db_err)?;
    }

    // 3) 分类法（category/tag/series）：写 post_terms
    post_repo::sync_terms(&txn, post_id, &req.term_ids).await.map_err(db_err)?;

    txn.commit().await.map_err(db_err)?;

    // 4) 缓存失效（见 §5 缓存矩阵）
    cache::invalidate_prefix(&state.redis, "cms:post:list:*").await;
    cache::del_one(&state.redis, &format!("cms:post:detail:{}", req.slug)).await;

    find_post_dto(state, post_id).await
}
```

```rust
// 定时发布（status='scheduled' 到点转 published）：复用现有 workers/ 模式，
// 起一个 tokio interval（≈ Spring @Scheduled），每分钟扫一次到点的 scheduled 文章。
// crates/cms-api/src/workers/scheduler.rs
pub async fn run(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        tick.tick().await;
        if let Err(e) = post_repo::publish_due(&state.db).await {
            tracing::warn!(error=?e, "scheduled publish failed");
        } else {
            cache::invalidate_prefix(&state.redis, "cms:post:list:*").await;
        }
    }
}
```

### 3.5 前端编辑器骨架（React + TipTap）

```tsx
// frontend/src/components/admin/ArticleEditor.tsx
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Image from "@tiptap/extension-image";
import Link from "@tiptap/extension-link";
import { uploadPhoto } from "@/hooks/useUploader";   // 复用现有 OSS 直传

export function ArticleEditor({ value, onChange }: {
  value: object | null; onChange: (json: object) => void;
}) {
  const editor = useEditor({
    extensions: [
      StarterKit,                                   // 段落/标题/列表/引用/代码块/粗斜体
      Link.configure({ openOnClick: false, HTMLAttributes: { rel: "noopener nofollow" } }),
      Image.configure({ inline: false }),
    ],
    content: value ?? { type: "doc", content: [] },
    onUpdate: ({ editor }) => onChange(editor.getJSON()),  // ★ 只回传 JSON，不回传 HTML
  });

  // 正文插图：走现有 presign→PUT→complete，拿回 OSS URL 后插入 image 节点
  async function onPickImage(file: File) {
    const { assetId, url } = await uploadPhoto(file);
    editor?.chain().focus().setImage({ src: url, "data-id": String(assetId) } as any).run();
  }

  return (
    <div className="article-editor">
      <Toolbar editor={editor} onPickImage={onPickImage} />
      <EditorContent editor={editor} />
    </div>
  );
}
```

保存时提交 `{ slug, status, visibility, translations:[{locale, title, excerpt?, body_json}], term_ids }`；后端清洗后回 `body_html`。前台文章页直接渲染 `body_html`，并用主题的排版令牌（§4）控制字体/行高/间距，做到「换主题=换文章观感」。

### 3.6 文章正文里的图片 ≠ 媒体卡片

注意区分两类图：

- **媒体卡片图**（现有 `photo` 流程）：一张图本身就是一条内容（`post_type='photo'`），有 privacy/variants/EXIF。
- **正文插图**（文章里的配图）：只是 `post_type='post'` 正文里的一个 image 节点，`src` 指向 `media_assets`（复用直传与变体，但不单独成卡）。

两者共用 `media_assets`/`media_variants` 底座，互不干扰。

---

## §4 可插拔主题引擎 + awesome-design-md 集成

你选了**完整可插拔主题包**。这一节是本提案的重头。先给整体架构，再拆 awesome-design-md 怎么接。

### 4.1 awesome-design-md 是什么、怎么用它

它是一组 `DESIGN.md` 文件（Google Stitch 提出的「设计系统纯文本文档」概念），每个文件用固定 9 节描述一个网站的设计系统：配色（§2，带十六进制）、字体层级（§3，表格）、组件样式（§4）、布局与间距/圆角（§5）、阴影（§6）、Do/Don't（§7）、响应式（§8）、给 Agent 的速查（§9，含「Quick Color Reference」代码块）。仓库现成 ~31 个（Notion、Sanity、Linear、Vercel、Stripe…）。

**集成思路**：把 `DESIGN.md` 当作**主题的「设计真源」**，从中抽取成机器可读的 **`tokens.json`（设计令牌契约）**，再由 `tokens.json` 生成 **CSS 变量**，单套前端渲染器只认 CSS 变量——于是「换主题」=「换一组令牌 + 少量布局开关」。

```
DESIGN.md  ──(令牌抽取器, 半自动+人工校对)──▶  tokens.json  ──(生成器)──▶  theme.css(:root{--gl-*})
   设计真源/给设计 Agent 看                       机器契约/给后端读              运行期/给浏览器用
```

> 为什么不直接让程序解析 DESIGN.md 的散文？因为 §2/§4 大量是自然语言描述，纯自动解析脆弱。**务实做法**：`tokens.json` 是契约，初版从 DESIGN.md 的 §9「Quick Color Reference」代码块 + §3/§5 表格**半自动抽取**（正则）后**人工校对一次**。DESIGN.md 继续作为设计依据保留在主题包里（也方便日后用 AI 重新生成 tokens）。

### 4.2 两层主题模型（务实分层落地「可插拔」）

| 层级 | 能力 | 切换方式 | 工程量 | 阶段 |
| --- | --- | --- | --- | --- |
| **Tier-1 令牌主题** | 换配色、字体、圆角、阴影、间距 + 少量布局开关（瀑布列数/密度/Hero 风格） | 纯运行期热切换，零部署 | 小 | M5（先做） |
| **Tier-2 模板主题** | 换版式结构（不同的 single/list 布局组件、自定义区块/Slot） | 编译期注册的 React 模板模块 + 模板清单 | 中大 | M6（后做） |

Tier-1 已经能覆盖 90% 的「换肤」诉求，且**完全热插拔、零风险**；Tier-2 解决「版式也要不同」的长尾。**先把 Tier-1 做扎实**，它直接消费 awesome-design-md。

> 为什么不做「主题内嵌任意 JS/HTML 模板」热加载？Rust/React 是编译期产物，运行期注入任意模板要么上 wasm 沙箱、要么上服务端模板引擎（Tera/Handlebars）放弃 React——都偏离当前 SPA 架构。Tier-2 用「编译期把模板组件注册进 registry + 主题清单声明用哪个」来实现可插拔，安全且类型安全。

### 4.3 主题包结构（文件系统，对标 `wp-content/themes/<slug>/`）

```
themes/
├── README.md                      # 包格式说明（本提案随附）
├── notion-editorial/              # 一个主题 = 一个目录
│   ├── theme.toml                 # 清单：名称/版本/作者/模式/支持的特性/模板映射
│   ├── DESIGN.md                  # 设计真源（来自 awesome-design-md，注明出处）
│   ├── tokens.json                # ★ 机器契约：颜色/字体/间距/圆角/阴影/布局
│   ├── theme.css                  # 由 tokens.json 生成（:root{--gl-*}），可手工微调
│   └── preview.png                # 主题缩略图（后台主题库卡片用）
└── sanity-noir/
    └── ...（同结构）
```

`theme.toml` 清单示例：

```toml
[theme]
slug = "notion-editorial"
name = "Notion Editorial"
version = "1.0.0"
author = "gathered-light"
mode = "light"                     # light | dark | auto
source = "awesome-design-md/notion"
license = "MIT"

[supports]
gallery = true                     # 是否支持图片瀑布流
article = true                     # 是否支持文章排版
dark_toggle = false

[templates]                        # Tier-2 用：每种路由用哪个模板组件（Tier-1 可省略）
home   = "default"
list   = "masonry"
single = "editorial"
photo  = "lightbox"

[layout]                           # Tier-1 布局开关
gallery_columns = 3
density = "comfortable"            # compact | comfortable | spacious
hero = "editorial"                 # editorial | minimal | cover
```

### 4.4 `tokens.json` 设计令牌契约

固定 schema，所有主题必须产出同一组**语义令牌**（不是原始色板，而是「角色」），这样前端组件只依赖角色、与具体主题解耦：

```jsonc
{
  "$schema": "../tokens.schema.json",
  "meta": { "name": "Notion Editorial", "source": "awesome-design-md/notion", "mode": "light" },
  "color": {
    "bg": "#ffffff",
    "bg-alt": "#f6f5f4",
    "surface": "#ffffff",
    "border": "rgba(0,0,0,0.1)",
    "text": "rgba(0,0,0,0.95)",
    "text-muted": "#615d59",
    "text-subtle": "#a39e98",
    "accent": "#0075de",
    "accent-hover": "#005bab",
    "on-accent": "#ffffff",
    "success": "#1aae39", "warning": "#dd5b00", "danger": "#dd0000"
  },
  "font": {
    "display": "\"Inter\", ui-sans-serif, system-ui, sans-serif",
    "body": "\"Inter\", ui-sans-serif, system-ui, sans-serif",
    "mono": "\"IBM Plex Mono\", ui-monospace, monospace"
  },
  "type": {
    "display-size": "64px", "display-weight": 700, "display-tracking": "-2.125px", "display-line": 1.0,
    "h2-size": "26px", "h2-weight": 700, "h2-tracking": "-0.625px",
    "body-size": "16px", "body-line": 1.5
  },
  "radius": { "input": "4px", "card": "12px", "pill": "9999px" },
  "space":  { "unit": "8px", "section-y": "80px", "gutter": "32px", "max-width": "1200px" },
  "shadow": {
    "card": "rgba(0,0,0,0.04) 0 4px 18px, rgba(0,0,0,0.02) 0 0.8px 2.9px",
    "deep": "rgba(0,0,0,0.05) 0 23px 52px, rgba(0,0,0,0.04) 0 14px 28px"
  }
}
```

> 上面的具体值就是从 awesome-design-md 的 Notion `DESIGN.md`（§9 Quick Color Reference + §3 字体表 + §5 间距/圆角）抽出来的真实令牌。随附的 `themes/notion-editorial/tokens.json` 和 `themes/sanity-noir/tokens.json` 即两份成品。

### 4.5 后端：主题 registry + 激活 + 定制器（复用 site_settings！）

不必为主题新建一堆表。**文件系统扫描 + 复用现有 `site_settings`**：

- 启动时扫描 `themes/` 目录，把每个 `theme.toml` + `tokens.json` 读入内存 `ThemeRegistry`（≈ Spring 启动时扫 classpath 下的 starter）。
- 当前激活主题：`site_settings` 存 `active_theme = "notion-editorial"`（替代旧的 `theme=warm/cool/bold`）。
- 用户定制（Customizer，覆盖个别令牌）：`site_settings` 存 `theme_mods:notion-editorial = {...json}` —— 正是 WP 的 `theme_mods_<theme>` 翻版。
- 最终下发 = `tokens.json` 深合并 `theme_mods` 覆盖项。

```rust
// crates/cms-api/src/services/theme_service.rs（骨架）
#[derive(Clone, Serialize)]
pub struct ActiveTheme {
    pub slug: String,
    pub manifest: ThemeManifest,    // 来自 theme.toml
    pub tokens: serde_json::Value,  // tokens.json ⊕ theme_mods 合并后
}

const ACTIVE_THEME_KEY: &str = "active_theme";
const CACHE_KEY: &str = "cms:theme:active";
const DEFAULT_THEME: &str = "notion-editorial";

pub async fn get_active(state: &AppState) -> AppResult<ActiveTheme> {
    if let Some(t) = cache::get(&state.redis, CACHE_KEY).await { return Ok(t); }

    let slug = settings_repo::get(&state.db, ACTIVE_THEME_KEY).await.map_err(db_err)?
        .filter(|s| state.themes.contains(s))         // 校验主题存在（防脏数据）
        .unwrap_or_else(|| DEFAULT_THEME.to_owned());

    let base = state.themes.tokens(&slug).ok_or(AppError::NotFound)?;     // 内存 registry
    let mods = settings_repo::get(&state.db, &format!("theme_mods:{slug}")).await.map_err(db_err)?
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(serde_json::json!({}));
    let tokens = deep_merge(base, mods);             // mods 覆盖 base

    let theme = ActiveTheme { slug: slug.clone(), manifest: state.themes.manifest(&slug).unwrap(), tokens };
    cache::set(&state.redis, CACHE_KEY, &theme, 300).await;
    Ok(theme)
}

pub async fn activate(state: &AppState, slug: &str) -> AppResult<ActiveTheme> {
    if !state.themes.contains(slug) { return Err(AppError::BadRequest("unknown theme")); }
    settings_repo::set(&state.db, ACTIVE_THEME_KEY, slug).await.map_err(db_err)?;
    cache::del_one(&state.redis, CACHE_KEY).await;       // 失效后下次 get 重建
    get_active(state).await
}
```

`ThemeRegistry` 在 `bootstrap.rs::AppState` 里加一个字段 `pub themes: Arc<ThemeRegistry>`，启动时 `ThemeRegistry::scan("themes/")` 装配（与现有 `db/redis/s3` 同级）。

### 4.6 主题相关 API

| Method | Path | 说明 | 鉴权 |
| --- | --- | --- | --- |
| GET | `/api/v1/theme` | 取当前激活主题的 `tokens`(已合并 mods) + `manifest` | 公开（前台启动拉一次） |
| GET | `/api/v1/admin/themes` | 列出 `themes/` 下所有已安装主题（卡片库） | 需登录 |
| POST | `/api/v1/admin/themes/activate` | `{slug}` 切换激活主题 → 失效 `cms:theme:active` | owner/editor |
| PATCH | `/api/v1/admin/themes/:slug/customize` | `{tokens 覆盖项}` 存 `theme_mods:<slug>` | owner/editor |

> 替代关系：现有 `PATCH /admin/settings/theme`（warm/cool/bold）由 `/admin/themes/activate` 取代；旧值在迁移时映射到一个内置主题（如 warm→notion-editorial、bold→sanity-noir）。`/api/v1/settings` 里的 `theme` 字段保留一版兼容期，前端切到 `/api/v1/theme` 后再下线。

### 4.7 DESIGN.md → tokens.json 抽取器（半自动脚手架）

提供一个导入脚本：给定 awesome-design-md 的某个 `DESIGN.md`，scaffold 出主题包骨架。建议放 `scripts/import_design_md.mjs`（Node，正则抽 §9 颜色块 + §3 字体表）或 Rust bin。产物需人工校对一次：

```
node scripts/import_design_md.mjs \
  --url https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/design-md/linear.app/DESIGN.md \
  --slug linear-minimal
# 生成 themes/linear-minimal/{theme.toml, DESIGN.md, tokens.json(草稿待校对)}
```

抽取规则（务实、够用）：
- **颜色**：优先解析 §9 `Quick Color Reference` 代码块里的 `name: #hex`；映射到语义角色（background→`color.bg`，accent/CTA→`color.accent`，text→`color.text` …）。
- **字体**：§3 字体表第一行 Display / Body 的 font-family、size、weight、letter-spacing。
- **圆角/间距/阴影**：§5/§6 表格的 token 值。
- 抽不到的角色给安全默认值，并在 tokens.json 里标 `"_todo": [...]` 提示人工补。

---

## §5 API / 缓存 / 安全增量

### 5.1 新增路由（挂进现有 `routes.rs`）

```rust
// 公开（前台）
.route("/theme", get(theme_handler::get_active))                 // 主题令牌
.route("/posts", get(post_handler::list_public))                 // 文章列表
.route("/posts/:slug", get(post_handler::get_public))            // 文章详情（body_html）
.route("/pages/:slug", get(post_handler::get_page))              // 独立页面

// 管理（protected_api，已有 jwt_guard）
.route("/admin/posts", get(post_handler::list_admin).post(post_handler::create))
.route("/admin/posts/:id", put(post_handler::update).delete(post_handler::delete))
.route("/admin/posts/:id/revisions", get(post_handler::list_revisions))
.route("/admin/posts/:id/restore", post(post_handler::restore_revision))
.route("/admin/themes", get(theme_handler::list_installed))
.route("/admin/themes/activate", post(theme_handler::activate))
.route("/admin/themes/:slug/customize", patch(theme_handler::customize))
.route("/admin/terms", get(term_handler::list).post(term_handler::create))   // 统一分类法
.route("/admin/terms/:id", patch(term_handler::update).delete(term_handler::delete))
```

返回包沿用现有 `{ code, message, data }` 风格与 `AppError`→`IntoResponse` 全局错误模型（`error.rs`，无需改）。

### 5.2 Redis Key 扩展（沿用现有 `cms:` 命名规范）

| Key | TTL | 失效时机 |
| --- | --- | --- |
| `cms:theme:active` | 300s | 切换主题 / 改 theme_mods 时 `DEL` |
| `cms:post:list:type={t}:cat={c}:p={n}` | 60s | 任意 post 写操作 → `SCAN+UNLINK cms:post:list:*` |
| `cms:post:detail:{slug}:lang={l}` | 300s | 该文章更新/删除时 `DEL` |
| `cms:taxonomy:terms:{taxonomy}:lang={l}` | 1h | 分类法写操作时 `DEL` |
| `cms:photo:list:cat={cat}` | 60s | **保留现状**（迁移后内部改查 posts，key 不变，前端无感） |

失效函数复用现有 `infra/cache.rs::invalidate_prefix`（SCAN+UNLINK，已防阻塞）。

### 5.3 安全要点（在现有清单上补 3 条）

1. **正文 XSS**：TipTap HTML 必须服务端 `ammonia` 白名单清洗（§3.2）。前端 `dangerouslySetInnerHTML` 只渲染 `body_html`，永不渲染未清洗内容。
2. **正文外链**：`<a>` 强制 `rel="noopener nofollow"`，`<img src>` 仅放行自家 OSS/CDN 域（在 ammonia `url_schemes` + 自定义 filter 里校验 host）。配合现有 CSP（§8 ARCHITECTURE）只允许自家域。
3. **主题令牌注入**：`tokens.json` 的值会进 CSS 变量。导入/定制时校验颜色为合法 hex/rgb、尺寸为 `\d+(px|rem|%)`、字体名白名单化，**禁止把任意字符串塞进 CSS**（防 `url()` 外联、CSS 注入式钓鱼）。复用现有 `validate_hex_color` 思路。
4. 权限：文章/主题写操作走现有 owner/editor/viewer——viewer 只读，editor 可写内容，仅 owner 可切主题（按需）。

---

## §6 前端架构改造

现状前端是单体 `PublicGallery` + `AdminApp`。改造点：

### 6.1 主题运行时（Tier-1 核心）

启动时拉 `/api/v1/theme`，把 tokens 注入 `:root` 的 CSS 变量，全站组件只用 `var(--gl-*)`：

```tsx
// frontend/src/theme/applyTheme.ts
export function applyTheme(tokens: ThemeTokens) {
  const root = document.documentElement;
  const flat = flatten(tokens);                       // {"color.bg":"#fff"} → "--gl-color-bg"
  for (const [k, v] of Object.entries(flat)) {
    root.style.setProperty(`--gl-${k.replace(/\./g, "-")}`, String(v));
  }
  root.dataset.themeMode = tokens.meta?.mode ?? "light";
}
```

```css
/* styles.css 改造：把写死的颜色换成变量，主题切换即时生效 */
:root { --gl-color-bg:#fff; --gl-color-text:rgba(0,0,0,.95); /* 默认，等接口覆盖 */ }
body { background: var(--gl-color-bg); color: var(--gl-color-text); font-family: var(--gl-font-body); }
.article-body { font-family: var(--gl-font-body); line-height: var(--gl-type-body-line); }
.article-body h2 { font-family: var(--gl-font-display); letter-spacing: var(--gl-type-h2-tracking); }
```

> 现有 `theme=warm/cool/bold` + `brand_effect`/`hero` 这些 Tweaks 面板逻辑，平滑升级为「主题定制器」：定制器改的就是 `theme_mods`，调用 `PATCH /admin/themes/:slug/customize`。原 Tweaks 的「品牌字光效/Hero 文案」作为主题的可选扩展令牌保留。

### 6.2 目录建议（在现有 `frontend/src` 上增量）

```
frontend/src/
├── theme/
│   ├── applyTheme.ts          # CSS 变量注入
│   ├── useTheme.ts            # 启动拉取 + 缓存（TanStack Query / 现有 fetch 封装）
│   └── ThemeProvider.tsx
├── components/
│   ├── admin/
│   │   ├── ArticleEditor.tsx  # §3.5 TipTap
│   │   ├── ArticleList.tsx
│   │   ├── ThemeGallery.tsx   # 主题库卡片 + 激活
│   │   └── ThemeCustomizer.tsx# 令牌微调 → theme_mods
│   ├── site/
│   │   ├── ArticleView.tsx    # 渲染 body_html + 主题排版
│   │   └── PublicGallery.tsx  # 现有，改用 CSS 变量
└── api/
    ├── posts.ts               # 新增
    └── theme.ts               # 新增
```

API 客户端沿用现有 `api/client.ts`（已有 JWT 注入 + 401 无感刷新，不动）。

---

## §7 实施路线图（M2 → M6）

延续 ARCHITECTURE.md 的 M 系列与 PR-by-PR + DoD 风格：

| 阶段 | 目标 | 关键产物 | DoD（验收） |
| --- | --- | --- | --- |
| **M2 内容模型地基** | posts/post_translations/统一分类法建表 + photos 数据迁入 | 新迁移 `m20260701_000004_content_model`；entity；photos→posts DML | 迁移 up/down 可逆；`/api/v1/photos` 行为与回归测试**完全不变**（内部已改查 posts） |
| **M3 文章编辑** | TipTap 编辑 + ammonia 清洗 + 渲染 + 版本 | `content_render.rs`(+walker 单测)；`post_service`；`ArticleEditor.tsx`；`/admin/posts` CRUD | 建一篇含图/标题/列表/代码的文章，前台渲染正确且无 XSS（注入 `<script>` 被清除）；revision 可回滚 |
| **M4 页面与分类法** | page 类型 + 层级分类 + （可选）菜单 | `/pages/:slug`；term CRUD；分类树缓存 | 建「关于」页可访问；分类可层级、可同时挂文章与照片 |
| **M5 主题引擎 Tier-1** | 令牌主题 + DESIGN.md 导入 + 定制器 | `ThemeRegistry`；`theme_service`；`/theme` API；`applyTheme.ts`；导入脚本；≥3 个内置主题 | 后台一键切换主题，前台配色/字体即时变；定制器改令牌持久化；旧 warm/cool/bold 平滑映射 |
| **M6 主题 Tier-2 + 长尾** | 模板主题（版式可换）+ 评论 + RSS + Meili 全文检索 | 模板 registry；`comments`；`/feed.xml`；Meili 索引 body 纯文本 | 切换两个不同版式主题；文章可评论；RSS 可订阅；标题/正文毫秒级搜索 |

每个 PR 沿用 ARCHITECTURE §0.5 的 DoD checklist 模板（命中的硬约束、是否破坏前端契约、是否新增 tracing span 等）。

---

## §8 新增依赖清单

```toml
# 工作区 Cargo.toml [workspace.dependencies] 增加：
ammonia      = "4"          # ★ HTML 清洗（XSS 白名单）—— 必须，当前无
slug         = "0.1"        # slug 规范化（替代手写 validate_slug，可选）
pulldown-cmark = "0.12"     # 若也想支持 Markdown 输入（可选）
toml         = "0.8"        # 读 theme.toml（serde 已在）

# 已具备、直接复用：serde_json(JSONB)、validator(令牌校验)、sea-orm、fred、image、tracing
```

前端 `package.json` 增加：

```jsonc
"@tiptap/react": "^2",
"@tiptap/starter-kit": "^2",
"@tiptap/extension-image": "^2",
"@tiptap/extension-link": "^2"
// 代码高亮可选：@tiptap/extension-code-block-lowlight + lowlight
```

---

## 附录 A：核心代码骨架索引

随本提案落地的**真实文件**（已创建，可直接用）：

- `themes/README.md` —— 主题包格式说明书
- `themes/notion-editorial/theme.toml` + `tokens.json` —— 暖色编辑风主题（源自 awesome-design-md/notion），适配「拾光集」气质
- `themes/sanity-noir/theme.toml` + `tokens.json` —— 暗色精密风主题（源自 awesome-design-md/sanity）

**仍需在后续 PR 落地的骨架**（本文 §2-§6 已给出代码片段，需补全为可编译）：

| 文件 | 角色（Spring 类比） | 本文位置 |
| --- | --- | --- |
| `migrations/src/sql/m20260701_000004_content_model_up.sql` | Flyway 迁移 | §2.3 / §2.4 |
| `cms-entity/src/{posts,post_translations,terms,term_taxonomies,post_terms,post_revisions,post_meta}.rs` | Entity | §2.5 |
| `cms-domain/src/post.rs`（PostType/PostStatus，复用 Privacy） | 领域枚举 | §2.5 |
| `cms-api/src/services/content_render.rs`（+ tiptap walker 单测） | 正文渲染/清洗 | §3.2 / §3.3 |
| `cms-api/src/services/post_service.rs` | @Service | §3.4 |
| `cms-api/src/services/theme_service.rs` + `infra/theme_registry.rs` | @Service + starter 扫描 | §4.5 |
| `cms-api/src/handlers/{post_handler,theme_handler,term_handler}.rs` | @RestController | §5.1 |
| `cms-api/src/workers/scheduler.rs`（定时发布） | @Scheduled | §3.4 |
| `frontend/src/theme/applyTheme.ts`、`components/admin/ArticleEditor.tsx` | 前端运行时/编辑器 | §3.5 / §6.1 |

> 衔接确认：本文所有表名/字段/路由/缓存 key/迁移写法均对齐现有仓库（`photos`/`site_settings`/`routes.rs`/`infra/cache.rs`/`migrations` 模式）。唯一会触及现有前端契约的是 §2 内容模型迁移——已通过「PhotoService 内部改造 + `/photos` DTO 不变」的兼容层规避，前端可零改动平滑过渡。

