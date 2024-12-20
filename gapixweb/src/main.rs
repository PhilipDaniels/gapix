#![forbid(unsafe_code)]

use std::{path::PathBuf, thread, time::Duration};

use anyhow::Result;
use args::parse_args;
use asset::static_handler;
use axum::{routing::get, Router};
use components::tabs::Tabs;
use directories::ProjectDirs;
use gapix_database::{migration::sea_orm::DatabaseConnection, ConnectionFactory};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use views::{ride, segment};

mod api;
mod args;
mod asset;
mod components;
mod error;
mod views;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let args = parse_args();
    info!("Command line arguments: {args:?}");

    let db_path = get_full_database_path(&args.database);
    let connection_factory = gapix_database::initialise_database(db_path).await?;
    let state = AppState { connection_factory };

    // Setup routes.
    let app = Router::new()
        .route("/", get(ride::rides_view))
        .route("/assets/*file", get(static_handler))
        .route(Tabs::Rides.href(), get(ride::rides_view))
        .route(Tabs::Segments.href(), get(segment::segments_view))
        .route(Tabs::Controls.href(), get(ride::rides_view))
        .route(Tabs::Settings.href(), get(ride::rides_view))
        .route(Tabs::Jobs.href(), get(ride::rides_view))
        //.route("/rides/:id", get(ride::ride_view))
        .with_state(state);

    // If user did not specify a port, let the OS choose a random one.
    let url = if let Some(port) = args.port {
        &format!("localhost:{port}")
    } else {
        "localhost:0"
    };

    let listener = tokio::net::TcpListener::bind(url).await?;

    // Figure out which port was actually used.
    let addr = listener.local_addr()?;
    let url = format!("http://localhost:{}", addr.port());
    info!("Listening on {url}");

    // Use a background thread to automatically open the correct URL in the
    // browser. We wait for a bit in the background thread to ensure axum is
    // started up (though this does not seem to really be necessary on my
    // machine).
    if args.auto_open {
        thread::spawn(|| {
            thread::sleep(Duration::from_secs_f32(0.5));
            // Ignore any errors, this is a "nice-to-have" anyway.
            let _ = opener::open_browser(url);
        });
    } else {
        // If not auto-opening, user needs to be told where the site is.
        println!("Listening on {url}");
    }

    // We block here. Closing the browser window does
    // not shut down the app.
    axum::serve(listener, app).await?;

    // This code only runs on exit.
    Ok(())
}

fn configure_tracing() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE) // Makes #[instrument] output something
        //.with_thread_ids(true)
        //.with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();
}

/// Returns the full path to the databsase file.
fn get_full_database_path(db_name: &str) -> PathBuf {
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

    db_path
}

#[derive(Clone)]
struct AppState {
    connection_factory: ConnectionFactory,
}

impl AppState {
    /// Convenience function to make a database connection.
    pub(crate) async fn db(&self) -> Result<DatabaseConnection> {
        self.connection_factory.make_db_connection().await
    }
}
