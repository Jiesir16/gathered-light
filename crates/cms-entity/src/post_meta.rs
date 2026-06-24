//! `SeaORM` Entity for `post_meta`（非检索型扩展字段，EAV）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "post_meta")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,
    pub post_id: i64,
    pub tenant_id: i64,
    #[sea_orm(column_type = "Text")]
    pub meta_key: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub meta_value: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
