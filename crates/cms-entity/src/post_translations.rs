//! `SeaORM` Entity for `post_translations`（i18n 内容；body_json=TipTap 真源，body_html=清洗后渲染）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "post_translations")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i64,
    pub post_id: i64,
    pub tenant_id: i64,
    #[sea_orm(column_type = "Text")]
    pub locale: String,
    #[sea_orm(column_type = "Text")]
    pub title: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub excerpt: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub body_json: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub body_html: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub location: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub caption: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub alt_text: Option<String>,
    pub word_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
