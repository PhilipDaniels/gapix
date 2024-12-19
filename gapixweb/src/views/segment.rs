use maud::{html, Markup};

use crate::components::{page::page, tabs::{tabs, Tabs}};

/// Returns the list of rides for the "Rides" tab.
pub async fn segments_view() -> Markup {
    let tab_content = html! { 
        p { "And here we got some segments" }
    };

    let html = tabs(Tabs::Segments, &tab_content);
    page(html)
}
