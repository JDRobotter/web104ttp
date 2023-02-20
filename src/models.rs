use diesel::{Queryable, Insertable};
use rocket::serde::{Serialize, Deserialize};

use crate::schema::pictures;
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
pub struct Picture {
    pub id: i32,
    pub word: String,
}

use crate::schema::blobs;
#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
pub struct Blob {
    pub id: i32,
    pub side: i32,
    pub thumbnail: bool,
    pub data: Vec<u8>,
    pub mime: String,
    pub show: bool,
}


