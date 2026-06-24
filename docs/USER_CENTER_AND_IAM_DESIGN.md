# 拾光集 · 用户中心 / 身份与权限（IAM）设计

> 配套文档：内容模型与主题见 `docs/CMS_REFACTOR_PROPOSAL.md`；本文专注 **用户中心（User Center）+ 身份认证（AuthN）+ 授权（AuthZ）+ 多租户隔离**。
>
> 范围由四个已确认决策框定：
> 1. **SSO = RP + IdP 双向**：既支持第三方登录入站（Google/GitHub/微信/钉钉…），本站也作为 OAuth2/OIDC Provider 对外签发令牌、供子系统单点登录。
> 2. **多租户硬隔离**：支持「组织 / 站点」两级，数据按租户硬隔离（SaaS 形态）。
> 3. **鉴权增强全开**：TOTP 2FA + 刷新令牌轮换/会话管理 + 审计日志 + 密码策略/登录风控。
> 4. 内容模型走**统一 Post 模型**（已定）。
>
> 读者：资深 Java/Spring Cloud 后端。全程映射到 **Spring Security / Spring Authorization Server / Sa-Token / Casbin / 多租户（ShardingSphere、MyBatis 拦截器）** 等你熟悉的概念。

---

## 目录

- [§0 决策摘要与现状基线](#0-决策摘要与现状基线)
- [§1 架构：身份作为限界上下文](#1-架构身份作为限界上下文)
- [§2 多租户隔离模型](#2-多租户隔离模型)
- [§3 用户与身份模型](#3-用户与身份模型)
- [§4 权限模型（RBAC + 资源归属 + 可选 ABAC）](#4-权限模型rbac--资源归属--可选-abac)
- [§5 鉴权与令牌体系重构](#5-鉴权与令牌体系重构)
- [§6 SSO 作为 RP：第三方登录入站](#6-sso-作为-rp第三方登录入站)
- [§7 SSO 作为 IdP：统一认证中心](#7-sso-作为-idp统一认证中心)
- [§8 多因子认证（TOTP 2FA）](#8-多因子认证totp-2fa)
- [§9 密码策略与登录风控](#9-密码策略与登录风控)
- [§10 审计日志](#10-审计日志)
- [§11 API 总表](#11-api-总表)
- [§12 数据迁移与兼容](#12-数据迁移与兼容)
- [§13 缓存与安全要点](#13-缓存与安全要点)
- [§14 新增依赖](#14-新增依赖)
- [§15 落地路线图](#15-落地路线图)
- [附录 A：crate / 模块结构](#附录-acrate--模块结构)

---

## §0 决策摘要与现状基线

### 0.1 现状基线（必须在此之上演进，不推翻）

| 现状 | 文件 | 评估 |
| --- | --- | --- |
| JWT **HS256**，`Claims{ sub, email, role, typ, iat, exp, jti }` | `infra/jwt.rs` | ⚠️ 对称密钥，**IdP 场景必须改非对称**（见 §5） |
| access + refresh 均为 JWT；refresh 存 Redis `auth:refresh:{jti}=user_id` | `auth_service.rs` | ✅ 已有「刷新即换新 + 删旧」雏形，升级为「家族轮换 + 重放检测」 |
| 黑名单 `auth:blacklist:{jti}`、全踢 `auth:user-rev:{uid}` epoch | `auth_service.rs` | ✅ 保留，纳入新会话体系 |
| `users(id,email,password_hash,display_name,role,ts)`，role ∈ owner/editor/viewer（硬编码校验） | `cms-entity/users.rs`、`user_service.rs::validate_role` | 🔨 升级为身份/凭证/成员分离 + RBAC |
| Argon2 **默认参数** | `auth_service.rs`、`user_service.rs` | 🔨 显式调参（§9） |
| `AppState{config,db,redis,s3,jwt:Arc<JwtKeys>}` | `bootstrap.rs` | 🔨 增 `keyring`(JWKS)、`tenants`(可选缓存)、`oidc_clients` |
| 登录失败计数（设计有，§ARCHITECTURE） | — | 🔨 落地到风控（§9） |

> 好消息：现有 token 设计**已经有 refresh-in-Redis、单 token 黑名单、全设备 epoch**三件套，不是从零开始。我们是在它上面做「非对称签名 + DB 会话注册表 + 多租户 claims」。

### 0.2 决策的连锁影响（先有心理准备）

- **IdP ⇒ 非对称签名**：对外当 OAuth2 Provider，RP 要能用公钥验签而不持有私钥 ⇒ HS256 必须升级 RS256 / EdDSA + JWKS。这是本次最大的底层改动。
- **多租户 ⇒ 所有内容表加 `tenant_id`/`site_id` + 行级安全（RLS）**：包括 §CMS_REFACTOR_PROPOSAL 里新建的 `posts`/`terms`/`media_assets`/`site_settings`/`themes-active` 等，建表时就带上租户维度（见 §2.4）。
- **全局用户 vs 租户成员**：一个 user 可属于多个 tenant（IdP/SSO 必然），所以 `users` 是**全局身份**，`memberships` 才挂租户与角色。现有「user.role」字段下沉为「membership.role」。

---

## §1 架构：身份作为限界上下文

把 **Identity（身份）** 当作一个独立的限界上下文（Bounded Context），与 CMS 内容域解耦。这正是「用户中心」该有的样子。

```
                         ┌─────────────────────────────────────────┐
   浏览器 / 子系统  ───▶ │  cms-api (Axum 单体)                      │
                         │                                          │
   /api/v1/auth/*        │  ┌── identity 模块组（未来可拆服务）──┐  │
   /oauth2/* (IdP)       │  │  authn_service   (登录/令牌/2FA)    │  │
   /.well-known/*        │  │  authz_service   (RBAC 判定)        │  │
                         │  │  tenant_service  (租户/站点/成员)   │  │
   /api/v1/admin/users   │  │  oidc_provider   (IdP 端点)         │  │
   /api/v1/me/*          │  │  oidc_client     (RP 第三方登录)    │  │
                         │  │  audit_service                      │  │
                         │  └──────────────┬──────────────────────┘  │
                         │   content 模块（posts/terms/media/theme） │
                         │                 │  共享 AppState(db/redis) │
                         └─────────────────┼─────────────────────────┘
                                           ▼
                          PostgreSQL（RLS 强隔离） + Redis（会话/限流/JWKS缓存）
```

**架构取舍：现在「模块化单体」，预留「拆服务」的缝**

- 现在：在 workspace 新增 `crates/cms-identity`（纯领域：User/Tenant/Role/Permission/Scope 等类型 + trait），handler/service 放进 `cms-api/src/identity/` 模块组。和现有 `cms-domain`/`cms-entity` 同构，编译期清晰隔离。
- 未来：当用户中心要独立扩容 / 给多个产品共用时，把 identity 模块连同 `users/tenants/...` 表整体切出去成独立 `identity-service`，cms-api 退化为它的 RP（通过 §7 的 IdP 协议接入自己）。**届时协议已经是标准 OIDC，无需重写。**

> Spring 类比：等价于「现在 `auth` 是同一个 Spring Boot 应用里的一个 module，但接口契约用标准 OAuth2；将来直接把它换成独立部署的 **Spring Authorization Server**，业务服务作为 Resource Server / OAuth2 Client 接入，零协议改动」。

---

## §2 多租户隔离模型

### 2.1 三种隔离策略对比

| 维度 | A. 共享库共享表 + `tenant_id` + RLS | B. 一租户一 Schema | C. 一租户一库 |
| --- | --- | --- | --- |
| 隔离强度 | 中（DB 强制 RLS，应用兜底） | 强 | 最强 |
| 成本/运维 | 低（一套库） | 中（schema 暴涨） | 高（库暴涨、迁移地狱） |
| 跨租户聚合（运营后台/计费） | 易 | 难 | 最难 |
| 连接池/资源 | 共享、高效 | 共享 | 每库一套，浪费 |
| 适合规模 | 中小→中大（数千租户内） | 强合规、少量大客户 | 银行级/强物理隔离 |
| 迁移改造量 | 中（表加列 + RLS 策略） | 大 | 很大 |

### 2.2 推荐：A（共享表 + `tenant_id` + Postgres RLS）

理由：CMS/SaaS 的典型选择，单库即可、便于运营聚合，且 **Postgres 原生 Row-Level Security 提供数据库层的硬隔离**——即使应用层漏判 `WHERE tenant_id=?`，DB 也会拒绝越权行。这是「应用 + 数据库双保险」，比纯靠 ORM 加条件（≈ MyBatis 多租户拦截器）更可靠。

> 留好升级路径：对**强合规大客户**可单独迁到 B（schema-per-tenant）。`tenant_id` 设计在前，将来按租户分 schema/分库都不破坏模型。

### 2.3 租户 / 站点 / 成员 三层模型

```
Tenant（组织/账户，计费与归属边界）
  │  1───N
  ├── Site（站点：独立域名 + 主题 + 内容空间；一个组织可开多个站）
  │     │  1───N
  │     └── Post / Term / Media / Settings ...   ← 内容按 site_id 归属（含 tenant_id 冗余便于 RLS）
  │
  └── Membership（user ↔ tenant，携带组织级角色）
        └── SiteGrant（可选：user 在某 site 的更细角色，缺省继承组织角色）
```

- **Tenant**：组织/账户。计费、配额、SSO 配置（自带 IdP 客户端、第三方登录开关）挂这里。
- **Site**：一个可访问的站点（域名、激活主题、`site_settings`）。内容隶属 site。对标 WordPress Multisite 的「network → site」。
- **User**：全局身份（§3），可加入多个 Tenant。
- **Membership**：user 在某 tenant 的成员关系 + 组织级角色（owner/admin/editor/author/viewer）。
- **SiteGrant**（可选精细化）：user 在某具体 site 的角色覆盖（如「某人只是 B 站的 editor」）。

### 2.4 内容表的租户化改造（与 CMS_REFACTOR_PROPOSAL 衔接）

§CMS_REFACTOR_PROPOSAL 中新建/现有的内容表统一加两列（`tenant_id` 冗余是为 RLS 与查询便利）：

```sql
ALTER TABLE posts          ADD COLUMN tenant_id BIGINT NOT NULL, ADD COLUMN site_id BIGINT NOT NULL;
ALTER TABLE post_translations ADD COLUMN tenant_id BIGINT NOT NULL;  -- 冗余，便于 RLS
ALTER TABLE terms          ADD COLUMN tenant_id BIGINT NOT NULL, ADD COLUMN site_id BIGINT NOT NULL;
ALTER TABLE media_assets   ADD COLUMN tenant_id BIGINT NOT NULL, ADD COLUMN site_id BIGINT NOT NULL;
ALTER TABLE site_settings  ADD COLUMN site_id BIGINT NOT NULL;       -- settings 从「全局」升级为「按站点」
-- 主题激活：原 site_settings 里的 active_theme/theme_mods 自动随 site_id 隔离
```

> 注意：原 `site_settings` 是全局 key/value，多租户后**主键从 `key` 变为 `(site_id, key)`**。这会动到 §CMS_REFACTOR_PROPOSAL §4.5 的主题激活实现（`active_theme` 改为按 site 查），但接口形状不变。

### 2.5 租户/站点解析（请求进来怎么知道是谁的）

按优先级链解析当前 site/tenant（≈ Spring 的 `TenantContext` + 拦截器）：

1. **公开前台**：按 **Host 头/域名**匹配 `site_domains` → site → tenant（自定义域名或 `{slug}.gathered-light.app` 子域）。
2. **管理端/API**：以 **JWT claim `tid`/`sid`** 为准（登录时选定的活跃租户/站点，见 §5.2）。
3. 显式 `X-Tenant-Id` / `X-Site-Id` 头（内部服务调用、运营后台跨租户操作，需特权 scope）。

解析结果写入请求扩展 `TenantContext{ tenant_id, site_id }`，并在同一中间件里**给本请求的 DB 连接设置 RLS 变量**。

### 2.6 Postgres RLS 落地（数据库层硬隔离）

```sql
-- 对每张含 tenant_id 的表开启 RLS，并加策略：只能看/写当前会话变量指定的租户
ALTER TABLE posts ENABLE ROW LEVEL SECURITY;
ALTER TABLE posts FORCE ROW LEVEL SECURITY;          -- 连表 owner 也受约束
CREATE POLICY tenant_isolation ON posts
    USING      (tenant_id = current_setting('app.tenant_id')::bigint)
    WITH CHECK (tenant_id = current_setting('app.tenant_id')::bigint);
-- terms / media_assets / post_translations ... 同理
```

应用侧：每个请求在事务开始时注入会话变量（用普通业务角色连接，**不要用 superuser/表 owner 绕过 RLS**）：

```rust
// identity/tenant_ctx.rs —— 取连接后先 SET LOCAL，作用域限本事务
// ≈ MyBatis 多租户拦截器自动拼 tenant 条件，但这里是 DB 层强制，漏判也越不了权
pub async fn bind_tenant(txn: &DatabaseTransaction, ctx: &TenantContext) -> Result<(), DbErr> {
    txn.execute_unprepared(&format!(
        "SET LOCAL app.tenant_id = '{}'; SET LOCAL app.site_id = '{}';",
        ctx.tenant_id, ctx.site_id
    )).await?;
    Ok(())
}
```

> 取舍：RLS 要求「每请求一事务 + SET LOCAL」。SeaORM 下统一用 `db.begin()` 包裹业务（你现有 service 已大量用事务），把 `bind_tenant` 放进事务开头的统一封装即可。对极热的公开只读列表，可用「应用层带 `tenant_id` 条件 + 跳过事务」走快路径，按需取舍。

---

## §3 用户与身份模型

核心原则：**把「身份 / 凭证 / 联合登录 / 资料 / 多因子」拆开**，而不是现在这样全堆在一张 `users` 表。这样才扛得住「一个人，多种登录方式，多个租户」。

### 3.1 拆分总览

| 表 | 职责 | 对标 |
| --- | --- | --- |
| `users` | **全局身份**：唯一标识一个人（主邮箱、状态、是否启用 2FA）。**不含租户、不含角色** | wp_users 的「人」部分 / Keycloak User |
| `user_credentials` | 本地密码凭证（argon2 hash、改密时间、是否需重置） | 把密码从身份里拆出来 |
| `user_identities` | 联合登录身份：`(provider, subject)` 唯一，一人可绑多个（Google/GitHub/微信…） | OIDC `sub` / Spring `OAuth2User` |
| `user_profiles` | 展示资料：昵称、头像、locale、时区、bio | wp_usermeta 的资料部分 |
| `user_mfa` | TOTP 密钥 + 恢复码（§8） | 2FA 凭证 |
| `tenants` / `sites` / `site_domains` | 租户与站点（§2） | network/site |
| `memberships` | user ↔ tenant + 组织角色 | 成员关系 |
| `site_grants` | user ↔ site 的角色覆盖（可选） | 细粒度 |
| `roles` / `permissions` / `role_permissions` | RBAC（§4） | Spring authorities |
| `sessions` / `refresh_tokens` | 会话与刷新令牌（§5） | 会话注册表 |
| `audit_logs` | 审计（§10） | 操作留痕 |
| `oauth_clients` / `oauth_authorization_codes` / `oauth_grants` | IdP（§7） | Spring Authorization Server 注册表 |

### 3.2 身份相关 DDL（迁移 `m20260710_000005_identity`）

```sql
-- 全局身份（注意：无 role、无 tenant_id —— 一个人可跨租户）
CREATE TABLE users (
    id            BIGSERIAL PRIMARY KEY,
    email         CITEXT UNIQUE NOT NULL,           -- 主邮箱（登录标识之一）
    email_verified_at TIMESTAMPTZ,
    status        TEXT NOT NULL DEFAULT 'active'
                  CHECK (status IN ('active','disabled','locked')),
    mfa_enabled   BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 本地密码凭证（与身份分离：将来「无密码登录」用户此表可无记录）
CREATE TABLE user_credentials (
    user_id        BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash  TEXT NOT NULL,                   -- argon2id（§9 显式调参）
    must_reset     BOOLEAN NOT NULL DEFAULT false,
    password_changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 联合登录身份（一人多绑）
CREATE TABLE user_identities (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider      TEXT NOT NULL,                    -- 'google'|'github'|'wechat'|'dingtalk'|'oidc:<issuer>'
    subject       TEXT NOT NULL,                    -- 该 provider 下的唯一 id（OIDC sub）
    email_at_provider TEXT,
    raw_profile   JSONB,                            -- 原始 userinfo 快照
    linked_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, subject)
);

CREATE TABLE user_profiles (
    user_id      BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    display_name TEXT,
    avatar_url   TEXT,
    locale       TEXT NOT NULL DEFAULT 'zh',
    timezone     TEXT NOT NULL DEFAULT 'Asia/Shanghai',
    bio          TEXT
);

-- 租户 / 站点
CREATE TABLE tenants (
    id          BIGSERIAL PRIMARY KEY,
    slug        TEXT UNIQUE NOT NULL,
    name        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','suspended')),
    plan        TEXT NOT NULL DEFAULT 'free',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE sites (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    slug        TEXT NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, slug)
);
CREATE TABLE site_domains (
    id          BIGSERIAL PRIMARY KEY,
    site_id     BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    domain      TEXT UNIQUE NOT NULL,               -- 自定义域名 / 子域，用于前台 Host 解析
    is_primary  BOOLEAN NOT NULL DEFAULT false
);

-- 成员关系（user 在某 tenant 的组织级角色）
CREATE TABLE memberships (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id   BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    role_id     BIGINT NOT NULL REFERENCES roles(id),
    status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','invited','suspended')),
    invited_by  BIGINT REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, tenant_id)
);

-- 站点级角色覆盖（可选）
CREATE TABLE site_grants (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    site_id     BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    role_id     BIGINT NOT NULL REFERENCES roles(id),
    PRIMARY KEY (user_id, site_id)
);
```

### 3.3 Entity 骨架（节选）

```rust
// crates/cms-entity/src/users.rs（重构后；旧的 password_hash/role 字段移除）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)] pub id: i64,
    #[sea_orm(column_type = "custom(\"citext\")", unique)] pub email: String,
    pub email_verified_at: Option<DateTimeWithTimeZone>,
    pub status: String,
    pub mfa_enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
// 关系：has_one credentials / has_many identities / has_many memberships ...
```

```rust
// crates/cms-identity/src/lib.rs —— 纯领域类型（无 SeaORM 依赖，可复用/可单测）
pub struct UserId(pub i64);
pub struct TenantId(pub i64);
pub struct SiteId(pub i64);

/// 登录后解析出的「主体」，贯穿请求（≈ Spring SecurityContext 的 Authentication）
#[derive(Clone, Debug)]
pub struct Principal {
    pub user_id: i64,
    pub tenant_id: i64,           // 活跃租户
    pub site_id: Option<i64>,     // 活跃站点
    pub roles: Vec<String>,       // 在该租户/站点解析出的角色
    pub permissions: Vec<String>, // 展开后的权限（缓存，见 §4.3）
    pub amr: Vec<String>,         // 认证方式：["pwd"] / ["pwd","otp"] / ["google"]
}
impl Principal {
    pub fn has(&self, perm: &str) -> bool { self.permissions.iter().any(|p| p == perm) }
}
```

### 3.4 与现有 `users` 表的迁移

现有单表 `users(email,password_hash,display_name,role)` 拆解为：

```sql
-- users 保留 id/email；password_hash → user_credentials；display_name → user_profiles；role → memberships
INSERT INTO user_credentials (user_id, password_hash)
    SELECT id, password_hash FROM users_legacy;
INSERT INTO user_profiles (user_id, display_name)
    SELECT id, display_name FROM users_legacy;
-- 所有老用户并入「默认租户/默认站点」，role 映射到 RBAC 角色（owner→owner, editor→editor, viewer→viewer）
INSERT INTO memberships (user_id, tenant_id, role_id)
    SELECT u.id, :default_tenant, r.id
    FROM users_legacy u JOIN roles r ON r.slug = u.role AND r.tenant_id IS NULL;
```

详见 §12 完整迁移与回滚。

---

## §4 权限模型（RBAC + 资源归属 + 可选 ABAC）

### 4.1 模型：角色 + 权限 + 绑定

经典 RBAC + 资源归属（≈ Spring Security `GrantedAuthority` + 方法级 `@PreAuthorize`）：

```sql
CREATE TABLE roles (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT REFERENCES tenants(id) ON DELETE CASCADE,  -- NULL = 系统内置角色（全租户共享）
    slug        TEXT NOT NULL,                  -- owner/admin/editor/author/viewer/自定义
    name_i18n   JSONB NOT NULL,
    is_system   BOOLEAN NOT NULL DEFAULT false, -- 内置角色不可删
    UNIQUE (tenant_id, slug)
);
CREATE TABLE permissions (
    id          BIGSERIAL PRIMARY KEY,
    code        TEXT UNIQUE NOT NULL,           -- 'post:publish' 见 4.2
    description TEXT
);
CREATE TABLE role_permissions (
    role_id        BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id  BIGINT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);
```

- **系统内置角色**（`tenant_id IS NULL`）：owner / admin / editor / author / viewer，开箱即用。
- **租户自定义角色**（`tenant_id = X`）：租户管理员可在内置之上自定义（如「只能管图片的 photo-editor」）。
- 用户的有效角色 = `memberships.role`（组织级） ⊕ `site_grants.role`（站点级覆盖）。

### 4.2 权限目录（命名规范 `domain:action[:scope]`）

`scope` 区分「自己的」`own` 与「任何人的」`any`，落实资源归属：

| code | 含义 | owner | admin | editor | author | viewer |
| --- | --- | :-: | :-: | :-: | :-: | :-: |
| `post:read` | 读内容（含草稿） | ✓ | ✓ | ✓ | ✓ | ✓ |
| `post:create` | 新建文章/图片 | ✓ | ✓ | ✓ | ✓ | |
| `post:edit:own` | 改自己的 | ✓ | ✓ | ✓ | ✓ | |
| `post:edit:any` | 改任何人的 | ✓ | ✓ | ✓ | | |
| `post:publish` | 发布/定时 | ✓ | ✓ | ✓ | | |
| `post:delete:any` | 删任何人的 | ✓ | ✓ | | | |
| `term:manage` | 分类法增删改 | ✓ | ✓ | ✓ | | |
| `media:upload` | 上传媒体 | ✓ | ✓ | ✓ | ✓ | |
| `theme:activate` | 切换/定制主题 | ✓ | ✓ | | | |
| `member:invite` | 邀请成员、改角色 | ✓ | ✓ | | | |
| `tenant:settings` | 租户/站点/计费设置 | ✓ | | | | |
| `oauth_client:manage` | 管理 IdP 接入方（§7） | ✓ | | | | |
| `audit:read` | 查审计日志 | ✓ | ✓ | | | |

> 这张表就是 `permissions` 种子 + 内置角色的 `role_permissions` 种子，建议放迁移的 DML 或 `seed_roles` bin。

### 4.3 鉴权判定（中间件 + 提取器）

权限在**登录签发令牌时解析并缓存**（角色→权限展开），放进 `Principal.permissions`；请求时 O(1) 判断，避免每次查库。角色权限变更时失效缓存 `cms:authz:perms:{user}:{tenant}`。

```rust
// identity/authz.rs —— 一个 Axum 提取器/宏，声明式校验（≈ @PreAuthorize("hasAuthority('post:publish')")）
pub fn require(principal: &Principal, perm: &str) -> Result<(), AppError> {
    if principal.has(perm) { Ok(()) } else { Err(AppError::Forbidden) }
}

/// 资源归属：edit 时先查 own/any
pub fn require_post_edit(principal: &Principal, post_author_id: i64) -> Result<(), AppError> {
    if principal.has("post:edit:any") { return Ok(()); }
    if principal.has("post:edit:own") && post_author_id == principal.user_id { return Ok(()); }
    Err(AppError::Forbidden)
}
```

```rust
// handler 里：
pub async fn publish(State(s): State<AppState>, Extension(p): Extension<Principal>, Path(id): Path<i64>)
    -> AppResult<Json<PostDto>>
{
    authz::require(&p, "post:publish")?;          // RBAC
    post_service::publish(&s, &p, id).await.map(Json)
}
```

### 4.4 何时上 Casbin（ABAC，可选）

当出现「按分类授权」「按内容状态/标签的动态规则」「跨资源的复杂矩阵」时，纯表 RBAC 会膨胀。此时引入 **`casbin` crate**（Rust 官方移植），用 `model.conf` + policy（可存 DB）做策略判定，等价 Java 的 casbin/Sa-Token 注解鉴权。**默认不引入**——先用 §4.1-4.3 的显式 RBAC，足够 CMS 用；ABAC 作为 M 后期可插拔增强。

---

## §5 鉴权与令牌体系重构

### 5.1 为什么必须从 HS256 升级到非对称签名

现状 HS256 用一把共享 `secret` 同时签发与验证。**一旦本站要当 IdP（§7），RP（子系统）必须能验签但不该拿到签发密钥**——对称密钥做不到。所以：

- **签名算法**：改 **RS256**（兼容性最好）或 **EdDSA/Ed25519**（更快更短，推荐新系统）。私钥只在用户中心，公钥经 **JWKS**（`/oauth2/jwks`）公开给所有 RP。
- **密钥轮换**：`signing_keys` 支持多把（带 `kid`），新键签发、旧键仍可验，过期下线。JWT 头带 `kid`，验签方按 `kid` 从 JWKS 选公钥。
- `AppState` 增 `Arc<KeyRing>`（当前 active 签名私钥 + 所有有效公钥），替换现有 `Arc<JwtKeys>`。

```sql
CREATE TABLE signing_keys (
    kid         TEXT PRIMARY KEY,                  -- 如 "2026-07-ed25519-01"
    algorithm   TEXT NOT NULL,                     -- 'EdDSA' | 'RS256'
    private_pem  TEXT NOT NULL,                    -- 生产建议从 KMS/Secrets 注入，不落库或加密落库
    public_jwk  JSONB NOT NULL,                    -- 对外 JWKS 暴露
    status      TEXT NOT NULL DEFAULT 'active'     -- active | retiring | revoked
                CHECK (status IN ('active','retiring','revoked')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ
);
```

### 5.2 Claims 重构（携带租户与角色上下文）

```rust
// infra/jwt.rs 重构后的 access token claims
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub iss: String,        // 签发者，本站 IdP URL（OIDC 必需）
    pub sub: String,        // 用户全局 id
    pub aud: Vec<String>,   // 受众（资源服务/客户端），RP 校验自己在不在内
    pub tid: i64,           // ★ 活跃租户（多租户隔离的关键）
    pub sid: Option<i64>,   // ★ 活跃站点
    pub roles: Vec<String>, // 该租户下角色
    pub scope: String,      // OAuth2 scope（IdP 颁发时用），空格分隔
    pub amr: Vec<String>,   // 认证方式 ["pwd"] / ["pwd","otp"] / ["google"]
    pub iat: i64,
    pub exp: i64,           // 短 TTL：15min
    pub jti: String,
}
```

要点：
- access token **绑定单一活跃租户 `tid`**。切换租户 = 用 refresh 重新换发到目标租户（见 §11 `/auth/switch-tenant`）。这样资源服务只看 `tid` 即可做隔离，无需查库。
- `roles` 进 token 便于粗判；**细权限不进 token**（会过大、会过期），由资源服务用 `roles` 查缓存展开（§4.3）。
- `amr` 支撑「敏感操作要求二次 2FA」的 step-up（§8）。

### 5.3 会话与刷新令牌轮换（防重放）

把现有「refresh 存 Redis」升级为 **DB 会话注册表 + 刷新令牌家族轮换 + 重放检测**：

```sql
CREATE TABLE sessions (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id    UUID NOT NULL,                    -- 一条「登录会话」一个家族
    device_label TEXT,                             -- UA 解析的设备名
    ip_created   INET,
    user_agent   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at   TIMESTAMPTZ
);
CREATE TABLE refresh_tokens (
    jti          UUID PRIMARY KEY,
    family_id    UUID NOT NULL REFERENCES sessions(family_id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL,
    token_hash   TEXT NOT NULL,                    -- 存 hash，不存明文
    expires_at   TIMESTAMPTZ NOT NULL,
    used_at      TIMESTAMPTZ,                      -- 被消费的时间（轮换后置位）
    replaced_by  UUID                              -- 指向轮换出的新 jti
);
```

**轮换 + 重放检测算法**（业界标准 refresh token rotation）：

```text
refresh(rt):
  rec = lookup(rt.jti)
  if rec is None or expired:            → 401
  if rec.used_at is not None:           # 旧令牌被二次使用 = 疑似泄露/重放
        revoke_family(rec.family_id)    # 把整条会话全踢（连级联设备）
        audit("refresh_replay", ...)    → 401
  mark rec.used_at = now, replaced_by = new_jti
  issue new access + new refresh(同 family)   # 家族延续
```

- **单设备登出**：`UPDATE sessions SET revoked_at=now() WHERE id=?`（用户在「会话管理」页点某设备「退出」）。
- **全设备登出**：保留现有 `auth:user-rev:{uid}` epoch 机制 + 批量 revoke sessions。
- Redis 仍做**快速失效缓存**：`auth:blacklist:{jti}`（access 即时拉黑）、`auth:revoked-fam:{family}`（家族踢出位），中间件查 Redis O(1)，DB 为权威。

### 5.4 中间件升级（鉴权 + 租户解析 + RLS 注入）

现有 `jwt_guard` 只验签 + 查黑名单。升级为一条「认证 → 解析主体 → 解析租户 → 绑定 RLS」链：

```rust
// middleware/auth.rs 重构
pub async fn auth_guard(State(s): State<AppState>, mut req: Request<Body>, next: Next)
    -> Result<Response, AppError>
{
    // 1) 验签（按 kid 选公钥）+ 类型/过期校验
    let claims = jwt::verify_access(&s.keyring, bearer(&req)?)?;
    // 2) 快速失效检查（Redis）：jti 黑名单 + 家族踢出 + 用户 epoch
    auth_service::ensure_active(&s, &claims).await?;
    // 3) 展开权限（缓存 cms:authz:perms:{user}:{tid}）
    let principal = authz_service::load_principal(&s, &claims).await?;
    // 4) 解析活跃站点（管理端以 claim 为准；前台另有 Host 解析中间件）
    req.extensions_mut().insert(TenantContext { tenant_id: principal.tenant_id, site_id: principal.site_id.unwrap_or_default() });
    req.extensions_mut().insert(principal);
    Ok(next.run(req).await)
}
```

> 业务 service 取连接后调 `tenant_ctx::bind_tenant(&txn, ctx)`（§2.6）注入 RLS。前台公开路由用独立的 `host_tenant_guard`（按域名解析 site）。

---

## §6 SSO 作为 RP：第三方登录入站

本站作为 **Relying Party**，让用户用外部身份提供方登录（≈ Spring Security `oauth2Login()`）。

### 6.1 支持的 Provider

| Provider | 协议 | 备注 |
| --- | --- | --- |
| Google / 通用 OIDC | OIDC（标准） | 用 `openidconnect` crate，自动发现 `/.well-known` |
| GitHub | OAuth2 + userinfo | 非 OIDC，手动取 `/user` |
| 微信开放平台 / 企业微信 | OAuth2 变体 | `openid`/`unionid` 作 subject；需处理 `unionid` 跨应用同一人 |
| 钉钉 | OAuth2 变体 | 同上，企业内部登录常用 |

抽象一个 `OAuthProvider` trait，把差异收敛：

```rust
// identity/oidc_client/mod.rs
#[async_trait]
pub trait OAuthProvider {
    fn authorize_url(&self, state: &str, nonce: &str, pkce_challenge: &str) -> String;
    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> AppResult<ProviderTokens>;
    async fn fetch_userinfo(&self, tokens: &ProviderTokens) -> AppResult<ExternalIdentity>;
    fn provider_id(&self) -> &str;     // "google" / "github" / "wechat" ...
}
pub struct ExternalIdentity { pub subject: String, pub email: Option<String>, pub name: Option<String>, pub avatar: Option<String>, pub raw: serde_json::Value }
```

### 6.2 登录/绑定流程

```text
[登录]
  浏览器 GET /api/v1/auth/oauth/google/start?site=...
     → 生成 state(防CSRF)+nonce+PKCE，存 Redis(5min)，302 到 Google authorize_url
  Google 回调 GET /api/v1/auth/oauth/google/callback?code=&state=
     → 校验 state；exchange_code；fetch_userinfo → ExternalIdentity
     → 查 user_identities(provider,subject)：
          命中  → 该 user 登录，签发本站令牌（§5）
          未命中→ 按 email 匹配既有 user？
                    有 → 引导「绑定到现有账号」（需验证，防账号劫持）
                    无 → 新建 user + user_identities + 默认 profile，加入目标 tenant（或走邀请制）
[绑定]（已登录用户在「账号安全」页加绑）
  GET /api/v1/me/identities/github/start → 回调后写 user_identities，关联当前 user
```

安全要点：`state` 防 CSRF、`nonce` 防 OIDC 重放、PKCE 防授权码拦截；**「按 email 自动合并账号」必须二次验证**（发确认邮件或要求输入现账号密码），否则攻击者用同 email 的外部账号可劫持。

### 6.3 落地位置

- 路由：`/api/v1/auth/oauth/:provider/start` + `/callback`（公开）；`/api/v1/me/identities/*`（需登录，绑定/解绑）。
- 配置：每租户可在 `tenants` 下配置自己的 OAuth app（client_id/secret），或用平台级默认。存 `tenant_oauth_configs`（provider, client_id, secret_encrypted, enabled）。
- 依赖：`openidconnect`（OIDC）、`oauth2`（底层）。

---

## §7 SSO 作为 IdP：统一认证中心

本站作为 **OAuth2 / OIDC Provider（Authorization Server）**，让其它子系统「用拾光集账号登录」。这是「用户中心」的核心，对标 **Spring Authorization Server**。

### 7.1 支持的流程

| 流程 | 用途 | 是否做 |
| --- | --- | --- |
| Authorization Code + **PKCE** | Web/SPA/移动子系统登录（主力） | ✅ 必做 |
| Refresh Token | 子系统续期 | ✅ |
| Client Credentials | 子系统后端对后端（无用户） | ✅（可选） |
| Device Code | 电视/CLI 等无浏览器设备 | ⏸ 后置 |
| Implicit / ROPC | 已废弃，不做 | ❌ |

### 7.2 IdP 数据表

```sql
CREATE TABLE oauth_clients (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT REFERENCES tenants(id) ON DELETE CASCADE,  -- 该接入方归属租户（NULL=平台级）
    client_id       TEXT UNIQUE NOT NULL,
    client_secret_hash TEXT,                       -- 机密客户端有；public 客户端(SPA)为空，靠 PKCE
    name            TEXT NOT NULL,
    redirect_uris   JSONB NOT NULL,                -- 白名单精确匹配
    grant_types     JSONB NOT NULL DEFAULT '["authorization_code","refresh_token"]',
    scopes          JSONB NOT NULL DEFAULT '["openid","profile","email"]',
    is_confidential BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE oauth_authorization_codes (
    code_hash       TEXT PRIMARY KEY,              -- 存 hash；一次性
    client_id       TEXT NOT NULL,
    user_id         BIGINT NOT NULL,
    tenant_id       BIGINT NOT NULL,
    redirect_uri    TEXT NOT NULL,
    scope           TEXT NOT NULL,
    nonce           TEXT,
    pkce_challenge  TEXT NOT NULL,                 -- S256
    expires_at      TIMESTAMPTZ NOT NULL,          -- 60s
    consumed_at     TIMESTAMPTZ
);
CREATE TABLE oauth_consents (                      -- 记住用户对某 client 的授权同意
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id  TEXT NOT NULL,
    scope      TEXT NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, client_id)
);
-- IdP 颁发给子系统的 refresh，与 §5.3 sessions/refresh_tokens 复用同一套表（family 轮换）
```

### 7.3 IdP 端点（标准 OIDC）

| 端点 | 说明 |
| --- | --- |
| `GET /.well-known/openid-configuration` | OIDC 发现文档（RP 自动配置入口） |
| `GET /oauth2/jwks` | 公钥集（§5.1 的 `signing_keys.public_jwk`），RP 验签用 |
| `GET /oauth2/authorize` | 授权端点：校验 client+redirect_uri+PKCE → 要求登录/同意 → 发 code |
| `POST /oauth2/token` | 令牌端点：code→token（校验 PKCE verifier）/ refresh / client_credentials |
| `GET /oauth2/userinfo` | 用 access token 取用户标准声明（sub/email/name/...） |
| `POST /oauth2/introspect` | 资源服务校验不透明令牌（如用） |
| `POST /oauth2/revoke` | 撤销令牌 |
| `POST /oauth2/register` | 动态客户端注册（可选，否则后台手工建 client） |

`/.well-known/openid-configuration` 示例（节选，便于 RP 接入）：

```jsonc
{
  "issuer": "https://id.gathered-light.app",
  "authorization_endpoint": "https://id.gathered-light.app/oauth2/authorize",
  "token_endpoint": "https://id.gathered-light.app/oauth2/token",
  "userinfo_endpoint": "https://id.gathered-light.app/oauth2/userinfo",
  "jwks_uri": "https://id.gathered-light.app/oauth2/jwks",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code","refresh_token","client_credentials"],
  "code_challenge_methods_supported": ["S256"],
  "id_token_signing_alg_values_supported": ["EdDSA","RS256"],
  "scopes_supported": ["openid","profile","email","offline_access"]
}
```

### 7.4 授权码流程（带 PKCE + 多租户）

```text
子系统(RP) ──/oauth2/authorize?client_id&redirect_uri&scope=openid&code_challenge&state──▶ 用户中心
   用户中心：校验 client/redirect_uri 白名单
            未登录 → 走本站登录页（可叠加 §6 第三方登录、§8 2FA）
            首次授权 → 展示同意页（scope 列表）；已同意过则跳过
            选择活跃租户（用户属多租户时）
   ◀── 302 redirect_uri?code=&state=
子系统 ──POST /oauth2/token (code + code_verifier + client 凭证)──▶ 用户中心
   用户中心：校验 code 一次性/未过期 + PKCE(S256(verifier)==challenge) + redirect_uri 一致
   ◀── { access_token(JWT,RS256/EdDSA), id_token(OIDC), refresh_token, expires_in }
子系统：用 jwks 验 id_token；用 access_token 调自身资源 / 调 /userinfo
```

### 7.5 与 Spring Authorization Server 的对照

| 拾光集 | Spring Authorization Server |
| --- | --- |
| `oauth_clients` 表 | `RegisteredClientRepository` |
| `/oauth2/authorize`/`token`/`jwks` 端点 | 自带的 `OAuth2AuthorizationServerConfigurer` 端点 |
| `signing_keys` + JWKS | `JWKSource<SecurityContext>` |
| `oauth_consents` | `OAuth2AuthorizationConsentService` |
| `AccessClaims`（带 tid/roles） | 自定义 `OAuth2TokenCustomizer` |

> Rust 侧没有「开箱即用的 Authorization Server」框架，但 OIDC 协议本身不复杂；以上端点用 Axum handler + `josekit`/`jsonwebtoken`（签名）+ `oauth2` 类型即可实现。**强烈建议严格按 RFC 6749/7636 + OIDC Core 实现并跑一遍 conformance 测试**，不要自创流程。

---

## §8 多因子认证（TOTP 2FA）

### 8.1 数据与流程

```sql
CREATE TABLE user_mfa (
    user_id        BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    totp_secret    TEXT NOT NULL,                  -- 加密存储（KMS/应用密钥），非明文
    confirmed_at   TIMESTAMPTZ,                    -- 启用确认时间
    recovery_codes JSONB NOT NULL DEFAULT '[]'     -- 一次性恢复码的 hash 列表
);
```

- **启用**：`POST /me/mfa/totp/setup` 生成 secret → 返回 `otpauth://` URI + 二维码 → 用户用 Authenticator 扫码 → `POST /me/mfa/totp/confirm` 提交一次 6 位码校验通过才 `confirmed_at`，同时下发 10 个一次性恢复码（只展示一次）。
- **登录校验**：密码/第三方通过后，若 `users.mfa_enabled`，进入二步：`POST /auth/mfa/verify`（TOTP 或恢复码）通过才签发正式令牌；其间用一个短时「半登录」临时令牌承接。
- **令牌 `amr`**：标记 `["pwd","otp"]`，供 step-up 判定。
- 依赖：`totp-rs`（RFC 6238）。时间窗 ±1 容错；恢复码用后即焚。

### 8.2 Step-up（敏感操作二次验证）

切主题、改密、管理成员、管理 IdP 客户端等敏感操作，可要求 token `amr` 含 `otp` 且新鲜（`auth_time` 在 N 分钟内），否则触发二次验证。等价 Spring Security 的 `ReauthenticationRequired`。

---

## §9 密码策略与登录风控

### 9.1 密码哈希（Argon2id 显式调参）

现状用 `Argon2::default()`。生产应显式固定参数并随硬件升级：

```rust
// identity/password.rs —— 显式 argon2id 参数（OWASP 基线起步，按压测调）
fn argon2() -> Argon2<'static> {
    use argon2::{Algorithm, Params, Version};
    let params = Params::new(19 * 1024 /*KiB≈19MB*/, 2 /*iters*/, 1 /*lanes*/, None).unwrap();
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}
```

- **强度校验**：注册/改密时跑 `zxcvbn`（Rust 移植）估强度，拒绝弱密码（评分 < 3）。
- **泄露库检查（可选）**：HIBP k-anonymity（发 SHA1 前 5 位查 range API），命中则拒。注意需走 `web_fetch` 等受控出网。
- **历史密码**：`must_reset` + 可选「禁止复用最近 N 个」（存历史 hash）。

### 9.2 登录风控

| 措施 | 实现 | 现状 |
| --- | --- | --- |
| 失败锁定 | `auth:login:fail:{email_hash}` INCR，>5 次锁 15min（Redis） | 设计已有，落地 |
| 验证码 | 失败 N 次后要求图形/滑块验证码（前端 + 校验端点） | 新增钩子 |
| 设备/地理异常 | 新设备/新 IP 段登录 → 发邮件提醒 + 记审计 | 新增 |
| 撞库防护 | 全局速率限制（tower-governor）+ 按 IP 限流 | 复用现有限流层 |
| 时序攻击 | 用户不存在也走一次假 verify，恒定耗时 | 加固 |

> 现有 `auth_service::login` 直接 `find_by_email → verify_password`，建议包一层 `login_guard`（查锁定 → 校验 → 失败计数/成功清零 → 风控信号）。

---

## §10 审计日志

### 10.1 表与写入

```sql
CREATE TABLE audit_logs (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT,                            -- 跨租户运营操作可空
    actor_id    BIGINT,                            -- 操作者 user（系统操作为空）
    action      TEXT NOT NULL,                     -- 'post.publish' / 'member.role.change' / 'auth.login' ...
    resource    TEXT,                              -- 'post:123' / 'user:45'
    summary     JSONB,                             -- 关键 diff / 上下文（脱敏）
    ip          INET,
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_tenant_time ON audit_logs(tenant_id, created_at DESC);
CREATE INDEX idx_audit_actor_time  ON audit_logs(actor_id, created_at DESC);
```

- **写哪些**：所有**写操作 + 敏感读**——登录/登出/2FA、改密、成员与角色变更、内容发布/删除、主题切换、IdP 客户端管理、令牌撤销、跨租户操作。
- **怎么写**：service 层统一 `audit_service::record(ctx, action, resource, summary)`；与 `tracing` 联动（同一 `trace_id` 进 summary，便于和日志关联）。**异步落库**（进 channel，worker 批量写），不阻塞主流程。
- **防篡改（可选增强）**：append-only（无 UPDATE/DELETE 权限）+ 每条带前一条的 hash 形成链（轻量防删改）。合规要求高时再上。
- **查询**：`GET /api/v1/admin/audit?actor=&action=&from=&to=`，需 `audit:read` 权限，受租户隔离。

### 10.2 与现有 tracing 的关系

`tracing` 是**技术日志**（排障、性能），`audit_logs` 是**业务事实**（谁在何时做了什么，要长期留存、可被审计/法务查）。两者并存：tracing 进 Loki/ES 短期留存，audit 进 PG 长期留存。

---

## §11 API 总表

> 沿用现有 `/api/v1` 前缀与 `{code,message,data}` 返回包。IdP 端点为标准 OIDC，放根路径（`/oauth2/*`、`/.well-known/*`）便于第三方接入。

### 认证（公开）

| Method | Path | 说明 |
| --- | --- | --- |
| POST | `/api/v1/auth/login` | 密码登录（成功后若需 2FA 返回半登录 token） |
| POST | `/api/v1/auth/mfa/verify` | 提交 TOTP/恢复码，换正式令牌 |
| POST | `/api/v1/auth/refresh` | 刷新令牌轮换（家族 + 重放检测） |
| POST | `/api/v1/auth/logout` | 当前会话登出 |
| POST | `/api/v1/auth/switch-tenant` | 切换活跃租户（refresh 换发到目标 `tid`） |
| GET | `/api/v1/auth/oauth/:provider/start` | 第三方登录起步（RP，§6） |
| GET | `/api/v1/auth/oauth/:provider/callback` | 第三方登录回调 |

### 用户中心 - 自助（需登录）

| Method | Path | 说明 |
| --- | --- | --- |
| GET | `/api/v1/me` | 当前用户 + 资料 + 所属租户列表 + 当前角色/权限 |
| PATCH | `/api/v1/me/profile` | 改昵称/头像/locale |
| POST | `/api/v1/me/password` | 改密码（旧密码 + 强度校验） |
| GET/POST/DELETE | `/api/v1/me/identities[...]` | 联合登录绑定/解绑（§6.2） |
| GET | `/api/v1/me/sessions` | 设备/会话列表 |
| DELETE | `/api/v1/me/sessions/:id` | 退出某设备 |
| POST | `/api/v1/me/mfa/totp/setup`·`/confirm`·`DELETE` | 2FA 启用/确认/关闭（§8） |

### 管理（需对应权限，租户隔离）

| Method | Path | 权限 |
| --- | --- | --- |
| GET/POST | `/api/v1/admin/members` | `member:invite`（邀请/列表） |
| PATCH/DELETE | `/api/v1/admin/members/:id` | `member:invite`（改角色/移除） |
| GET/POST/PATCH/DELETE | `/api/v1/admin/roles[...]` | `tenant:settings`（自定义角色 + 权限分配） |
| GET | `/api/v1/admin/permissions` | 权限目录（只读） |
| GET/POST | `/api/v1/admin/sites[...]` | `tenant:settings`（站点 + 域名管理） |
| GET/POST/PATCH/DELETE | `/api/v1/admin/oauth-clients[...]` | `oauth_client:manage`（IdP 接入方，§7） |
| GET | `/api/v1/admin/audit` | `audit:read`（审计查询，§10） |

### IdP（标准 OIDC，根路径）

`/.well-known/openid-configuration`、`/oauth2/authorize`、`/oauth2/token`、`/oauth2/jwks`、`/oauth2/userinfo`、`/oauth2/introspect`、`/oauth2/revoke`（详见 §7.3）。

---

## §12 数据迁移与兼容

分两条线，均可灰度、可回滚：

### 12.1 身份线（现有 users 拆分）

1. 建新表（§3.2/§4.1/§5/§7/§8/§10），把现有 `users` 重命名 `users_legacy`，新建精简 `users`。
2. 回填：`users`(id,email) ← legacy；`user_credentials` ← legacy.password_hash；`user_profiles` ← legacy.display_name。
3. 种子内置角色 + 权限目录（§4.2）。
4. 建「默认租户 + 默认站点」，把所有老用户写 `memberships`（role 映射 owner/editor/viewer）。
5. 内容表（posts/terms/media/site_settings）回填 `tenant_id=默认租户, site_id=默认站点`，再加 `NOT NULL` 与 RLS 策略。

### 12.2 令牌线（HS256 → 非对称）

1. 生成首把 EdDSA/RS 密钥写 `signing_keys`，`AppState` 用 `KeyRing` 替换 `JwtKeys`。
2. **过渡期双验**：验签时先按 `kid` 走非对称；对**旧 HS256 令牌**保留一个兼容验证分支，直到旧 access（15min）/refresh（7d）全部自然过期。
3. 过期后删除 HS256 兼容分支与 `jwt.secret` 配置。

> 前端契约：`/api/v1/auth/login` 返回结构在 `TokenPairResp` 基础上**新增** `requires_mfa`、`tenants`（可切换租户列表）字段，旧字段保留，前端渐进适配。

---

## §13 缓存与安全要点

### 缓存 key 扩展（沿用 `cms:`/`auth:` 命名）

| Key | TTL | 用途/失效 |
| --- | --- | --- |
| `auth:blacklist:{jti}` | =access 剩余 | access 即时拉黑（保留现状） |
| `auth:user-rev:{uid}` | =refresh ttl | 全设备登出 epoch（保留现状） |
| `auth:revoked-fam:{family}` | =refresh ttl | 会话家族踢出位（新增） |
| `cms:authz:perms:{uid}:{tid}` | 300s | 角色→权限展开缓存；角色/权限变更时 `DEL` |
| `oidc:authcode:{code_hash}` | 60s | 授权码一次性（也可纯 DB） |
| `oidc:jwks` | 1h | 对外 JWKS 响应缓存 |
| `auth:login:fail:{email_hash}` | 15min | 失败计数（落地风控） |

### 安全红线

1. **私钥不出域**：签名私钥从 KMS/Secret 注入，不硬编码、不进前端、不随仓库。
2. **令牌最短必要 TTL**：access 15min；refresh 轮换；id_token 仅登录态。
3. **redirect_uri / client / PKCE 全校验**：IdP 三大件，任一不过即拒（防开放重定向、授权码注入）。
4. **RLS 兜底**：业务连接用受 RLS 约束的角色，杜绝「忘加 tenant 条件」越权。
5. **账号合并需二次验证**（§6.2）；**敏感操作 step-up 2FA**（§8.2）。
6. **审计不可绕过**：写操作统一经 service，审计在事务边界记录。

---

## §14 新增依赖

```toml
# [workspace.dependencies] 增加：
jsonwebtoken = "9"          # 已有；改用 RS256/EdDSA（开 ring/aws-lc 特性）
josekit      = "0.10"       # 可选：更完整的 JWK/JWKS/JWE 处理（IdP 友好）
oauth2       = "5"          # RP/IdP 的 OAuth2 类型与流程
openidconnect= "4"          # RP：OIDC 客户端（自动发现）
totp-rs      = "5"          # 2FA TOTP
zxcvbn       = "3"          # 密码强度
ed25519-dalek= "2"          # EdDSA 签名（或用 rsa = "0.9" 走 RS256）
ipnetwork    = "0.20"       # audit_logs.ip(INET) 映射（SeaORM）
woothee      = "0.13"       # UA 解析设备名（会话管理，可选）
casbin       = "2"          # 可选：ABAC 策略引擎（默认不启用，§4.4）
```

前端新增：TOTP 二维码展示（`qrcode`）、会话/成员/角色管理页、第三方登录按钮、租户切换器。

---

## §15 落地路线图

IAM 体量大，**强烈建议分阶段**，不要一锅端。各阶段独立可上线、可回滚：

| 阶段 | 目标 | DoD |
| --- | --- | --- |
| **I1 身份拆分 + RBAC** | users 拆 identity/credential/profile；roles/permissions/memberships；中间件加载 Principal + 权限判定 | 老用户可登录（兼容）；权限目录生效；越权返回 403；现有接口回归通过 |
| **I2 多租户 + RLS** | tenants/sites/域名解析；内容表加 tenant_id/site_id + RLS；claims 带 tid/sid | 两个租户数据互不可见（含「故意不加 where」也被 RLS 拦）；按域名解析前台站点 |
| **I3 令牌重构 + 会话** | HS256→EdDSA+JWKS（双验过渡）；sessions/refresh 家族轮换 + 重放检测；会话管理页 | 旧令牌平滑过期；重放旧 refresh 触发整族踢出；可单设备登出 |
| **I4 2FA + 风控 + 审计** | TOTP 启用/校验/恢复码；登录锁定/验证码/异常提醒；audit_logs + 查询 | 开 2FA 后两步登录；暴力登录被锁；敏感操作留痕可查 |
| **I5 SSO-RP** | 第三方登录（先 Google/GitHub）+ 账号绑定 | 第三方可登录/绑定；同 email 合并需二次验证 |
| **I6 SSO-IdP** | OIDC Provider：authorize/token/jwks/userinfo + PKCE + consent | 一个示例 RP（甚至本站自己）走通授权码登录；通过 OIDC 基础 conformance |

> 与 CMS 路线图的关系：**I1+I2 应在 CMS_REFACTOR_PROPOSAL 的 M2（内容模型）一起做**——因为 posts 等表建表时就该带 tenant_id/site_id，否则二次加列+回填更痛。I3-I6 可在内容/主题功能之后并行推进。

---

## 附录 A：crate / 模块结构

```
crates/
├── cms-identity/                 # 新增：身份领域（纯类型 + trait，无运行时依赖）
│   └── src/{lib.rs, user.rs, tenant.rs, role.rs, permission.rs, principal.rs, token.rs}
├── cms-entity/                   # 增 entity：users(重构)/user_credentials/user_identities/
│   │                             #   user_profiles/user_mfa/tenants/sites/site_domains/
│   │                             #   memberships/site_grants/roles/permissions/role_permissions/
│   │                             #   sessions/refresh_tokens/signing_keys/oauth_clients/
│   │                             #   oauth_authorization_codes/oauth_consents/audit_logs
└── cms-api/src/
    ├── identity/                 # 新增模块组（未来可整体切出为 identity-service）
    │   ├── authn_service.rs      # 登录/令牌/2FA/风控
    │   ├── authz_service.rs      # Principal 加载 + RBAC 判定 + 权限缓存
    │   ├── tenant_service.rs     # 租户/站点/成员/邀请
    │   ├── tenant_ctx.rs         # TenantContext + RLS 绑定
    │   ├── oidc_client/          # RP：第三方登录（OAuthProvider 实现）
    │   ├── oidc_provider/        # IdP：authorize/token/jwks/userinfo
    │   ├── session_service.rs    # 会话/refresh 家族轮换
    │   └── audit_service.rs
    ├── middleware/auth.rs        # 重构：auth_guard（验签+失效+Principal+租户+RLS）
    ├── infra/{jwt.rs, keyring.rs}# 非对称签名 + JWKS
    └── handlers/{auth,me,member,role,site,oauth_client,oidc,audit}_handler.rs
```

> 衔接确认：本设计与现有 `infra/jwt.rs`(HS256 Claims)、`auth_service.rs`(refresh-in-Redis/blacklist/user-rev)、`cms-entity/users.rs`、`bootstrap.rs::AppState{jwt}` 一一对照演进；所有新增均标注了「替换 / 兼容过渡 / 保留」策略，可灰度落地。


