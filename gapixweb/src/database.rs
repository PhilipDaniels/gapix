use std::path::PathBuf;

use anyhow::bail;
use directories::ProjectDirs;

/// Returns the path to the database. Differs in debug and release builds.
pub fn database_path() -> anyhow::Result<PathBuf> {
    let mut pb = match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
        Some(dirs) => dirs.data_local_dir().to_path_buf(),
        None => bail!("Cannot determine path to database")
    };

    #[cfg(debug_assertions)]
    pb.push("gapixweb-debug.db");

    #[cfg(not(debug_assertions))]
    pb.push("gapixweb.db");

    Ok(pb)
}

