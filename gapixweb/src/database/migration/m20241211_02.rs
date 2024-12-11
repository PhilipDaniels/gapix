use sea_orm_migration::prelude::*;

use crate::database::File;

#[derive(DeriveMigrationName)]
pub struct Migration;

use super::SchemaManagerExtensions;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table_for_entity(File).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table_for_entity(File).await
    }
}
