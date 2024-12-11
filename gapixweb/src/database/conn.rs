use std::path::PathBuf;

use anyhow::{bail, Result};
use directories::ProjectDirs;
use sea_orm::{Database, DatabaseConnection};
use tracing::info;

pub async fn make_connection() -> Result<DatabaseConnection> {
    let conn_str = conn_str()?;
    info!("conn_str={conn_str}");
    let db = Database::connect(&conn_str).await?;
    Ok(db)
}

/// We need to convert the database Path into a string in order to be
/// able to format it without wrapping it in quotes. This will break if
/// people use non-UTF-8 paths, but I am willing to live with that.
fn conn_str() -> Result<String> {
    let p = database_path()?;
    let p = p.to_string_lossy();
    let conn_str = format!("sqlite:{}?mode=rwc", p);
    Ok(conn_str)
}

/// Returns the path to the database. Differs in debug and release builds.
fn database_path() -> Result<PathBuf> {
    let mut pb = match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
        Some(dirs) => dirs.data_local_dir().to_path_buf(),
        None => bail!("Cannot determine path to database"),
    };

    // Need to have the directory created before we can open or create a file
    // there.
    std::fs::create_dir_all(&pb)?;

    #[cfg(debug_assertions)]
    pb.push("gapixweb-debug.db");

    #[cfg(not(debug_assertions))]
    pb.push("gapixweb.db");

    Ok(pb)
}
