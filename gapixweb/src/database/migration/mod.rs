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

}


async fn create_table<E>(manager: &SchemaManager<'_>, entity: E) -> Result<(), DbErr>
where
    E: EntityTrait,
{
    let backend = manager.get_database_backend();
    let schema = Schema::new(backend);
    let mut table_create_statement = schema.create_table_from_entity(entity);
    let table_create_statement = table_create_statement.if_not_exists();
    let stmt = backend.build(table_create_statement);
    let conn = manager.get_connection();
    let _ = conn.execute(stmt).await?;

    Ok(())
}
