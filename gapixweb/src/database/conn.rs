use std::path::PathBuf;

use anyhow::{bail, Result};
use directories::ProjectDirs;
use sea_orm::{Database, DatabaseConnection};
use tracing::info;

use crate::DB_CONN_STR;

pub async fn make_connection() -> Result<DatabaseConnection> {
    let conn_str = &*DB_CONN_STR;
    info!("make_connection(), conn_str={conn_str}");
    let db = Database::connect(conn_str).await?;
    Ok(db)
}

/// Creates the connection string for SQLite based on the database name. Since
/// this goes into a global static, and we literally cannot do anything if this
/// doesn't work, we panic if there is a failure.
pub fn make_conn_str(db_name: &str) -> String {
    if db_name.is_empty() {
        panic!("Database name is empty");
    }

    let mut db_path: PathBuf = db_name.into();

    if db_path.is_relative() {
        let mut pb = match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(dirs) => dirs.data_local_dir().to_path_buf(),
            None => panic!("Cannot determine data_local_dir()"),
        };
        pb.push(db_name);
        db_path = pb;
    };

    // Need to have the directory created before we can open or create a file
    // there.
    let parent = db_path.parent().unwrap();
    if let Err(e) = std::fs::create_dir_all(parent) {
        panic!("Could not create database parent directory {parent:?}");
    }

    // We need to convert the database Path into a string in order to be able to
    // format it without wrapping it in quotes. This will break if people use
    // non-UTF-8 paths, but I am willing to live with that.
    let db_path = db_path.to_string_lossy();
    let conn_str = format!("sqlite:{}?mode=rwc", db_path);
    conn_str
}
