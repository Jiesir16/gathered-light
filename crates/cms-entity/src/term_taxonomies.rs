//! `SeaORM` Entity for `term_taxonomies`（词条在某分类法下的归属，支持层级）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "term_taxonomies")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,
    pub tenant_id: i64,
    pub term_id: i64,
    #[sea_orm(column_type = "Text")]
    pub taxonomy: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
