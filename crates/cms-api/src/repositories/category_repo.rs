use cms_entity::{categories, photos};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, Set, Statement,
};
use serde_json::{Value, json};

#[derive(Debug, FromQueryResult)]
pub struct CategoryWithCount {
    pub id: i64,
    pub slug: String,
    pub name_i18n: Value,
    pub sort_order: i32,
    pub photo_count: i64,
}

pub async fn list(db: &DatabaseConnection) -> Result<Vec<CategoryWithCount>, DbErr> {
    let sql = r#"
SELECT
    c.id,
    c.slug,
    c.name_i18n,
    c.sort_order,
    COUNT(p.id)::BIGINT AS photo_count
FROM categories c
LEFT JOIN photos p ON p.category_id = c.id
GROUP BY c.id, c.slug, c.name_i18n, c.sort_order
ORDER BY c.sort_order ASC, c.id ASC
"#;

    CategoryWithCount::find_by_statement(Statement::from_string(db.get_database_backend(), sql))
        .all(db)
        .await
}

pub async fn count(db: &DatabaseConnection) -> Result<i64, DbErr> {
    categories::Entity::find().count(db).await.map(|n| n as i64)
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> Result<Option<categories::Model>, DbErr> {
    categories::Entity::find_by_id(id).one(db).await
}

pub async fn find_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<categories::Model>, DbErr> {
    categories::Entity::find()
        .filter(categories::Column::Slug.eq(slug))
        .one(db)
        .await
}

pub async fn insert(
    db: &DatabaseConnection,
    slug: String,
    name_zh: String,
    name_en: String,
    sort_order: i32,
) -> Result<categories::Model, DbErr> {
    categories::ActiveModel {
        slug: Set(slug),
        name_i18n: Set(json!({ "zh": name_zh, "en": name_en })),
        sort_order: Set(sort_order),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn update(
    db: &DatabaseConnection,
    id: i64,
    slug: Option<String>,
    name_zh: Option<String>,
    name_en: Option<String>,
    sort_order: Option<i32>,
) -> Result<categories::Model, DbErr> {
    let existing = categories::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("category".into()))?;
    let current_zh = i18n_value(&existing.name_i18n, "zh");
    let current_en = i18n_value(&existing.name_i18n, "en");
    let mut model: categories::ActiveModel = existing.into();
    if let Some(slug) = slug {
        model.slug = Set(slug);
    }
    if name_zh.is_some() || name_en.is_some() {
        let next_zh = match name_zh {
            Some(value) => value,
            None => current_zh,
        };
        let next_en = match name_en {
            Some(value) => value,
            None => current_en,
        };
        model.name_i18n = Set(json!({ "zh": next_zh, "en": next_en }));
    }
    if let Some(sort_order) = sort_order {
        model.sort_order = Set(sort_order);
    }
    model.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    let result = categories::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(DbErr::RecordNotFound(format!("category {id}")));
    }
    Ok(())
}

pub async fn count_photos_in(db: &DatabaseConnection, id: i64) -> Result<i64, DbErr> {
    photos::Entity::find()
        .filter(photos::Column::CategoryId.eq(id))
        .count(db)
        .await
        .map(|n| n as i64)
}

fn i18n_value(value: &Value, key: &str) -> String {
    match value.get(key).and_then(Value::as_str) {
        Some(text) => text.to_owned(),
        None => String::new(),
    }
}
