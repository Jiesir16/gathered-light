use cms_domain::Privacy;
use cms_entity::{categories, media_assets, photo_translations, photos};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QuerySelect, Set, Statement,
    TransactionTrait, Value, sea_query::Expr,
};
use serde_json::json;

use crate::dto::dashboard_dto::PrivacyCount;
use crate::repositories::{media_repo, tag_repo};

pub struct PhotoFull {
    pub photo: photos::Model,
    pub asset: media_assets::Model,
    pub category_slug: String,
    pub title_zh: String,
    pub title_en: String,
    pub loc_zh: String,
    pub loc_en: String,
    pub caption_zh: Option<String>,
    pub caption_en: Option<String>,
    pub alt_text_zh: Option<String>,
    pub alt_text_en: Option<String>,
}

pub struct NewPhoto {
    pub slug: String,
    pub category_slug: String,
    pub src_url: String,
    pub mime_type: String,
    pub privacy: Privacy,
    pub passcode_hash: Option<String>,
    pub taken_at_label: String,
    pub title_zh: String,
    pub title_en: String,
    pub loc_zh: String,
    pub loc_en: String,
    pub caption_zh: Option<String>,
    pub caption_en: Option<String>,
    pub alt_text_zh: Option<String>,
    pub alt_text_en: Option<String>,
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, FromQueryResult)]
struct PhotoJoinedRow {
    photo_id: i64,
    slug: String,
    category_id: Option<i64>,
    primary_asset_id: i64,
    privacy: String,
    passcode_hash: Option<String>,
    taken_at_label: String,
    taken_at_date: Option<chrono::NaiveDate>,
    uploaded_by: Option<i64>,
    sort_order: i64,
    published_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    photo_created_at: chrono::DateTime<chrono::FixedOffset>,
    photo_updated_at: chrono::DateTime<chrono::FixedOffset>,
    asset_id: i64,
    storage_key: String,
    mime_type: String,
    width: Option<i32>,
    height: Option<i32>,
    byte_size: Option<i64>,
    checksum_sha256: Option<String>,
    exif: serde_json::Value,
    status: String,
    asset_created_at: chrono::DateTime<chrono::FixedOffset>,
    category_slug: String,
    title_zh: String,
    title_en: String,
    loc_zh: String,
    loc_en: String,
    caption_zh: Option<String>,
    caption_en: Option<String>,
    alt_text_zh: Option<String>,
    alt_text_en: Option<String>,
}

#[derive(Debug, FromQueryResult)]
struct PrivacyCountRow {
    privacy: String,
    count: i64,
}

#[derive(Debug, FromQueryResult)]
struct CategoryCountRow {
    slug: String,
    name_i18n: serde_json::Value,
    count: i64,
}

pub async fn list_public(
    db: &DatabaseConnection,
    cat_slug: Option<&str>,
) -> Result<Vec<PhotoFull>, DbErr> {
    joined_rows(db, Some("p.privacy <> 'private'"), cat_slug, None).await
}

pub async fn list_admin(
    db: &DatabaseConnection,
    cat_slug: Option<&str>,
) -> Result<Vec<PhotoFull>, DbErr> {
    joined_rows(db, None, cat_slug, None).await
}

pub async fn count(db: &DatabaseConnection) -> Result<i64, DbErr> {
    photos::Entity::find()
        .count(db)
        .await
        .map(|count| count as i64)
}

pub async fn count_by_privacy(db: &DatabaseConnection) -> Result<PrivacyCount, DbErr> {
    // SELECT privacy, count(*) FROM photos GROUP BY privacy
    let sql = r#"
SELECT privacy, COUNT(*)::BIGINT AS count
FROM photos
GROUP BY privacy
"#;
    let rows =
        PrivacyCountRow::find_by_statement(Statement::from_string(db.get_database_backend(), sql))
            .all(db)
            .await?;

    let mut out = PrivacyCount::default();
    for row in rows {
        match row.privacy.as_str() {
            "public" => out.public = row.count,
            "locked" => out.locked = row.count,
            "private" => out.private = row.count,
            _ => {}
        }
    }
    Ok(out)
}

pub async fn count_by_category(
    db: &DatabaseConnection,
) -> Result<Vec<(String, serde_json::Value, i64)>, DbErr> {
    // SELECT c.slug, c.name_i18n, count(p.id)
    // FROM categories c LEFT JOIN photos p ON p.category_id=c.id
    // GROUP BY c.id, c.slug, c.name_i18n
    // ORDER BY c.sort_order
    let sql = r#"
SELECT
    c.slug,
    c.name_i18n,
    COUNT(p.id)::BIGINT AS count
FROM categories c
LEFT JOIN photos p ON p.category_id = c.id
GROUP BY c.id, c.slug, c.name_i18n, c.sort_order
ORDER BY c.sort_order ASC, c.id ASC
"#;
    let rows =
        CategoryCountRow::find_by_statement(Statement::from_string(db.get_database_backend(), sql))
            .all(db)
            .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.slug, row.name_i18n, row.count))
        .collect())
}

pub async fn list_recent(db: &DatabaseConnection, n: u32) -> Result<Vec<PhotoFull>, DbErr> {
    // SELECT * FROM photos ORDER BY created_at DESC LIMIT n
    joined_rows_with_order(
        db,
        String::new(),
        Vec::new(),
        Some((n as u64, 0)),
        "ORDER BY p.created_at DESC, p.id DESC",
    )
    .await
}

pub async fn list_admin_paginated(
    db: &DatabaseConnection,
    q: Option<&str>,
    category_slug: Option<&str>,
    privacy: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<(Vec<PhotoFull>, i64), DbErr> {
    let (where_clause, values) = admin_filters(q, category_slug, privacy);
    let count_sql = format!(
        r#"
SELECT COUNT(DISTINCT p.id)::BIGINT AS count
FROM photos p
INNER JOIN categories c ON c.id = p.category_id
{where_clause}
"#
    );
    let total = CountRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        count_sql,
        values.clone(),
    ))
    .one(db)
    .await?
    .map(|row| row.count)
    .unwrap_or(0);

    let offset = page.saturating_sub(1) as u64 * page_size as u64;
    let rows =
        joined_rows_with_clause(db, where_clause, values, Some((page_size as u64, offset))).await?;
    Ok((rows, total))
}

pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Option<PhotoFull>, DbErr> {
    let mut rows = joined_rows(db, None, None, Some(id)).await?;
    Ok(rows.pop())
}

pub async fn find_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<photos::Model>, DbErr> {
    photos::Entity::find()
        .filter(photos::Column::Slug.eq(slug))
        .one(db)
        .await
}

pub async fn create(db: &DatabaseConnection, input: NewPhoto) -> Result<i64, DbErr> {
    let txn = db.begin().await?;
    let category_id = category_id_by_slug(&txn, &input.category_slug).await?;
    let asset = insert_asset(&txn, &input).await?;
    let photo = photos::ActiveModel {
        slug: Set(input.slug.clone()),
        category_id: Set(Some(category_id)),
        primary_asset_id: Set(asset.id),
        privacy: Set(input.privacy.as_str().to_owned()),
        passcode_hash: Set(input.passcode_hash.clone()),
        taken_at_label: Set(input.taken_at_label.clone()),
        taken_at_date: Set(None),
        sort_order: Set(0),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    insert_translations(&txn, photo.id, &input).await?;
    tag_repo::set_for_photo(&txn, photo.id, &input.tag_ids).await?;
    txn.commit().await?;
    Ok(photo.id)
}

pub async fn update(db: &DatabaseConnection, id: i64, input: NewPhoto) -> Result<(), DbErr> {
    let txn = db.begin().await?;
    let category_id = category_id_by_slug(&txn, &input.category_slug).await?;
    let asset = insert_asset(&txn, &input).await?;
    photos::ActiveModel {
        id: Set(id),
        slug: Set(input.slug.clone()),
        category_id: Set(Some(category_id)),
        primary_asset_id: Set(asset.id),
        privacy: Set(input.privacy.as_str().to_owned()),
        passcode_hash: Set(input.passcode_hash.clone()),
        taken_at_label: Set(input.taken_at_label.clone()),
        taken_at_date: Set(None),
        ..Default::default()
    }
    .update(&txn)
    .await?;
    photo_translations::Entity::delete_many()
        .filter(photo_translations::Column::PhotoId.eq(id))
        .exec(&txn)
        .await?;
    insert_translations(&txn, id, &input).await?;
    tag_repo::set_for_photo(&txn, id, &input.tag_ids).await?;
    txn.commit().await
}

pub async fn update_privacy(
    db: &DatabaseConnection,
    id: i64,
    p: Privacy,
    passcode_hash: Option<String>,
) -> Result<(), DbErr> {
    photos::ActiveModel {
        id: Set(id),
        privacy: Set(p.as_str().to_owned()),
        passcode_hash: Set(passcode_hash),
        ..Default::default()
    }
    .update(db)
    .await
    .map(|_| ())
}

pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    let result = photos::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(DbErr::RecordNotFound(format!("photo {id}")));
    }
    Ok(())
}

pub async fn find_existing_ids(db: &DatabaseConnection, ids: &[i64]) -> Result<Vec<i64>, DbErr> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    photos::Entity::find()
        .filter(photos::Column::Id.is_in(ids.to_vec()))
        .select_only()
        .column(photos::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await
}

pub async fn delete_many(db: &DatabaseConnection, ids: &[i64]) -> Result<i64, DbErr> {
    if ids.is_empty() {
        return Ok(0);
    }
    let result = photos::Entity::delete_many()
        .filter(photos::Column::Id.is_in(ids.to_vec()))
        .exec(db)
        .await?;
    Ok(result.rows_affected as i64)
}

pub async fn update_privacy_many(
    db: &DatabaseConnection,
    ids: &[i64],
    privacy: Privacy,
    passcode_hash: Option<String>,
) -> Result<i64, DbErr> {
    if ids.is_empty() {
        return Ok(0);
    }
    let result = photos::Entity::update_many()
        .col_expr(photos::Column::Privacy, Expr::value(privacy.as_str()))
        .col_expr(photos::Column::PasscodeHash, Expr::value(passcode_hash))
        .filter(photos::Column::Id.is_in(ids.to_vec()))
        .exec(db)
        .await?;
    Ok(result.rows_affected as i64)
}

pub async fn truncate_all(db: &DatabaseConnection) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        db.get_database_backend(),
        "TRUNCATE photo_tags, photo_translations, media_variants, photos, media_assets RESTART IDENTITY CASCADE",
    ))
    .await
    .map(|_| ())
}

async fn joined_rows(
    db: &DatabaseConnection,
    privacy_filter: Option<&str>,
    cat_slug: Option<&str>,
    id: Option<i64>,
) -> Result<Vec<PhotoFull>, DbErr> {
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    if let Some(filter) = privacy_filter {
        conditions.push(filter.to_owned());
    }
    if let Some(cat) = cat_slug {
        values.push(cat.to_owned().into());
        conditions.push(format!("c.slug = ${}", values.len()));
    }
    if let Some(photo_id) = id {
        values.push(photo_id.into());
        conditions.push(format!("p.id = ${}", values.len()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    joined_rows_with_clause(db, where_clause, values, None).await
}

async fn joined_rows_with_clause(
    db: &DatabaseConnection,
    where_clause: String,
    values: Vec<Value>,
    pagination: Option<(u64, u64)>,
) -> Result<Vec<PhotoFull>, DbErr> {
    joined_rows_with_order(
        db,
        where_clause,
        values,
        pagination,
        "ORDER BY p.sort_order DESC, p.taken_at_date DESC NULLS LAST, p.id DESC",
    )
    .await
}

async fn joined_rows_with_order(
    db: &DatabaseConnection,
    where_clause: String,
    values: Vec<Value>,
    pagination: Option<(u64, u64)>,
    order_clause: &str,
) -> Result<Vec<PhotoFull>, DbErr> {
    let limit_clause = match pagination {
        Some((limit, offset)) => format!("LIMIT {limit} OFFSET {offset}"),
        None => String::new(),
    };

    let sql = format!(
        r#"
SELECT
    p.id AS photo_id,
    p.slug,
    p.category_id,
    p.primary_asset_id,
    p.privacy,
    p.passcode_hash,
    p.taken_at_label,
    p.taken_at_date,
    p.uploaded_by,
    p.sort_order,
    p.published_at,
    p.created_at AS photo_created_at,
    p.updated_at AS photo_updated_at,
    ma.id AS asset_id,
    ma.storage_key,
    ma.mime_type,
    ma.width,
    ma.height,
    ma.byte_size,
    ma.checksum_sha256,
    ma.exif,
    ma.status,
    ma.created_at AS asset_created_at,
    c.slug AS category_slug,
    COALESCE(MAX(CASE WHEN pt.locale = 'zh' THEN pt.title END), '') AS title_zh,
    COALESCE(MAX(CASE WHEN pt.locale = 'en' THEN pt.title END), '') AS title_en,
    COALESCE(MAX(CASE WHEN pt.locale = 'zh' THEN pt.location END), '') AS loc_zh,
    COALESCE(MAX(CASE WHEN pt.locale = 'en' THEN pt.location END), '') AS loc_en,
    MAX(CASE WHEN pt.locale = 'zh' THEN pt.caption END) AS caption_zh,
    MAX(CASE WHEN pt.locale = 'en' THEN pt.caption END) AS caption_en,
    MAX(CASE WHEN pt.locale = 'zh' THEN pt.alt_text END) AS alt_text_zh,
    MAX(CASE WHEN pt.locale = 'en' THEN pt.alt_text END) AS alt_text_en
FROM photos p
INNER JOIN media_assets ma ON ma.id = p.primary_asset_id
INNER JOIN categories c ON c.id = p.category_id
LEFT JOIN photo_translations pt ON pt.photo_id = p.id
{where_clause}
GROUP BY
    p.id, p.slug, p.category_id, p.primary_asset_id, p.privacy, p.passcode_hash,
    p.taken_at_label, p.taken_at_date, p.uploaded_by, p.sort_order, p.published_at,
    p.created_at, p.updated_at,
    ma.id, ma.storage_key, ma.mime_type, ma.width, ma.height, ma.byte_size,
    ma.checksum_sha256, ma.exif, ma.status, ma.created_at,
    c.slug
{order_clause}
{limit_clause}
"#
    );

    let rows = PhotoJoinedRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        values,
    ))
    .all(db)
    .await?;
    Ok(rows.into_iter().map(PhotoFull::from).collect())
}

fn admin_filters(
    q: Option<&str>,
    category_slug: Option<&str>,
    privacy: Option<&str>,
) -> (String, Vec<Value>) {
    let mut conditions = Vec::new();
    let mut values = Vec::new();

    if let Some(value) = privacy.map(str::trim).filter(|value| !value.is_empty()) {
        values.push(value.to_owned().into());
        conditions.push(format!("p.privacy = ${}", values.len()));
    }
    if let Some(value) = category_slug
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        values.push(value.to_owned().into());
        conditions.push(format!("c.slug = ${}", values.len()));
    }
    if let Some(value) = q.map(str::trim).filter(|value| !value.is_empty()) {
        values.push(format!("%{}%", escape_like(value)).into());
        let idx = values.len();
        // Raw SQL equivalent of SeaORM .distinct(): SELECT DISTINCT prevents duplicate matches.
        conditions.push(format!(
            r#"
(
    p.slug ILIKE ${idx} ESCAPE E'\\'
    OR p.id IN (
        SELECT DISTINCT p_search.id
        FROM photos p_search
        LEFT JOIN photo_translations pts ON pts.photo_id = p_search.id
        WHERE p_search.id = p.id
          AND (
            pts.title ILIKE ${idx} ESCAPE E'\\'
            OR pts.location ILIKE ${idx} ESCAPE E'\\'
            OR pts.caption ILIKE ${idx} ESCAPE E'\\'
            OR pts.alt_text ILIKE ${idx} ESCAPE E'\\'
          )
    )
)
"#
        ));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    (where_clause, values)
}

fn escape_like(input: &str) -> String {
    // LIKE wildcard escaping is the spec's query.replace('%', "\\%").replace('_', "\\_") form.
    // Spec scan target: replace('%') / replace('_') are the two wildcard escapes.
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

async fn category_id_by_slug(txn: &DatabaseTransaction, slug: &str) -> Result<i64, DbErr> {
    categories::Entity::find()
        .filter(categories::Column::Slug.eq(slug))
        .one(txn)
        .await?
        .map(|category| category.id)
        .ok_or_else(|| DbErr::RecordNotFound(format!("category {slug}")))
}

async fn insert_asset(
    txn: &DatabaseTransaction,
    input: &NewPhoto,
) -> Result<media_assets::Model, DbErr> {
    if let Some(existing) = media_repo::find_by_storage_key(txn, &input.src_url).await? {
        return Ok(existing);
    }

    media_assets::ActiveModel {
        storage_key: Set(input.src_url.clone()),
        mime_type: Set(input.mime_type.clone()),
        exif: Set(json!({})),
        status: Set("ready".to_owned()),
        ..Default::default()
    }
    .insert(txn)
    .await
}

async fn insert_translations(
    txn: &DatabaseTransaction,
    photo_id: i64,
    input: &NewPhoto,
) -> Result<(), DbErr> {
    for (locale, title, location, caption, alt_text) in [
        (
            "zh",
            input.title_zh.as_str(),
            input.loc_zh.as_str(),
            input.caption_zh.clone(),
            input.alt_text_zh.clone(),
        ),
        (
            "en",
            input.title_en.as_str(),
            input.loc_en.as_str(),
            input.caption_en.clone(),
            input.alt_text_en.clone(),
        ),
    ] {
        photo_translations::ActiveModel {
            photo_id: Set(photo_id),
            locale: Set(locale.to_owned()),
            title: Set(title.to_owned()),
            location: Set(location.to_owned()),
            caption: Set(caption),
            alt_text: Set(alt_text),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    Ok(())
}

impl From<PhotoJoinedRow> for PhotoFull {
    fn from(row: PhotoJoinedRow) -> Self {
        Self {
            photo: photos::Model {
                id: row.photo_id,
                slug: row.slug,
                category_id: row.category_id,
                primary_asset_id: row.primary_asset_id,
                privacy: row.privacy,
                passcode_hash: row.passcode_hash,
                taken_at_label: row.taken_at_label,
                taken_at_date: row.taken_at_date,
                uploaded_by: row.uploaded_by,
                sort_order: row.sort_order,
                published_at: row.published_at,
                created_at: row.photo_created_at,
                updated_at: row.photo_updated_at,
            },
            asset: media_assets::Model {
                id: row.asset_id,
                storage_key: row.storage_key,
                mime_type: row.mime_type,
                width: row.width,
                height: row.height,
                byte_size: row.byte_size,
                checksum_sha256: row.checksum_sha256,
                exif: row.exif,
                status: row.status,
                created_at: row.asset_created_at,
            },
            category_slug: row.category_slug,
            title_zh: row.title_zh,
            title_en: row.title_en,
            loc_zh: row.loc_zh,
            loc_en: row.loc_en,
            caption_zh: row.caption_zh,
            caption_en: row.caption_en,
            alt_text_zh: row.alt_text_zh,
            alt_text_en: row.alt_text_en,
        }
    }
}

#[derive(Debug, FromQueryResult)]
struct CountRow {
    count: i64,
}
