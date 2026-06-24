//! M2+I1+I2：统一 Post 内容模型 + 身份中心 + 多租户(RLS)。
//! 详见 docs/CMS_REFACTOR_PROPOSAL.md 与 docs/USER_CENTER_AND_IAM_DESIGN.md。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "sql/m20260701_000004_unified_content_and_iam_up.sql"
            ))
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "sql/m20260701_000004_unified_content_and_iam_down.sql"
            ))
            .await
            .map(|_| ())
    }
}
