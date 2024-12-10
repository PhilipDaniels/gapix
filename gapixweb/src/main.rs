use std::{thread, time::Duration};

use axum::{routing::get, Router};
use maud::{html, Markup, DOCTYPE};
use tags::tag_list;

mod tags;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let app = Router::new().route("/", get(root));

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
                script src="https://unpkg.com/htmx.org@1.9.4" integrity="sha384-zUfuhFKKZCbHTY6aRR46gxiqszMk5tcHjsVFxnUo8VMus4kHGVdIYVbOYYNlKmHV" crossorigin="anonymous" {}
                script src="https://cdn.tailwindcss.com" {}
            }
            body {
                (content)
            }
        }
    }
}