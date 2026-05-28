//! M2-1: photos.passcode_hash 改成 passcode（明文）。
//!
//! 业务理由见 sql/*_up.sql。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("sql/m20260528_000002_passcode_plain_up.sql"))
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("sql/m20260528_000002_passcode_plain_down.sql"))
            .await
            .map(|_| ())
    }
}
