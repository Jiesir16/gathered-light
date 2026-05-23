use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginReq {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshReq {
    pub refresh_token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserDto {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct TokenPairResp {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub user: UserDto,
}

#[derive(Debug, Serialize)]
pub struct MeResp {
    pub user: UserDto,
}
