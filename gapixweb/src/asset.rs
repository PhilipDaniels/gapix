use rust_embed::Embed;

#[derive(Embed)]
#[folder = "assets"]
#[include = "*.js"]
pub struct Asset;
