use sea_orm::Schema;
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;

use crate::database::model::file::Entity as File;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        let backend = manager.get_database_backend();
        let schema = Schema::new(backend);
        // Doesn't work for defaults?
        let table_create_statement = schema.create_table_from_entity(File);
        let _ = conn.execute(backend.build(&table_create_statement)).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(File).to_owned())
            .await
    }
}
