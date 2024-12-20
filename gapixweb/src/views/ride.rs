use axum::response::{IntoResponse, Response};

use super::page::PageTemplate;

/// Returns the list of rides for the "Rides" tab.
pub async fn rides_view() -> Response {
    let t = PageTemplate { name: "aaa" };
    t.into_response()
}


/*
/// Returns the markup for a single ride.
pub async fn ride_view(State(state): State<AppState>, Path(_id): Path<i32>) -> ApiResult<Markup> {
    let _conn = state.db().await.unwrap();

    // let file = entity::get_file(conn, id).await.unwrap();

    // let markup = html! {
    //     ul class="list-disc" {
    //         li { "Id:" (file.id) }
    //         li { "Name:" (file.name) }
    //         li { "Hash:" (file.hash) }
    //     }
    // };

    // Ok(markup)

    Ok(html!())
}
*/
