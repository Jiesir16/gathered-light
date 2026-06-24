-- 拾光集 · M2+I1+I2：统一 Post 内容模型 + 身份中心 + 多租户(RLS)
-- 设计依据：docs/CMS_REFACTOR_PROPOSAL.md §2 + docs/USER_CENTER_AND_IAM_DESIGN.md §2/§3/§4/§5/§7/§8/§10
--
-- 原则（非破坏式 / 绞杀者）：
--   * 本迁移只「新增」表，不改动现有 users / photos / categories / tags / media_*，旧链路继续工作。
--   * 现有单表 users 暂保留 password_hash/role（过渡期）；新链路用 user_credentials + memberships。
--   * 内容/分类法表建表即带 tenant_id/site_id，并启用行级安全(RLS)，避免二次加列回填。
--   * UUID/INET 一律用 TEXT 落库，避免引入额外 SeaORM feature/依赖；entity 用 String。

-- ════════════════════════════════════════════════════════════════
-- A. 租户 / 站点
-- ════════════════════════════════════════════════════════════════
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
    domain      TEXT UNIQUE NOT NULL,
    is_primary  BOOLEAN NOT NULL DEFAULT false
);

-- ════════════════════════════════════════════════════════════════
-- B. RBAC：角色 / 权限 / 绑定
-- ════════════════════════════════════════════════════════════════
CREATE TABLE roles (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT REFERENCES tenants(id) ON DELETE CASCADE,   -- NULL = 系统内置角色
    slug        TEXT NOT NULL,
    name_i18n   JSONB NOT NULL,
    is_system   BOOLEAN NOT NULL DEFAULT false,
    UNIQUE (tenant_id, slug)
);

CREATE TABLE permissions (
    id          BIGSERIAL PRIMARY KEY,
    code        TEXT UNIQUE NOT NULL,
    description TEXT
);

CREATE TABLE role_permissions (
    role_id        BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id  BIGINT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

-- ════════════════════════════════════════════════════════════════
-- C. 身份：用户卫星表（引用现有 users.id，不改 users 本体）
-- ════════════════════════════════════════════════════════════════
CREATE TABLE user_credentials (
    user_id        BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash  TEXT NOT NULL,
    must_reset     BOOLEAN NOT NULL DEFAULT false,
    password_changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_identities (
    id                BIGSERIAL PRIMARY KEY,
    user_id           BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider          TEXT NOT NULL,
    subject           TEXT NOT NULL,
    email_at_provider TEXT,
    raw_profile       JSONB,
    linked_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
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

CREATE TABLE user_mfa (
    user_id        BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    totp_secret    TEXT NOT NULL,
    confirmed_at   TIMESTAMPTZ,
    recovery_codes JSONB NOT NULL DEFAULT '[]'::jsonb
);

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

CREATE TABLE site_grants (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    site_id     BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    role_id     BIGINT NOT NULL REFERENCES roles(id),
    PRIMARY KEY (user_id, site_id)
);

-- ════════════════════════════════════════════════════════════════
-- D. 会话 / 刷新令牌 / 签名密钥
-- ════════════════════════════════════════════════════════════════
CREATE TABLE sessions (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id    TEXT UNIQUE NOT NULL,            -- UUID 字符串
    device_label TEXT,
    ip_created   TEXT,
    user_agent   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at   TIMESTAMPTZ
);

CREATE TABLE refresh_tokens (
    jti          TEXT PRIMARY KEY,
    family_id    TEXT NOT NULL REFERENCES sessions(family_id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL,
    token_hash   TEXT NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    used_at      TIMESTAMPTZ,
    replaced_by  TEXT
);
CREATE INDEX idx_refresh_family ON refresh_tokens(family_id);

CREATE TABLE signing_keys (
    kid         TEXT PRIMARY KEY,
    algorithm   TEXT NOT NULL,
    private_pem TEXT NOT NULL,
    public_jwk  JSONB NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','retiring','revoked')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ
);

-- ════════════════════════════════════════════════════════════════
-- E. IdP（OAuth2 / OIDC Provider）
-- ════════════════════════════════════════════════════════════════
CREATE TABLE oauth_clients (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT REFERENCES tenants(id) ON DELETE CASCADE,
    client_id       TEXT UNIQUE NOT NULL,
    client_secret_hash TEXT,
    name            TEXT NOT NULL,
    redirect_uris   JSONB NOT NULL DEFAULT '[]'::jsonb,
    grant_types     JSONB NOT NULL DEFAULT '["authorization_code","refresh_token"]'::jsonb,
    scopes          JSONB NOT NULL DEFAULT '["openid","profile","email"]'::jsonb,
    is_confidential BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE oauth_authorization_codes (
    code_hash       TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL,
    user_id         BIGINT NOT NULL,
    tenant_id       BIGINT NOT NULL,
    redirect_uri    TEXT NOT NULL,
    scope           TEXT NOT NULL,
    nonce           TEXT,
    pkce_challenge  TEXT NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    consumed_at     TIMESTAMPTZ
);

CREATE TABLE oauth_consents (
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id  TEXT NOT NULL,
    scope      TEXT NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, client_id)
);

-- ════════════════════════════════════════════════════════════════
-- F. 审计
-- ════════════════════════════════════════════════════════════════
CREATE TABLE audit_logs (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT,
    actor_id    BIGINT,
    action      TEXT NOT NULL,
    resource    TEXT,
    summary     JSONB,
    ip          TEXT,
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_tenant_time ON audit_logs(tenant_id, created_at DESC);
CREATE INDEX idx_audit_actor_time  ON audit_logs(actor_id, created_at DESC);

-- ════════════════════════════════════════════════════════════════
-- G. 统一内容模型（posts 及其卫星表），建表即带 tenant_id/site_id
-- ════════════════════════════════════════════════════════════════
CREATE TABLE posts (
    id                 BIGSERIAL PRIMARY KEY,
    tenant_id          BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    site_id            BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    post_type          TEXT NOT NULL DEFAULT 'post' CHECK (post_type IN ('post','page','photo')),
    slug               TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'draft'
                       CHECK (status IN ('draft','published','scheduled','private','trash')),
    visibility         TEXT NOT NULL DEFAULT 'public'
                       CHECK (visibility IN ('public','locked','private')),
    passcode_hash      TEXT,
    author_id          BIGINT REFERENCES users(id),
    parent_id          BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    primary_asset_id   BIGINT REFERENCES media_assets(id),
    featured_asset_id  BIGINT REFERENCES media_assets(id),
    menu_order         BIGINT NOT NULL DEFAULT 0,
    taken_at_label     TEXT NOT NULL DEFAULT '',
    taken_at_date      DATE,
    published_at       TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (site_id, post_type, slug)
);
CREATE INDEX idx_posts_listing
    ON posts (tenant_id, site_id, post_type, status, visibility, menu_order DESC, published_at DESC);
CREATE INDEX idx_posts_published
    ON posts (site_id, post_type, published_at DESC)
    WHERE status = 'published' AND visibility = 'public';

CREATE TABLE post_translations (
    id          BIGSERIAL PRIMARY KEY,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tenant_id   BIGINT NOT NULL,
    locale      TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    excerpt     TEXT,
    body_json   JSONB,
    body_html   TEXT,
    location    TEXT NOT NULL DEFAULT '',
    caption     TEXT,
    alt_text    TEXT,
    word_count  INT NOT NULL DEFAULT 0,
    UNIQUE (post_id, locale)
);
CREATE INDEX idx_post_tr_locale ON post_translations(locale);

CREATE TABLE terms (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    site_id     BIGINT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    slug        TEXT NOT NULL,
    name_i18n   JSONB NOT NULL,
    UNIQUE (site_id, slug)
);

CREATE TABLE term_taxonomies (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   BIGINT NOT NULL,
    term_id     BIGINT NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
    taxonomy    TEXT NOT NULL CHECK (taxonomy IN ('category','tag','series')),
    parent_id   BIGINT REFERENCES term_taxonomies(id) ON DELETE SET NULL,
    sort_order  INT NOT NULL DEFAULT 0,
    UNIQUE (term_id, taxonomy)
);
CREATE INDEX idx_tt_taxonomy ON term_taxonomies(taxonomy);

CREATE TABLE post_terms (
    post_id            BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    term_taxonomy_id   BIGINT NOT NULL REFERENCES term_taxonomies(id) ON DELETE CASCADE,
    tenant_id          BIGINT NOT NULL,
    PRIMARY KEY (post_id, term_taxonomy_id)
);

CREATE TABLE post_revisions (
    id          BIGSERIAL PRIMARY KEY,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tenant_id   BIGINT NOT NULL,
    locale      TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    body_json   JSONB,
    author_id   BIGINT REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_revisions_post ON post_revisions(post_id, created_at DESC);

CREATE TABLE post_meta (
    id         BIGSERIAL PRIMARY KEY,
    post_id    BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tenant_id  BIGINT NOT NULL,
    meta_key   TEXT NOT NULL,
    meta_value JSONB,
    UNIQUE (post_id, meta_key)
);

-- ════════════════════════════════════════════════════════════════
-- H. 行级安全(RLS)：内容/分类法表按当前会话租户硬隔离
--    策略用 current_setting('app.tenant_id', true)：未设置时返回 NULL → 不匹配 → 零行（安全默认）
--    应用侧每请求事务内 `SET LOCAL app.tenant_id = '<tid>'`（见 IAM §2.6）
-- ════════════════════════════════════════════════════════════════
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'posts','post_translations','terms','term_taxonomies','post_terms','post_revisions','post_meta'
    ] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY;', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY;', t);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON %I '
            || 'USING (tenant_id = current_setting(''app.tenant_id'', true)::bigint) '
            || 'WITH CHECK (tenant_id = current_setting(''app.tenant_id'', true)::bigint);', t);
    END LOOP;
END $$;

-- ════════════════════════════════════════════════════════════════
-- I. 种子：默认租户 + 默认站点 + 系统角色 + 权限目录 + 角色权限
-- ════════════════════════════════════════════════════════════════
INSERT INTO tenants (slug, name) VALUES ('default', 'Default Tenant');
INSERT INTO sites (tenant_id, slug, name)
    SELECT id, 'default', 'Default Site' FROM tenants WHERE slug = 'default';

INSERT INTO roles (tenant_id, slug, name_i18n, is_system) VALUES
    (NULL, 'owner',  '{"zh":"所有者","en":"Owner"}',   true),
    (NULL, 'admin',  '{"zh":"管理员","en":"Admin"}',   true),
    (NULL, 'editor', '{"zh":"编辑","en":"Editor"}',     true),
    (NULL, 'author', '{"zh":"作者","en":"Author"}',     true),
    (NULL, 'viewer', '{"zh":"只读","en":"Viewer"}',     true);

INSERT INTO permissions (code, description) VALUES
    ('post:read',           'Read content including drafts'),
    ('post:create',         'Create posts/photos'),
    ('post:edit:own',       'Edit own content'),
    ('post:edit:any',       'Edit anyone''s content'),
    ('post:publish',        'Publish / schedule'),
    ('post:delete:any',     'Delete anyone''s content'),
    ('term:manage',         'Manage taxonomies'),
    ('media:upload',        'Upload media'),
    ('theme:activate',      'Switch / customize theme'),
    ('member:invite',       'Invite members, change roles'),
    ('tenant:settings',     'Tenant / site / billing settings'),
    ('oauth_client:manage', 'Manage IdP relying parties'),
    ('audit:read',          'Read audit logs');

-- owner: 全部权限
INSERT INTO role_permissions (role_id, permission_id)
    SELECT r.id, p.id FROM roles r CROSS JOIN permissions p
    WHERE r.tenant_id IS NULL AND r.slug = 'owner';

-- admin
INSERT INTO role_permissions (role_id, permission_id)
    SELECT r.id, p.id FROM roles r JOIN permissions p
        ON p.code IN ('post:read','post:create','post:edit:own','post:edit:any','post:publish',
                      'post:delete:any','term:manage','media:upload','theme:activate',
                      'member:invite','audit:read')
    WHERE r.tenant_id IS NULL AND r.slug = 'admin';

-- editor
INSERT INTO role_permissions (role_id, permission_id)
    SELECT r.id, p.id FROM roles r JOIN permissions p
        ON p.code IN ('post:read','post:create','post:edit:own','post:edit:any','post:publish',
                      'term:manage','media:upload')
    WHERE r.tenant_id IS NULL AND r.slug = 'editor';

-- author
INSERT INTO role_permissions (role_id, permission_id)
    SELECT r.id, p.id FROM roles r JOIN permissions p
        ON p.code IN ('post:read','post:create','post:edit:own','media:upload')
    WHERE r.tenant_id IS NULL AND r.slug = 'author';

-- viewer
INSERT INTO role_permissions (role_id, permission_id)
    SELECT r.id, p.id FROM roles r JOIN permissions p
        ON p.code IN ('post:read')
    WHERE r.tenant_id IS NULL AND r.slug = 'viewer';
