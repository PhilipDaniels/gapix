use axum::response::{IntoResponse, Response};

use crate::components::tabs::{Tabs, TabsTemplate};

/// Returns the list of segments for the "Segments" tab.
pub async fn segments_view() -> Response {
    let t = TabsTemplate { active_tab: Tabs::Segments };
    t.into_response()
}
