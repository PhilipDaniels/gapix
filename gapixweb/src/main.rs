use std::{thread, time::Duration};

use asset::static_handler;
use axum::{
    routing::get,
    Router,
};
use database::database_path;
use index::index;

mod asset;
mod database;
mod index;
mod tags;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let db_path = database_path()?;
    println!("db_path = {db_path:?}");

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
    println!("Listening on {url}");
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
    tracing_subscriber::fmt::init();
}
