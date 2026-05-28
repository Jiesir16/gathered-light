use cms_entity::site_settings;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set, sea_query::OnConflict,
};

pub async fn get(db: &DatabaseConnection, key: &str) -> Result<Option<String>, DbErr> {
    site_settings::Entity::find()
        .filter(site_settings::Column::Key.eq(key))
        .one(db)
        .await
        .map(|setting| setting.map(|model| model.value))
}

pub async fn set(db: &DatabaseConnection, key: &str, value: &str) -> Result<(), DbErr> {
    site_settings::Entity::insert(site_settings::ActiveModel {
        key: Set(key.to_owned()),
        value: Set(value.to_owned()),
        updated_at: Set(chrono::Utc::now().fixed_offset()),
    })
    .on_conflict(
        OnConflict::column(site_settings::Column::Key)
            .update_columns([site_settings::Column::Value, site_settings::Column::UpdatedAt])
            .to_owned(),
    )
    .exec_without_returning(db)
    .await?;
    Ok(())
}
