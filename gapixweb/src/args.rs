use clap::{arg, command, Parser};

pub fn parse_args() -> Args {
    Args::parse()
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        help = "Specify a port to serve the site on. If not specified, a random unused port is chosen."
    )]
    pub port: Option<u32>,

    #[arg(
        short,
        long,
        default_value = "false",
        help = "Whether to automatically open the website in the browser."
    )]
    pub auto_open: bool,

    #[arg(
        short,
        long,
        default_value = "gapixweb.db",
        help = "Filename of the database to open. Can be an absolute path or a filename such as 'gapixweb-debug.db'. \
        If just a filename, then the database will be located in your 'data_local_dir' as specified by \
        the 'directories' crate: https://crates.io/crates/directories. The database is in SQLite format."
    )]
    pub database: String
}
