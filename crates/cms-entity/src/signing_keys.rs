//! `SeaORM` Entity for `signing_keys`（JWT 非对称签名密钥环；公钥对外 JWKS）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "signing_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub kid: String,
    #[sea_orm(column_type = "Text")]
    pub algorithm: String,
    #[sea_orm(column_type = "Text")]
    pub private_pem: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub public_jwk: Json,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub expires_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
