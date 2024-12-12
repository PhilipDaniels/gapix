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
}
