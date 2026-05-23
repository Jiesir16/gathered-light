use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::dto::photo_dto::I18nText;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    50
}

#[derive(Debug, Deserialize, Validate)]
pub struct NewTagReq {
    pub slug: String,
    pub name: I18nText,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTagReq {
    pub slug: Option<String>,
    pub name: Option<I18nText>,
}
