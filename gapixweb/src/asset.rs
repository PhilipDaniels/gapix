use axum::{
    http::{header, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

use crate::error::ApiError;

#[derive(Embed)]
#[folder = "assets"]
#[include = "*.js"]
pub struct Asset;

// We use a wildcard matcher ("/assets/*file") to match against everything
// within our defined assets directory. This is the directory on our Asset
// struct above, where folder = "assets".
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("assets/") {
        path = path.replace("assets/", "");
    }

    StaticFile(path)
}

struct StaticFile<T>(T);

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
            None => ApiError::not_found().into_response(),
        }
    }
}
