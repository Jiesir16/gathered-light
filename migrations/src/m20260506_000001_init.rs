//! M1 初始化迁移：建表 + 索引（与 docs/ARCHITECTURE.md §2 一致）。
//!
//! SQL 直接放在 `sql/` 目录，便于 DBA review；Rust 这边只做包装。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("sql/m20260506_000001_init_up.sql"))
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("sql/m20260506_000001_init_down.sql"))
            .await
            .map(|_| ())
    }
}
