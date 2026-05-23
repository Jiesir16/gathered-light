use std::collections::HashMap;

use cms_entity::{photo_tags, tags};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, Set, Statement,
    sea_query::OnConflict,
};
use serde_json::{Value, json};

#[derive(Debug, FromQueryResult)]
struct TagForPhotoRow {
    photo_id: i64,
    id: i64,
    slug: String,
    name_i18n: Value,
}

pub async fn list(
    db: &DatabaseConnection,
    page: u32,
    page_size: u32,
) -> Result<Vec<tags::Model>, DbErr> {
    let page = page.max(1);
    let page_size = page_size.max(1);
    tags::Entity::find()
        .order_by_asc(tags::Column::Id)
        .paginate(db, page_size as u64)
        .fetch_page((page - 1) as u64)
        .await
}

pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<tags::Model>, DbErr> {
    tags::Entity::find()
        .order_by_asc(tags::Column::Id)
        .all(db)
        .await
}

pub async fn count(db: &DatabaseConnection) -> Result<i64, DbErr> {
    tags::Entity::find().count(db).await.map(|n| n as i64)
}

pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<tags::Model>, DbErr> {
    tags::Entity::find_by_id(id).one(db).await
}

pub async fn find_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<tags::Model>, DbErr> {
    tags::Entity::find()
        .filter(tags::Column::Slug.eq(slug))
        .one(db)
        .await
}

pub async fn insert(
    db: &DatabaseConnection,
    slug: String,
    name_zh: String,
    name_en: String,
) -> Result<tags::Model, DbErr> {
    tags::ActiveModel {
        slug: Set(slug),
        name_i18n: Set(json!({ "zh": name_zh, "en": name_en })),
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
) -> Result<tags::Model, DbErr> {
    let existing = tags::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("tag".into()))?;
    let current_zh = i18n_value(&existing.name_i18n, "zh");
    let current_en = i18n_value(&existing.name_i18n, "en");
    let mut model: tags::ActiveModel = existing.into();
    if let Some(slug) = slug {
        model.slug = Set(slug);
    }
    if name_zh.is_some() || name_en.is_some() {
        model.name_i18n = Set(json!({
            "zh": name_zh.unwrap_or(current_zh),
            "en": name_en.unwrap_or(current_en),
        }));
    }
    model.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    let result = tags::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(DbErr::RecordNotFound(format!("tag {id}")));
    }
    Ok(())
}

pub async fn find_for_photos(
    db: &DatabaseConnection,
    photo_ids: &[i64],
) -> Result<HashMap<i64, Vec<tags::Model>>, DbErr> {
    if photo_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = (1..=photo_ids.len())
        .map(|idx| format!("${idx}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
SELECT
    pt.photo_id,
    t.id,
    t.slug,
    t.name_i18n
FROM tags t
INNER JOIN photo_tags pt ON pt.tag_id = t.id
WHERE pt.photo_id IN ({placeholders})
ORDER BY pt.photo_id ASC, t.id ASC
"#
    );
    let values = photo_ids
        .iter()
        .copied()
        .map(Into::into)
        .collect::<Vec<_>>();
    let rows = TagForPhotoRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        values,
    ))
    .all(db)
    .await?;

    let mut out: HashMap<i64, Vec<tags::Model>> = HashMap::new();
    for row in rows {
        out.entry(row.photo_id).or_default().push(tags::Model {
            id: row.id,
            slug: row.slug,
            name_i18n: row.name_i18n,
        });
    }
    Ok(out)
}

pub async fn set_for_photo<C: ConnectionTrait>(
    conn: &C,
    photo_id: i64,
    tag_ids: &[i64],
) -> Result<(), DbErr> {
    photo_tags::Entity::delete_many()
        .filter(photo_tags::Column::PhotoId.eq(photo_id))
        .exec(conn)
        .await?;

    let mut ids = tag_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Ok(());
    }

    let rows = ids.into_iter().map(|tag_id| photo_tags::ActiveModel {
        photo_id: Set(photo_id),
        tag_id: Set(tag_id),
    });
    photo_tags::Entity::insert_many(rows).exec(conn).await?;
    Ok(())
}

pub async fn assign_many<C: ConnectionTrait>(
    conn: &C,
    photo_ids: &[i64],
    tag_ids: &[i64],
    replace: bool,
) -> Result<(), DbErr> {
    if photo_ids.is_empty() {
        return Ok(());
    }
    if replace {
        photo_tags::Entity::delete_many()
            .filter(photo_tags::Column::PhotoId.is_in(photo_ids.to_vec()))
            .exec(conn)
            .await?;
    }
    if tag_ids.is_empty() {
        return Ok(());
    }

    let mut rows = Vec::with_capacity(photo_ids.len() * tag_ids.len());
    for photo_id in photo_ids {
        for tag_id in tag_ids {
            rows.push(photo_tags::ActiveModel {
                photo_id: Set(*photo_id),
                tag_id: Set(*tag_id),
            });
        }
    }
    photo_tags::Entity::insert_many(rows)
        .on_conflict(
            OnConflict::columns([photo_tags::Column::PhotoId, photo_tags::Column::TagId])
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
    Ok(())
}

fn i18n_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
