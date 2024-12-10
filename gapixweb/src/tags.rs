use maud::{html, Markup};


pub fn tag_list() -> Markup {
    html! {
        ul class="list-disc max-w-md mx-auto bg-slate-300" {
            li { "200" }
            li { "300" }
            li { "DIY" }
        }
    }
}
