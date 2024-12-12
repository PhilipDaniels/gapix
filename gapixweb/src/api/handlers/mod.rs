use axum::extract::{Multipart, Path};
use maud::{html, Markup};
use tracing::info;

use crate::{database::entity, error::ApiResult};

pub async fn get_file(Path(id): Path<i32>) -> ApiResult<Markup> {
    let file = entity::get_file(id).await.unwrap();

    let markup = html! {
        ul class="list-disc" {
            li { "Id:" (file.id) }
            li { "Name:" (file.name) }
            li { "Hash:" (file.hash) }
        }
    };

    Ok(markup)
}

pub fn upload_file_view() -> Markup {
    html! {
        form action="/file" method="post" enctype="multipart/form-data" {
            label for="file" { "Upload file:" }
            input type="file" name="file" multiple {}
            input type="submit" value="Upload" {}
        }
    }
}

/// Note that this is a multipart post - each file that is uploaded is
/// once around the loop. It can also accept a single file.
pub async fn post_files(mut multipart: Multipart) {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        info!(
            "Length of `{name}` (`{file_name}`: `{content_type}`) is {} bytes",
            data.len()
        );
    }

    //Ok(markup)
}
