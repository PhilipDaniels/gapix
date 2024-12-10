use std::{thread, time::Duration};

use asset::Asset;
use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use maud::{html, Markup, DOCTYPE};
use tags::tag_list;

mod asset;
mod tags;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let app = Router::new()
        .route("/", get(root))
        .route("/assets/*file", get(static_handler))
        .fallback_service(get(not_found));

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

async fn root() -> Markup {
    index_view("GaPiX Web", tag_list())
}

async fn not_found() -> Html<&'static str> {
    Html("<h1>404</h1><p>Not Found</p>")
}

fn configure_tracing() {
    tracing_subscriber::fmt::init();
}

pub fn index_view(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html class="no-js" lang="en" {
            head {
                meta charset="utf-8";
                title { (title) }
                script src="assets/htmx.2.0.0.min.js" {}
                script src="assets/tailwindcss.3.4.16.js" {}
            }
            body {
                (content)
            }
        }
    }
}

// We use a wildcard matcher ("/dist/*file") to match against everything
// within our defined assets directory. This is the directory on our Asset
// struct below, where folder = "examples/public/".
async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("assets/") {
        path = path.replace("assets/", "");
    }

    StaticFile(path)
}

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
  T: Into<String>,
{
  fn into_response(self) -> Response {
    let path = self.0.into();

    match Asset::get(path.as_str()) {
      Some(content) => {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
      }
      None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
  }
}