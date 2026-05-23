use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use cms_entity::users::Model;
use sea_orm::DbErr;
use validator::Validate;

use crate::{
    bootstrap::AppState,
    dto::user_dto::{
        NewUserReq, ResetPasswordReq, UpdateUserReq, UserListItem, UserListQuery, UserListResp,
    },
    error::{AppError, AppResult},
    infra::jwt::Claims,
    repositories::user_repo,
};

#[tracing::instrument(skip(state))]
pub async fn list_users(state: &AppState, query: UserListQuery) -> AppResult<UserListResp> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let items = user_repo::list(&state.db, page, page_size)
        .await
        .map_err(db_err)?
        .iter()
        .map(to_item)
        .collect();
    let total = user_repo::count(&state.db).await.map_err(db_err)?;
    Ok(UserListResp {
        items,
        total,
        page,
        page_size,
    })
}

#[tracing::instrument(skip(state, req))]
pub async fn create_user(state: &AppState, req: NewUserReq) -> AppResult<UserListItem> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    validate_role(&req.role)?;
    if user_repo::find_by_email(&state.db, &req.email)
        .await
        .map_err(db_err)?
        .is_some()
    {
        return Err(AppError::Conflict("email already exists"));
    }

    let password_hash = hash_password(&req.password)?;
    let user = user_repo::insert_with_display_name(
        &state.db,
        req.email,
        password_hash,
        req.display_name,
        req.role,
    )
    .await
    .map_err(db_err)?;
    Ok(to_item(&user))
}

#[tracing::instrument(skip(state, current, req), fields(user_id = %current.sub))]
pub async fn update_user(
    state: &AppState,
    current: &Claims,
    id: i64,
    req: UpdateUserReq,
) -> AppResult<UserListItem> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    if let Some(role) = req.role.as_deref() {
        validate_role(role)?;
        if id == current_user_id(current)? {
            return Err(AppError::Forbidden);
        }
    }

    let user = user_repo::update(&state.db, id, req.display_name.map(Some), req.role)
        .await
        .map_err(db_err)?;
    Ok(to_item(&user))
}

#[tracing::instrument(skip(state, req))]
pub async fn reset_password(state: &AppState, id: i64, req: ResetPasswordReq) -> AppResult<()> {
    req.validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let password_hash = hash_password(&req.new_password)?;
    user_repo::update_password(&state.db, id, password_hash)
        .await
        .map_err(db_err)?;
    Ok(())
}

#[tracing::instrument(skip(state, current), fields(user_id = %current.sub))]
pub async fn delete_user(state: &AppState, current: &Claims, id: i64) -> AppResult<()> {
    if id == current_user_id(current)? {
        return Err(AppError::Forbidden);
    }
    user_repo::delete(&state.db, id).await.map_err(db_err)?;
    Ok(())
}

fn to_item(user: &Model) -> UserListItem {
    UserListItem {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
        created_at: user.created_at.to_rfc3339(),
    }
}

fn validate_role(role: &str) -> AppResult<()> {
    match role {
        "owner" | "editor" | "viewer" => Ok(()),
        _ => Err(AppError::Validation(
            "role must be owner, editor, or viewer".to_owned(),
        )),
    }
}

fn current_user_id(current: &Claims) -> AppResult<i64> {
    current
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Unauthorized("invalid token"))
}

fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal("password hash failed"))
        .map(|hash| hash.to_string())
}

fn db_err(error: DbErr) -> AppError {
    match error {
        DbErr::RecordNotFound(_) => AppError::NotFound,
        other => AppError::Other(other.into()),
    }
}
