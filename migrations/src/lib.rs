//! 数据库迁移（≈ Flyway / Liquibase 的角色）。
//!
//! 用法：
//! ```sh
//! export DATABASE_URL=postgres://postgres:postgres@localhost:5432/gathered_light
//! cargo run -p migrations -- up        # 全量正向迁移
//! cargo run -p migrations -- down      # 回滚最近一次
//! cargo run -p migrations -- fresh     # drop 所有表再 up（仅 dev）
//! ```

pub use sea_orm_migration::prelude::*;

mod m20260506_000001_init;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260506_000001_init::Migration)]
    }
}
