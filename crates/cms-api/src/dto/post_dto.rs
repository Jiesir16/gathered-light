//! 统一 Post（文章/页面）的 DTO。
//!
//! 与图片卡（photo_dto）并行存在：photo 走旧 `photos` 表（兼容期），
//! 文章/页面走新 `posts` 表（post_type = post/page）。详见 docs/CMS_REFACTOR_PROPOSAL.md §2/§3。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

/// 单语言正文输入（编辑器提交 body_json）。
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct ArticleTranslationReq {
    #[validate(length(min = 1, max = 16))]
    pub locale: String,
    #[validate(length(min = 1, max = 300))]
    pub title: String,
    #[serde(default)]
    pub excerpt: Option<String>,
    /// TipTap / ProseMirror 文档 JSON（真源）。
    #[serde(default)]
    pub body_json: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Validate)]
pub struct ArticleReq {
    #[serde(default)]
    pub slug: Option<String>,
    /// post / page（默认 post）。photo 不走此接口。
    #[serde(default = "default_post_type")]
    pub post_type: String,
    /// draft / published / scheduled / private / trash（默认 draft）。
    #[serde(default = "default_status")]
    pub status: String,
    /// public / locked / private（默认 public）。
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub menu_order: i64,
    /// 关联的 term_taxonomy id 列表（分类/标签/系列）。
    #[serde(default)]
    pub term_taxonomy_ids: Vec<i64>,
    #[validate(length(min = 1, message = "at least one translation"))]
    #[validate(nested)]
    pub translations: Vec<ArticleTranslationReq>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArticleTranslationDto {
    pub locale: String,
    pub title: String,
    pub excerpt: Option<String>,
    /// 服务端清洗后的安全 HTML，前台直接渲染。
    pub body_html: Option<String>,
    pub word_count: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArticleDto {
    pub id: i64,
    pub slug: String,
    pub post_type: String,
    pub status: String,
    pub visibility: String,
    pub author_id: Option<i64>,
    pub menu_order: i64,
    pub term_taxonomy_ids: Vec<i64>,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub translations: Vec<ArticleTranslationDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArticleListItem {
    pub id: i64,
    pub slug: String,
    pub post_type: String,
    pub status: String,
    pub visibility: String,
    pub author_id: Option<i64>,
    pub published_at: Option<String>,
    pub updated_at: String,
    /// 各语言的标题/摘要摘要信息（列表展示用）。
    pub translations: Vec<ArticleTranslationSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArticleTranslationSummary {
    pub locale: String,
    pub title: String,
    pub excerpt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleListQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// 过滤：post / page。缺省 post。
    #[serde(default = "default_post_type")]
    pub post_type: String,
    /// 过滤状态：draft/published/...
    #[serde(default)]
    pub status: Option<String>,
    /// slug 关键字（前缀/包含）过滤。
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleListResp {
    pub items: Vec<ArticleListItem>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize)]
pub struct RevisionSummary {
    pub id: i64,
    pub locale: String,
    pub title: String,
    pub author_id: Option<i64>,
    pub created_at: String,
}

fn default_post_type() -> String {
    "post".to_owned()
}
fn default_status() -> String {
    "draft".to_owned()
}
fn default_visibility() -> String {
    "public".to_owned()
}
fn default_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    20
}
