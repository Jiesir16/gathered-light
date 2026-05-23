use std::collections::HashMap;

use cms_entity::{
    media_assets::{ActiveModel, Column, Entity, Model},
    media_variants,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set, Statement, TransactionTrait,
};

use crate::dto::dashboard_dto::MediaStats;

#[derive(Debug, FromQueryResult)]
struct StatusCountRow {
    status: String,
    count: i64,
}

pub struct NewAsset {
    pub storage_key: String,
    pub mime_type: String,
    pub byte_size: i64,
}

pub async fn insert_ready(db: &DatabaseConnection, asset: NewAsset) -> Result<Model, DbErr> {
    insert_with_status(db, asset, "ready").await
}

pub async fn insert_with_status(
    db: &DatabaseConnection,
    asset: NewAsset,
    status: &str,
) -> Result<Model, DbErr> {
    ActiveModel {
        storage_key: Set(asset.storage_key),
        mime_type: Set(asset.mime_type),
        byte_size: Set(Some(asset.byte_size)),
        status: Set(status.to_owned()),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn claim_pending(db: &DatabaseConnection, n: u64) -> Result<Vec<Model>, DbErr> {
    let txn = db.begin().await?;
    let candidates: Vec<Model> = Entity::find()
        .filter(Column::Status.eq("pending"))
        .order_by_asc(Column::Id)
        .limit(n)
        .all(&txn)
        .await?;

    for model in &candidates {
        let mut active: ActiveModel = model.clone().into();
        active.status = Set("processing".to_owned());
        active.update(&txn).await?;
    }
    txn.commit().await?;

    Ok(candidates)
}

pub async fn update_status(db: &DatabaseConnection, id: i64, status: &str) -> Result<(), DbErr> {
    let model = Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("media_asset".to_owned()))?;
    let mut active: ActiveModel = model.into();
    active.status = Set(status.to_owned());
    active.update(db).await?;
    Ok(())
}

pub async fn insert_variant(
    db: &DatabaseConnection,
    asset_id: i64,
    variant: &str,
    storage_key: &str,
    width: i32,
    height: i32,
) -> Result<(), DbErr> {
    use cms_entity::media_variants::ActiveModel as VariantActiveModel;

    VariantActiveModel {
        asset_id: Set(asset_id),
        variant: Set(variant.to_owned()),
        storage_key: Set(storage_key.to_owned()),
        width: Set(width),
        height: Set(height),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

pub async fn find_variant_by_key(
    db: &DatabaseConnection,
    storage_key: &str,
) -> Result<Option<cms_entity::media_variants::Model>, DbErr> {
    use cms_entity::media_variants::{Column as VariantColumn, Entity as VariantEntity};

    VariantEntity::find()
        .filter(VariantColumn::StorageKey.eq(storage_key))
        .one(db)
        .await
}

pub async fn find_by_storage_key<C>(db: &C, key: &str) -> Result<Option<Model>, DbErr>
where
    C: ConnectionTrait,
{
    Entity::find()
        .filter(Column::StorageKey.eq(key))
        .one(db)
        .await
}

pub async fn find_variants_for_assets(
    db: &DatabaseConnection,
    asset_ids: &[i64],
) -> Result<HashMap<i64, Vec<media_variants::Model>>, DbErr> {
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = media_variants::Entity::find()
        .filter(media_variants::Column::AssetId.is_in(asset_ids.to_vec()))
        .all(db)
        .await?;
    let mut by_asset: HashMap<i64, Vec<media_variants::Model>> = HashMap::new();
    for variant in rows {
        by_asset.entry(variant.asset_id).or_default().push(variant);
    }
    Ok(by_asset)
}

pub async fn count_by_status(db: &DatabaseConnection) -> Result<MediaStats, DbErr> {
    // SELECT status, count(*) FROM media_assets GROUP BY status
    let sql = r#"
SELECT status, COUNT(*)::BIGINT AS count
FROM media_assets
GROUP BY status
"#;
    let rows =
        StatusCountRow::find_by_statement(Statement::from_string(db.get_database_backend(), sql))
            .all(db)
            .await?;

    let mut stats = MediaStats::default();
    for row in rows {
        match row.status.as_str() {
            "pending" => stats.pending = row.count,
            "processing" => stats.processing = row.count,
            "ready" => stats.ready = row.count,
            "failed" => stats.failed = row.count,
            _ => {}
        }
    }
    stats.total = stats.pending + stats.processing + stats.ready + stats.failed;
    Ok(stats)
}
