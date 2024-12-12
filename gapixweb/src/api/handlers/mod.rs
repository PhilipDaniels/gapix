use axum::extract::Path;
use maud::{html, Markup};

use crate::error::ApiResult;

pub async fn get_file(Path(id): Path<i32>) -> ApiResult<Markup> {
    let markup = html! {
        p { "A file: " (id) }
    };

    Ok(markup)
}
