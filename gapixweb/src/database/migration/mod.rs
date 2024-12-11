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
