use axum::response::{IntoResponse, Response};
use rinja_axum::Template;

use crate::components::tabs::Tabs;

#[derive(Template)]
#[template(path = "rides.html")]
pub struct RidesTemplate {
    pub active_tab: Tabs
}

/// Returns the list of rides for the "Rides" tab.
pub async fn rides_view() -> Response {
    //let t = TabsTemplate { active_tab: Tabs::Rides };
    let t = RidesTemplate { active_tab: Tabs::Rides };
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
