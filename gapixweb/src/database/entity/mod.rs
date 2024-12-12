use anyhow::Result;
use sea_orm::EntityTrait;

use super::conn::make_connection;

pub mod file;

use crate::database::entity::file as FF;

pub async fn get_file(id: i32) -> Result<FF::Model> {
    let conn = make_connection().await.unwrap();
    let file = FF::Entity::find_by_id(id).one(&conn).await.unwrap();
    let model = file.unwrap();
    Ok(model.into())
}