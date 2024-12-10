use maud::{html, Markup, DOCTYPE};

use crate::tags::tag_list;

pub async fn index() -> Markup {
    index_view("GaPiX Web", tag_list())
}

fn index_view(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html class="no-js" lang="en" {
            head {
                meta charset="utf-8";
                title { (title) }
                script src="assets/htmx.2.0.0.min.js" {}
                script src="assets/tailwindcss.3.4.16.js" {}
            }
            body {
                (content)
            }
        }
    }
}

