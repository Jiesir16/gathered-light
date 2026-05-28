use cms_domain::Privacy;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct I18nText {
    pub zh: String,
    pub en: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryDto {
    pub key: String,
    pub zh: String,
    pub en: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagSummary {
    pub id: i64,
    pub slug: String,
    pub name: I18nText,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoDto {
    pub id: i64,
    pub slug: String,
    /// 默认首图 URL（瀑布流卡片）。等价于 variants.medium。
    /// 保留是为了向后兼容老前端代码。新代码用 variants 字段。
    pub src: String,
    pub cat: String,
    pub title: I18nText,
    pub loc: I18nText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<I18nText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<I18nText>,
    pub date: String,
    pub privacy: Privacy,
    pub tags: Vec<TagSummary>,
    /// 各尺寸 URL：public 照片是永久 URL，locked/private 是 1h presigned。
    /// 缺哪个 variant 就该字段空（worker 尚未跑完时 medium/full/webp 可能为 None）。
    pub variants: PhotoVariants,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PhotoVariants {
    /// 长边 400px，列表缩略
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    /// 长边 900px JPG，瀑布流卡片
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    /// 长边 1800px，lightbox 详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full: Option<String>,
    /// 长边 900px WebP，比 medium 再省 30-40% 带宽（现代浏览器优先用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webp: Option<String>,
    /// 用户上传的原图（10MB+），用于「保存原图」按钮
    /// admin 接口必有；public 接口仅当 privacy=public 时有
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PhotoReq {
    pub src: String,
    pub cat: String,
    pub title: I18nText,
    pub loc: I18nText,
    pub date: String,
    pub privacy: Privacy,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub caption: Option<I18nText>,
    #[serde(default)]
    pub alt_text: Option<I18nText>,
    #[serde(default)]
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoQuery {
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoListQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub privacy: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

#[derive(Debug, Serialize)]
pub struct PhotoListResp {
    pub items: Vec<PhotoDto>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct UnlockReq {
    pub passcode: String,
}

#[derive(Debug, Serialize)]
pub struct UnlockResp {
    pub unlocked: bool,
}

#[derive(Debug, Deserialize)]
pub struct PrivacyReq {
    pub privacy: Privacy,
}

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
    pub mode: BulkTagMode,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BulkTagMode {
    #[default]
    Replace,
    Append,
}

fn default_bulk_mode() -> BulkTagMode {
    BulkTagMode::Replace
}

#[derive(Debug, Serialize)]
pub struct BulkResp {
    pub affected: i64,
    pub skipped: Vec<i64>,
}
