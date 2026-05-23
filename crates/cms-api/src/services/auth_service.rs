use argon2::{Argon2, PasswordVerifier, password_hash::PasswordHash};
use axum::http::HeaderMap;
use fred::{interfaces::KeysInterface, prelude::Expiration};
use sea_orm::DbErr;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::auth_dto::{LoginReq, MeResp, RefreshReq, TokenPairResp, UserDto},
    error::{AppError, AppResult},
    infra::jwt::{self, Claims},
    repositories::user_repo,
};

pub async fn login(state: &AppState, req: LoginReq) -> AppResult<TokenPairResp> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let user = user_repo::find_by_email(&state.db, &req.email)
        .await
        .map_err(db_err)?
        .ok_or(AppError::Unauthorized("invalid credentials"))?;

    verify_password(&user.password_hash, &req.password)?;
    issue_pair(state, user_to_dto(&user)).await
}

pub async fn refresh(state: &AppState, req: RefreshReq) -> AppResult<TokenPairResp> {
    let claims = jwt::decode(&state.config.jwt, &req.refresh_token, "refresh")?;
    ensure_refresh_active(state, &claims).await?;
    ensure_user_not_revoked(state, &claims).await?;

    let user_id = parse_user_id(&claims.sub)?;
    let user = user_repo::find_by_id(&state.db, user_id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::Unauthorized("invalid token"))?;

    let old_key = refresh_key(&claims.jti);
    let _: i64 = state.redis.del(old_key.as_str()).await.map_err(redis_err)?;
    issue_pair(state, user_to_dto(&user)).await
}

pub async fn logout(state: &AppState, claims: &Claims) -> AppResult<()> {
    let ttl = (claims.exp - chrono::Utc::now().timestamp()).max(1);
    let key = format!("auth:blacklist:{}", claims.jti);
    let _: () = state
        .redis
        .set(key.as_str(), "1", Some(Expiration::EX(ttl)), None, false)
        .await
        .map_err(redis_err)?;
    Ok(())
}

pub async fn logout_all(state: &AppState, claims: &Claims) -> AppResult<()> {
    let key = format!("auth:user-rev:{}", claims.sub);
    let epoch = chrono::Utc::now().timestamp().to_string();
    let _: () = state
        .redis
        .set(
            key.as_str(),
            epoch,
            Some(Expiration::EX(state.config.jwt.refresh_ttl_secs as i64)),
            None,
            false,
        )
        .await
        .map_err(redis_err)?;
    Ok(())
}

pub async fn me(state: &AppState, claims: &Claims) -> AppResult<MeResp> {
    let user_id = parse_user_id(&claims.sub)?;
    let user = user_repo::find_by_id(&state.db, user_id)
        .await
        .map_err(db_err)?
        .ok_or(AppError::Unauthorized("invalid token"))?;
    Ok(MeResp {
        user: user_to_dto(&user),
    })
}

pub async fn require_access(state: &AppState, headers: &HeaderMap) -> AppResult<Claims> {
    let token = bearer_token_from(headers)?;
    let claims = jwt::decode(&state.config.jwt, token, "access")?;

    let blacklist_key = format!("auth:blacklist:{}", claims.jti);
    let blacklisted: i64 = state
        .redis
        .exists(blacklist_key.as_str())
        .await
        .map_err(redis_err)?;
    if blacklisted > 0 {
        return Err(AppError::Unauthorized("token revoked"));
    }

    ensure_user_not_revoked(state, &claims).await?;
    Ok(claims)
}

pub fn bearer_token_from(headers: &HeaderMap) -> AppResult<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
        })
        .ok_or(AppError::Unauthorized("missing bearer token"))
}

async fn issue_pair(state: &AppState, user: UserDto) -> AppResult<TokenPairResp> {
    let cfg = &state.config.jwt;
    let access_token = jwt::encode(cfg, &user, "access", cfg.access_ttl_secs)?;
    let refresh_token = jwt::encode(cfg, &user, "refresh", cfg.refresh_ttl_secs)?;
    let refresh_claims = jwt::decode(cfg, &refresh_token, "refresh")?;
    let key = refresh_key(&refresh_claims.jti);
    let _: () = state
        .redis
        .set(
            key.as_str(),
            user.id.to_string(),
            Some(Expiration::EX(cfg.refresh_ttl_secs as i64)),
            None,
            false,
        )
        .await
        .map_err(redis_err)?;

    Ok(TokenPairResp {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: cfg.access_ttl_secs,
        user,
    })
}

async fn ensure_refresh_active(state: &AppState, claims: &Claims) -> AppResult<()> {
    let key = refresh_key(&claims.jti);
    let stored_user_id: Option<String> = state.redis.get(key.as_str()).await.map_err(redis_err)?;
    match stored_user_id {
        Some(user_id) if user_id == claims.sub => Ok(()),
        _ => Err(AppError::Unauthorized("invalid token")),
    }
}

async fn ensure_user_not_revoked(state: &AppState, claims: &Claims) -> AppResult<()> {
    let key = format!("auth:user-rev:{}", claims.sub);
    let epoch: Option<String> = state.redis.get(key.as_str()).await.map_err(redis_err)?;
    if let Some(epoch) = epoch.and_then(|value| value.parse::<i64>().ok()) {
        if claims.iat <= epoch {
            return Err(AppError::Unauthorized("session terminated"));
        }
    }
    Ok(())
}

fn verify_password(hash: &str, password: &str) -> AppResult<()> {
    let parsed =
        PasswordHash::new(hash).map_err(|_| AppError::Unauthorized("invalid credentials"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::Unauthorized("invalid credentials"))
}

fn user_to_dto(user: &cms_entity::users::Model) -> UserDto {
    UserDto {
        id: user.id,
        email: user.email.clone(),
        display_name: user
            .display_name
            .clone()
            .unwrap_or_else(|| "Gathered Light Admin".to_owned()),
        role: user.role.clone(),
    }
}

fn parse_user_id(sub: &str) -> AppResult<i64> {
    sub.parse::<i64>()
        .map_err(|_| AppError::Unauthorized("invalid token"))
}

fn refresh_key(jti: &str) -> String {
    format!("auth:refresh:{jti}")
}

fn db_err(error: DbErr) -> AppError {
    match error {
        DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}

fn redis_err(error: fred::error::RedisError) -> AppError {
    AppError::Other(error.into())
}
