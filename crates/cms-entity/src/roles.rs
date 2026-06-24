//! `SeaORM` Entity for `roles`（RBAC 角色；tenant_id 为 NULL 表示系统内置角色）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "roles")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,
    pub tenant_id: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub slug: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub name_i18n: Json,
    pub is_system: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
