use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::dto::photo_dto::I18nText;

#[derive(Debug, Clone, Serialize)]
pub struct CategoryAdminDto {
    pub id: i64,
    pub slug: String,
    pub name: I18nText,
    pub sort_order: i32,
    pub photo_count: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct NewCategoryReq {
    pub slug: String,
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
