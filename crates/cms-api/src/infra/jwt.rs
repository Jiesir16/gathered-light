use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode as jwt_decode,
    encode as jwt_encode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::JwtCfg,
    dto::auth_dto::UserDto,
    error::{AppError, AppResult},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub typ: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

pub fn encode(
    cfg: &JwtCfg,
    user: &UserDto,
    token_type: &'static str,
    ttl_secs: u64,
) -> AppResult<String> {
    let iat = chrono::Utc::now().timestamp();
    let exp = iat + ttl_secs as i64;
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: user.role.clone(),
        typ: token_type.into(),
        iat,
        exp,
        jti: Uuid::new_v4().to_string(),
    };

    jwt_encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(cfg.secret.as_bytes()),
    )
    .map_err(|_| AppError::Internal("encode jwt"))
}

pub fn decode(cfg: &JwtCfg, token: &str, expected_type: &'static str) -> AppResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = jwt_decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized("invalid token"))?;

    let claims = token_data.claims;
    if claims.typ != expected_type {
        return Err(AppError::Unauthorized("wrong token type"));
    }

    Ok(claims)
}
