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

#[derive(Debug, Default, Serialize)]
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
    pub src: String,
    pub created_at: String,
}

#[derive(Debug, Default, Serialize)]
pub struct MediaStats {
    pub total: i64,
    pub pending: i64,
    pub processing: i64,
    pub ready: i64,
    pub failed: i64,
}
