use maud::{html, Markup, DOCTYPE};

/// Takes 'content' and wraps the standard header, body and footer around it.
pub fn page(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html class="no-js" lang="en" {
            head {
                meta charset="utf-8";
                title { "GaPiX Web" }
                script src="assets/htmx.2.0.0.min.js" {}
                link href="assets/daisyui.4.12.22.full.min.css" rel="stylesheet" type="text/css" {}
                script src="assets/tailwindcss.3.4.16.js" {}
            }
            body {
                (content)
            }
        }
    }
}
