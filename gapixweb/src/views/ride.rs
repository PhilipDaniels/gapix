use axum::extract::{Path, State};
use maud::{html, Markup};

use crate::{components::{page::page, tabs::{tabs, Tabs}}, error::ApiResult, AppState};

/// Returns the list of rides for the "Rides" tab.
pub async fn rides_view() -> Markup {
    let tab_content = html! { 
        p { "List of rides goes here" }
    };

    let html = tabs(Tabs::Rides, &tab_content);
    page(html)
}


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
