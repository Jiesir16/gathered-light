use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct UserListItem {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UserListResp {
    pub items: Vec<UserListItem>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

#[derive(Debug, Deserialize, Validate)]
pub struct NewUserReq {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub display_name: Option<String>,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "editor".into()
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserReq {
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordReq {
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}
