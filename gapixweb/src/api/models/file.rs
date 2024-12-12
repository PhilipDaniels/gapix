
pub struct FileRequest {
    pub filename: String,
    pub data: Vec<u8>
}

pub struct FileResponse {
    pub id: i32,
    pub name: String,
    pub hash: String,
}

