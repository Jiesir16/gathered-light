# gathered-light · 自主构建任务书（给 Claude Code 跑 loop 用）

> 把这份文件当作长期目标 + 循环工作流。一次做一个 slice，做完即验证即提交，直到 backlog 全部打勾。

## 目标（Goal）

把 gathered-light 从「图片 MVP」改造为 **仿 WordPress 的多租户 Headless CMS**：统一 Post 内容模型 + TipTap 文章、可插拔主题、完整用户中心/IAM（多租户硬隔离、RBAC、RP+IdP 的 SSO、2FA、会话轮换、审计）。终态要求：**代码实现、可编译、有测试、可运行**。

## 真理来源（先读，按它做）

1. `docs/CMS_REFACTOR_PROPOSAL.md` — 内容模型 / TipTap / 主题引擎
2. `docs/USER_CENTER_AND_IAM_DESIGN.md` — 多租户 / 身份 / RBAC / 令牌 / SSO / 2FA / 审计
3. `docs/ARCHITECTURE.md` — 既有约定与硬约束

冲突时：**代码与文档冲突→以文档为准；文档没写→模仿现有代码模式**。不要自创新架构风格。

## 已完成（不要重做）

- 分支 `feat/unified-post-iam-multitenant`。
- 迁移 `m20260701_000004_unified_content_and_iam`：26 张表（posts/post_translations/terms/term_taxonomies/post_terms/post_revisions/post_meta；tenants/sites/site_domains；roles/permissions/role_permissions；user_credentials/user_identities/user_profiles/user_mfa；memberships/site_grants；sessions/refresh_tokens/signing_keys；oauth_clients/oauth_authorization_codes/oauth_consents；audit_logs）+ 内容表 RLS + 种子（默认 tenant/site = id 1、5 个系统角色、13 条权限目录、角色授权）。
- `cms-entity`：26 个实体（最小 Relation，按需补关系）。`cms-domain`：`PostType`/`PostStatus`。
- 文章后端：`services/content_render.rs`（TipTap JSON→HTML + ammonia 清洗 + 摘要/字数）、`dto/post_dto.rs`、`repositories/post_repo.rs`（每操作 `SET LOCAL app.tenant_id` 触发 RLS）、`services/post_service.rs`、`handlers/post_handler.rs`；路由 `/api/v1/posts`、`/posts/:slug`、`/admin/posts[/:id][/revisions]`。当前用默认租户/站点 (1,1)，待 S1 接入真实 Principal。

**第一件事**：`cargo check -p cms-domain -p cms-entity -p cms-api` 跑通已提交代码，修掉编译错误（重点怀疑：ammonia Builder 链式 API、sea-orm `offset/limit/contains`、validator 在 `Vec` 上的 `#[validate(nested)]`）。修完提交，再开始 backlog。

## 硬约束（每个 slice 的 DoD 闸门）

1. **非破坏**：现有 photos / auth / users / settings 链路保持可编译、测试通过。绞杀者式演进，不删现役表/字段直到替代物上线并验证。
2. **模仿模式**：handlers→services→repositories 分层；`AppError`/`AppResult`；`infra::cache::*`；`fred`；sea-orm `ActiveModel` 写、`Statement`/查询构造器读。
3. **先验证再前进**：`cargo check -p cms-api` + `cargo clippy --all-targets -- -D warnings` 全绿；`cargo test`；迁移 `up`→`down`→`up` 在测试库干净可逆；新接口手工冒烟。**红的不许往下走**。
4. **多租户**：所有内容/身份查询带 tenant 维度；事务内 `SET LOCAL app.tenant_id`；生产 DB 角色用**非超级用户**（否则 RLS 被绕过）。
5. **安全**：入库 HTML 一律 ammonia；IdP 用 RS256/EdDSA + JWKS（IdP 上线后停 HS256，滚动期双验）；授权码必须 PKCE；口令 argon2id；**绝不打印密钥/令牌**。
6. **每个 slice 一次提交**（conventional commit），并更新本文件 backlog 勾选 + 在 `docs/BUILD_LOG.md` 追加两行小结。

## 循环（Loop）

重复直到 backlog 全勾：

1. 取最上面未勾的 slice。
2. 完整实现（后端 + 测试；列了前端的连前端）。
3. 跑 DoD 验证（约束 3）。
4. `git commit`；勾选；`docs/BUILD_LOG.md` 记两行。
5. 文档没覆盖的小决策：取与文档一致的最小合理方案，记进 BUILD_LOG，继续。**只有真被挡住**（如需要真实第三方密钥/凭证）才停下来问。

## Backlog（有序；每行一个 slice + 其 DoD）

- [ ] **S1 身份地基**：迁移把现有 `users` 拆出 `user_credentials`/`user_profiles` + 写入 `memberships`(默认租户, role 映射 owner/editor/viewer)；登录改查 `user_credentials`；构建 `Principal{user_id,tenant_id,site_id,roles,permissions}`（角色→权限展开缓存 `cms:authz:perms:{uid}:{tid}`）；`jwt_guard`→`auth_guard`（验签 + 黑名单 + 载入 Principal + 解析租户 + 注入 `TenantContext`）；`authz::require(perm)` / `require_post_edit(own|any)`。**DoD**：旧登录仍可用；viewer 调 `post:create` 返回 403；租户 2 看不到租户 1 的 posts（RLS 实测）。
- [ ] **S2 文章鉴权**：`post_handler` 去掉默认 (1,1)，改用 `Principal`/`TenantContext`；`author_id` 取自 principal；强制 `post:create/edit:own|any/publish/delete`。**DoD**：归属规则生效 + 测试。
- [ ] **S3 分类法**：`terms`/`term_taxonomies` CRUD + 给 post 绑定；category/tag 作为 taxonomy；前台分类列表缓存。**DoD**：建分类→挂到文章→按分类列出。
- [ ] **S4 主题引擎**：启动扫 `themes/`（theme.toml + tokens.json）装配 `ThemeRegistry` 到 `AppState`；`GET /api/v1/theme`（active tokens ⊕ mods，缓存 `cms:theme:active`）；管理端 list/activate/customize 落 `site_settings`（`active_theme`、`theme_mods:<slug>` 按 site）；旧 warm/cool/bold 映射到内置主题。**DoD**：API 切主题→`/theme` 即变。
- [ ] **S5 令牌重构**：`signing_keys` + `KeyRing`(AppState)；access 改 RS256/EdDSA 带 `kid`；claims 加 `tid/sid/roles/amr/iss/aud`；`/oauth2/jwks` + `/.well-known/openid-configuration`；旧 HS256 滚动期双验直至过期。**DoD**：令牌经 JWKS 可验；过渡期旧令牌仍接受。
- [ ] **S6 会话 + 轮换**：`sessions`/`refresh_tokens` 家族轮换 + 重放检测（旧 refresh 被重用→踢整族）；`GET/DELETE /me/sessions`；全设备登出沿用 `auth:user-rev`。**DoD**：重放旧 refresh 触发整族失效。
- [ ] **S7 2FA TOTP**：`/me/mfa/totp` setup/confirm/disable + 恢复码；`mfa_enabled` 时登录两步；`amr=otp`；敏感操作 step-up。**DoD**：开 2FA 后登录需验证码。
- [ ] **S8 口令策略 + 风控**：argon2id 显式调参；zxcvbn 强度；失败锁定（`auth:login:fail`）；用户不存在也恒定耗时。**DoD**：弱口令被拒；5 次失败锁 15 分钟。
- [ ] **S9 审计**：`audit_service::record`（异步）+ `audit_logs`；auth/member/role/post/theme/oauth 写操作留痕；`GET /admin/audit`（权限 `audit:read`、租户隔离）。**DoD**：发布文章写入一条审计。
- [ ] **S10 SSO·RP**：`OAuthProvider` trait；先 Google + GitHub；`/auth/oauth/:provider/start|callback`；`user_identities`；同邮箱合并需二次验证；`/me/identities`。**DoD**：Google 登录创建/绑定用户。
- [ ] **S11 SSO·IdP**：`oauth_clients` 管理 CRUD；`/oauth2/authorize`（PKCE + redirect_uri 白名单 + consent）+ `/oauth2/token` + `/oauth2/userinfo` + revoke/introspect。**DoD**：示例 client 走通授权码+PKCE 并拿到可经 JWKS 验签的 JWT。
- [ ] **S12 前端·内容+主题**：TipTap `ArticleEditor`（只回传 JSON）、后台文章列表/编辑、正文插图走现有 presign；主题运行时 `applyTheme`（`/theme`→CSS 变量）、`ArticleView` 渲染 body_html、`ThemeGallery`+`ThemeCustomizer`。**DoD**：UI 里写/发文章；前台实时换主题。
- [ ] **S13 前端·IAM**：登录 + 2FA + 社会化登录按钮；租户切换器；成员/角色管理；会话页。**DoD**：UI 完成含 2FA 的完整登录。
- [ ] **S14 绞杀 photos→posts（可选收尾）**：`/photos` 改由 posts(post_type=photo) 供数据、保持现有 `PhotoDto` 形状；迁移数据；再下线旧 `photos`。**DoD**：前端 `/photos` 行为不变，底层走 posts。

## 常用命令

```bash
docker compose up -d postgres redis minio
DATABASE_URL=postgres://postgres:postgres@localhost:5432/gathered_light cargo run -p migrations -- up
cargo run -p cms-api
cd frontend && npm i && npm run dev
# 每个 slice 验证：
cargo check -p cms-api && cargo clippy --all-targets -- -D warnings && cargo test
```

## 新增依赖（按 slice 引入，已在文档列出）

`jsonwebtoken`(RS256/EdDSA) · `josekit`/`rsa`/`ed25519-dalek` · `oauth2` · `openidconnect` · `totp-rs` · `zxcvbn` · `ipnetwork`(若改 INET) · `woothee`(UA) · 可选 `casbin`。前端：`@tiptap/*`、`qrcode`。
