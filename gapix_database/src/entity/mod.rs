use anyhow::Result;
use sea_orm::{ConnectionTrait, EntityTrait};

pub mod file;

use crate::entity::file as FF;

pub async fn get_file<C: ConnectionTrait>(conn: C, id: i32) -> Result<FF::Model> {
    let file = FF::Entity::find_by_id(id).one(&conn).await.unwrap();
    let model = file.unwrap();
    Ok(model.into())
}
