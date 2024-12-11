use std::{thread, time::Duration};

use asset::static_handler;
use axum::{routing::get, Router};
use index::index;
use sea_orm::{ActiveValue, EntityTrait};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use database::{conn::make_connection, migration::{Migrator, MigratorTrait}, ActiveFile, File};

mod asset;
mod database;
mod index;
mod tags;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let conn = make_connection().await?;
    assert!(conn.ping().await.is_ok());
    // Apply all pending migrations.
    Migrator::up(&conn, None).await?;

    // TEST: Insert an entity.
    let f = ActiveFile {
        name: ActiveValue::Set("~/Cycling/ride.gpx".to_string()),
        hash: ActiveValue::Set("my hash".to_string()),
        data: ActiveValue::Set(vec![12u8,234u8,64u8,2u8]),
        ..Default::default()
    };
    let _ = File::insert(f).exec(&conn).await?;

    let app = Router::new()
        .route("/", get(index))
        .route("/assets/*file", get(static_handler));

    // Bind to a random port, then use a background thread to automatically open
    // the correct URL in the browser. We wait for a bit in the background
    // thread to ensure axum is started up (though this does not seem to really
    // be necessary on my machine.)
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("http://localhost:{}", addr.port());
    info!("Listening on {url}");
    thread::spawn(|| {
        thread::sleep(Duration::from_secs_f32(0.5));
        // Ignore any errors, this is a "nice-to-have" anyway.
        let _ = opener::open_browser(url);
    });

    // We block here. Closing the browser window does
    // not shut down the app.
    axum::serve(listener, app).await?;

    // This code only runs on exit.
    Ok(())
}

fn configure_tracing() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)   // Makes #[instrument] output something
        //.with_thread_ids(true)
        //.with_thread_names(true)
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .init();
}
