use std::path::Path;

use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::info;

pub mod entity;
pub mod migration;

/// Initialises the database. Call this before doing any other database
/// operations. Checks that the db can be opened and applies any pending
/// migrations.
pub async fn initialise_database<P: AsRef<Path>>(db_path: P) -> Result<ConnectionFactory> {
    let db_path = db_path.as_ref();

    // Need to have the directory created before we can open or create a file
    // there.
    let parent = db_path.parent().unwrap();
    if let Err(err) = std::fs::create_dir_all(parent) {
        panic!("Could not create database parent directory {parent:?}; {err:?}");
    }

    let factory = ConnectionFactory::new(db_path);

    // Apply all pending migrations.
    let db = factory.make_db_connection().await?;
    assert!(db.ping().await.is_ok());
    Migrator::up(&db, None).await?;

    Ok(factory)
}

/// Structure that can be used to create new database connections.
#[derive(Clone)]
pub struct ConnectionFactory {
    conn_str: String,
}

impl ConnectionFactory {
    fn new(db_path: &Path) -> Self {
        // We need to convert the database Path into a string in order to be able to
        // format it without wrapping it in quotes. This will break if people use
        // non-UTF-8 paths, but I am willing to live with that.
        let conn_str = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());
        Self { conn_str }
    }

    /// Creates a DatabaseConnection object based on the connection string that
    /// was determined when the factory was created.
    pub async fn make_db_connection(&self) -> Result<DatabaseConnection> {
        info!("make_connection(), conn_str={}", self.conn_str);
        let mut opt = ConnectOptions::new(self.conn_str.to_owned());
        opt.sqlx_logging(false); // Disable SQLx log
        let db = Database::connect(opt).await?;
        Ok(db)
    }
}
