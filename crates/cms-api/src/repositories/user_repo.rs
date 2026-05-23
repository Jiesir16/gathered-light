use cms_entity::users::{ActiveModel, Column, Entity as Users, Model};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};

pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> Result<Option<Model>, DbErr> {
    Users::find().filter(Column::Email.eq(email)).one(db).await
}

pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<Model>, DbErr> {
    Users::find_by_id(id).one(db).await
}

pub async fn insert(
    db: &DatabaseConnection,
    email: String,
    password_hash: String,
    role: String,
) -> Result<Model, DbErr> {
    ActiveModel {
        email: Set(email),
        password_hash: Set(password_hash),
        role: Set(role),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn insert_with_display_name(
    db: &DatabaseConnection,
    email: String,
    password_hash: String,
    display_name: Option<String>,
    role: String,
) -> Result<Model, DbErr> {
    ActiveModel {
        email: Set(email),
        password_hash: Set(password_hash),
        display_name: Set(display_name),
        role: Set(role),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn list(db: &DatabaseConnection, page: u32, page_size: u32) -> Result<Vec<Model>, DbErr> {
    let page = page.max(1);
    let page_size = page_size.max(1);
    Users::find()
        .order_by_desc(Column::Id)
        .paginate(db, page_size as u64)
        .fetch_page((page - 1) as u64)
        .await
}

pub async fn count(db: &DatabaseConnection) -> Result<i64, DbErr> {
    Users::find().count(db).await.map(|n| n as i64)
}

pub async fn update(
    db: &DatabaseConnection,
    id: i64,
    display_name: Option<Option<String>>,
    role: Option<String>,
) -> Result<Model, DbErr> {
    let mut model: ActiveModel = Users::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("user".into()))?
        .into();
    if let Some(display_name) = display_name {
        model.display_name = Set(display_name);
    }
    if let Some(role) = role {
        model.role = Set(role);
    }
    model.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    Users::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn update_password(db: &DatabaseConnection, id: i64, hash: String) -> Result<(), DbErr> {
    let mut model: ActiveModel = Users::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("user".into()))?
        .into();
    model.password_hash = Set(hash);
    model.update(db).await?;
    Ok(())
}
