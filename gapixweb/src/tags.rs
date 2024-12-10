use maud::{html, Markup};


pub fn tag_list() -> Markup {
    html! {
        ul {
            li { "200" }
            li { "300" }
            li { "DIY" }
        }
    }
}
