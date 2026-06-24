//! `SeaORM` Entity for `terms`（统一分类法的「词条」；slug 在 (site_id, slug) 维度唯一）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "terms")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,
    pub tenant_id: i64,
    pub site_id: i64,
    #[sea_orm(column_type = "Text")]
    pub slug: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub name_i18n: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
