-- 拾光集 · 初始化 schema
-- 与 docs/ARCHITECTURE.md §2 完全一致。所有表带 created_at/updated_at（除 join 表）。

CREATE EXTENSION IF NOT EXISTS citext;

-- ─────────────────────────────────────────────────────────────────
-- users
-- ─────────────────────────────────────────────────────────────────
CREATE TABLE users (
    id              BIGSERIAL PRIMARY KEY,
    email           CITEXT      UNIQUE NOT NULL,
    password_hash   TEXT        NOT NULL,
    display_name    TEXT,
    role            TEXT        NOT NULL DEFAULT 'editor'
                    CHECK (role IN ('owner','editor','viewer')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────
-- categories（与前端 design/data.jsx 中 CATEGORIES 数组对齐）
-- ─────────────────────────────────────────────────────────────────
CREATE TABLE categories (
    id          BIGSERIAL  PRIMARY KEY,
    slug        TEXT       UNIQUE NOT NULL,
    name_i18n   JSONB      NOT NULL,            -- {"zh":"街拍","en":"Street"}
    sort_order  INT        NOT NULL DEFAULT 0
);

-- ─────────────────────────────────────────────────────────────────
-- media_assets（OSS 物理对象 1:1）
-- ─────────────────────────────────────────────────────────────────
CREATE TABLE media_assets (
    id              BIGSERIAL  PRIMARY KEY,
    storage_key     TEXT       UNIQUE NOT NULL,
    mime_type       TEXT       NOT NULL,
    width           INT,
    height          INT,
    byte_size       BIGINT,
    checksum_sha256 TEXT,
    exif            JSONB      NOT NULL DEFAULT '{}'::jsonb,
    status          TEXT       NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','processing','ready','failed')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_media_assets_status ON media_assets(status);

CREATE TABLE media_variants (
    id          BIGSERIAL  PRIMARY KEY,
    asset_id    BIGINT     NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    variant     TEXT       NOT NULL,           -- thumb_400 / medium_900 / full_1800 / webp_900
    storage_key TEXT       NOT NULL,
    width       INT        NOT NULL,
    height      INT        NOT NULL,
    UNIQUE (asset_id, variant)
);

-- ─────────────────────────────────────────────────────────────────
-- photos（前端 PhotoCard 1:1）
-- ─────────────────────────────────────────────────────────────────
CREATE TABLE photos (
    id                 BIGSERIAL  PRIMARY KEY,
    slug               TEXT       UNIQUE NOT NULL,
    category_id        BIGINT     REFERENCES categories(id),
    primary_asset_id   BIGINT     NOT NULL REFERENCES media_assets(id),
    privacy            TEXT       NOT NULL DEFAULT 'public'
                       CHECK (privacy IN ('public','locked','private')),
    passcode_hash      TEXT,                                        -- argon2(passcode)，仅 locked
    taken_at_label     TEXT       NOT NULL DEFAULT '',              -- "2024.05" 原样
    taken_at_date      DATE,                                        -- 用于 ORDER BY / 区间查询
    uploaded_by        BIGINT     REFERENCES users(id),
    sort_order         BIGINT     NOT NULL DEFAULT 0,
    published_at       TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_photos_listing
    ON photos (privacy, category_id, sort_order DESC, taken_at_date DESC);
CREATE INDEX idx_photos_published_desc
    ON photos (published_at DESC) WHERE privacy = 'public';

-- ─────────────────────────────────────────────────────────────────
-- photo_translations（i18n 子表，避免 jsonb 检索成本）
-- ─────────────────────────────────────────────────────────────────
CREATE TABLE photo_translations (
    id          BIGSERIAL  PRIMARY KEY,
    photo_id    BIGINT     NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    locale      TEXT       NOT NULL,                 -- "zh" / "en" / ...
    title       TEXT       NOT NULL DEFAULT '',
    location    TEXT       NOT NULL DEFAULT '',
    caption     TEXT,
    alt_text    TEXT,
    UNIQUE (photo_id, locale)
);
CREATE INDEX idx_pt_locale ON photo_translations(locale);

-- ─────────────────────────────────────────────────────────────────
-- tags / photo_tags
-- ─────────────────────────────────────────────────────────────────
CREATE TABLE tags (
    id         BIGSERIAL PRIMARY KEY,
    slug       TEXT      UNIQUE NOT NULL,
    name_i18n  JSONB     NOT NULL
);

CREATE TABLE photo_tags (
    photo_id   BIGINT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    tag_id     BIGINT NOT NULL REFERENCES tags(id)   ON DELETE CASCADE,
    PRIMARY KEY (photo_id, tag_id)
);

-- ─────────────────────────────────────────────────────────────────
-- 种子：与 design/data.jsx CATEGORIES 同步（不含 "all"）
-- ─────────────────────────────────────────────────────────────────
INSERT INTO categories (slug, name_i18n, sort_order) VALUES
    ('street',    '{"zh":"街拍","en":"Street"}',    10),
    ('landscape', '{"zh":"风景","en":"Landscape"}', 20),
    ('life',      '{"zh":"生活","en":"Life"}',      30);
