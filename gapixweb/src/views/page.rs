use rinja_axum::Template;

#[derive(Template)]
#[template(path = "page.html")]
pub struct PageTemplate<'a> {
    pub name: &'a str,
}

