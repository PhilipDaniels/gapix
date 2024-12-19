/*
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
        let name = field.name().unwrap().to_string(); // "file"
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        let hash = Sha256::digest(&data);

        info!(
            "Length of `{name}` (`{file_name}`: `{content_type}`) is {} bytes, hash = {hash:?}",
            data.len()
        );

        // TODO: File length, date of insertion.
        // Unique index on hash. Check before upload.
        // Hash should be a binary blob? Maybe a newtype.

        // let f = entity::file::ActiveModel {
        //     name: ActiveValue::Set(file_name),
        //     hash: ActiveValue::Set(format!("{:x?}", hash)),
        //     data: ActiveValue::Set(data.to_vec()),
        //     ..Default::default()
        // };

        // let conn = state.connection_factory.make_db_connection().await.unwrap();
        // let res = f.insert(&conn).await.unwrap();
        // info!("Returned Id = {}", res.id);
    }

    //Ok(markup)
}
*/
