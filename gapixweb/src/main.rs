#![forbid(unsafe_code)]

use std::{thread, time::Duration};

use api::handlers::{get_file, post_files};
use args::parse_args;
use asset::static_handler;
use axum::{routing::{get, post}, Router};
use database::{
    conn::make_connection,
    migration::{Migrator, MigratorTrait},
};
use index::index;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

mod api;
mod args;
mod asset;
mod database;
mod error;
mod index;
mod tags;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let args = parse_args();
    info!("Command line arguments: {args:?}");

    // Apply all pending migrations.
    let conn = make_connection().await?;
    assert!(conn.ping().await.is_ok());
    Migrator::up(&conn, None).await?;

    // // TEST: Insert an entity.
    // let f = ActiveFile {
    //     name: ActiveValue::Set("~/Cycling/ride.gpx".to_string()),
    //     hash: ActiveValue::Set("my hash".to_string()),
    //     data: ActiveValue::Set(vec![12u8, 234u8, 64u8, 2u8]),
    //     ..Default::default()
    // };
    // let _ = File::insert(f).exec(&conn).await?;

    let app = Router::new()
        .route("/", get(index))
        .route("/assets/*file", get(static_handler))
        .route("/file/:id", get(get_file))
        .route("/file", post(post_files));

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
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .init();
}
