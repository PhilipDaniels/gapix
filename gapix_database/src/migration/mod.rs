use sea_orm::{EntityTrait, Schema};
pub use sea_orm_migration::prelude::*;

mod m20241211_01;
mod m20241211_02;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241211_01::Migration),
            Box::new(m20241211_02::Migration),
        ]
    }
}

trait SchemaManagerExtensions {
    /// Creates a table based on an entity definition. This only does something
    /// if the table does not already exist, so it can only be used for the
    /// first table create. It will create PKs, but not indexes.
    async fn create_table_for_entity<E>(&self, entity: E) -> Result<(), DbErr>
    where
        E: EntityTrait;

    /// Drops the table for the entity. If the table does not exist this is a
    /// no-op.
    async fn drop_table_for_entity<E>(&self, entity: E) -> Result<(), DbErr>
    where
        E: EntityTrait;
}

impl SchemaManagerExtensions for SchemaManager<'_> {
    async fn create_table_for_entity<E>(&self, entity: E) -> Result<(), DbErr>
    where
        E: EntityTrait,
    {
        let backend = self.get_database_backend();
        let schema = Schema::new(backend);
        let mut table_create_statement = schema.create_table_from_entity(entity);
        let table_create_statement = table_create_statement.if_not_exists();
        let stmt = backend.build(table_create_statement);
        let conn = self.get_connection();
        let _ = conn.execute(stmt).await?;

        Ok(())
    }

    async fn drop_table_for_entity<E>(&self, entity: E) -> Result<(), DbErr>
    where
        E: EntityTrait,
    {
        let mut drop = Table::drop();
        let stmt = drop.table(entity).if_exists();
        self.drop_table(stmt.to_owned()).await
    }
}
