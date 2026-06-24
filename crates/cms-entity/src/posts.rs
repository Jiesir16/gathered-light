//! `SeaORM` Entity for `posts`（统一内容主体：post/page/photo，多租户）。
//! slug 唯一性是 (site_id, post_type, slug) 复合约束（见迁移），故此处 slug 列不单独标 unique。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,
    pub tenant_id: i64,
    pub site_id: i64,
    #[sea_orm(column_type = "Text")]
    pub post_type: String,
    #[sea_orm(column_type = "Text")]
    pub slug: String,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub visibility: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub passcode_hash: Option<String>,
    pub author_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub primary_asset_id: Option<i64>,
    pub featured_asset_id: Option<i64>,
    pub menu_order: i64,
    #[sea_orm(column_type = "Text")]
    pub taken_at_label: String,
    pub taken_at_date: Option<Date>,
    pub published_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
