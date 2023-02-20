use rocket::*;
use rocket_dyn_templates::{Template, context};
use rocket::fairing::AdHoc;
use anyhow::Result;

use std::sync::{Arc, Mutex};
use std::path::Path;

#[macro_use]
extern crate diesel;

mod words;
mod database;
mod schema;
mod models;
mod uploader;
mod image_process;

use rocket_sync_db_pools::database;

#[database("pictures")]
struct PicturesDbConn(diesel::SqliteConnection);

use crate::uploader::FileUploader;
pub struct AppState {

    uploader: Arc<Mutex<FileUploader>>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct PictureWithShows {
    id: i32,
    word: String,
    shows: Vec<bool>,
}

#[get("/")]
async fn index(conn: PicturesDbConn, state: &State<AppState>) -> Template {

    let (pictures, shows) = conn.run(move |c| { 
        let pictures = database::list_pictures(c);
        let shows = database::list_blobs_show(c).unwrap();
        (pictures,shows)
    } ).await;

    let mut pws = vec![];
    for pic in pictures {

        let mut ps = vec![false; 2];
        for (pic_id, pic_side, show) in &shows {
            if *pic_id == pic.id {
                ps[*pic_side as usize] = *show;
            }
        }

        pws.push(PictureWithShows {
            id: pic.id,
            word: pic.word,
            shows: ps,
        });
    }

    Template::render("index", context! {
        pictures: pws,
    })
}

#[get("/edit/<side>/<n>")]
async fn edit(conn: PicturesDbConn, side: u32, n: u32) -> Template {
    let show = conn.run(move |c| { 
        database::fetch_blob_show(c, side as i32, n as i32).unwrap()
    }).await;

    Template::render("edit", context! {
        side,
        n,
        show,
    })
}

use rocket::serde::{Deserialize, Serialize, json::Json};

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct UploadPrepare {
    side: u32,
    n: u32,
    size: usize,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct UploadStatus {
    uid: u32,
    ready: bool,
}

#[post("/upload/prepare", data="<params>")]
async fn upload_prepare(state: &State<AppState>, params: Json<UploadPrepare>) -> Json<UploadStatus> {

    println!("preparing upload for {:?}",params);

    let uid = {
        let mut uploader = state.uploader.lock().unwrap();
        uploader.clean_sessions();

        uploader.new_session(params.n, params.side, params.size).unwrap()
    };

    Json(UploadStatus {
        uid,
        ready: true,
    })
}


#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ChunkUpload<'r> {
    uid: u32,
    position: usize,
    data: &'r str,
}


use base64::{Engine as _, engine::general_purpose};
#[post("/upload/chunk", data="<chunk>")]
async fn upload_chunk(state: &State<AppState>, chunk: Json<ChunkUpload<'_>>) -> Result<(), rocket::response::Debug<anyhow::Error>> {
    println!("new chunk");

    // try to decode base64
    let data = general_purpose::STANDARD.decode(chunk.data).unwrap();

    {
        let mut uploader = state.uploader.lock().unwrap();
        uploader.add_chunk(chunk.uid, chunk.position, &data)?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct UploadFinish {
    uid: u32,
}

#[post("/upload/finish", data="<params>")]
async fn upload_finish(state: &State<AppState>, conn: PicturesDbConn, params: Json<UploadFinish>) -> Result<(), rocket::response::Debug<anyhow::Error>> {

    println!("finish");
    let uid = params.uid;

    // fetch pictures bytes from uploader
    let (pic_id, pic_side, bytes) = {
        let mut uploader = state.uploader.lock().unwrap();
        uploader.take(uid)?
    };

    println!("got {} bytes", bytes.len());

    let (image_bytes, thumb_bytes) = image_process::process(bytes)?;

    println!("thumb {}", thumb_bytes.len());

    conn.run(move |c| {

        // remove previous images from dbb
        database::remove_picture_blobs(c, pic_id as i32, pic_side as i32).unwrap();

        // push images to database
        database::insert_picture_blob(c, pic_id as i32, pic_side as i32, false, image_bytes).unwrap();
        database::insert_picture_blob(c, pic_id as i32, pic_side as i32, true, thumb_bytes).unwrap();

    }).await;

    Ok(())
}

use rocket::http::ContentType;

#[get("/image/<side>/<n>?<thumb>")]
async fn image(conn: PicturesDbConn, side: u32, n: u32, thumb: Option<u8>) 
    -> Result<(ContentType, Vec<u8>), rocket::response::Debug<anyhow::Error>> {

    let thumb = thumb.unwrap_or(0) != 0;
    let blob = conn.run(move |c| {
        database::fetch_picture_blob(c, side as i32, n as i32, thumb).unwrap()
    }).await;

    Ok((ContentType::JPEG, blob))
}

use rocket::fs::{relative, FileServer};

#[launch]
fn rocket() -> _ {

    build()
        .manage(AppState {
            uploader: Arc::new(Mutex::new(FileUploader::new())),
        })
        .mount("/static", FileServer::from(relative!("./static")))
        .mount("/", routes![index, edit, upload_prepare, upload_chunk, upload_finish, image])
        .attach(Template::fairing())
        .attach(PicturesDbConn::fairing())
        .attach(AdHoc::on_liftoff("Database update", |rocket| 
            Box::pin(async move {
                let conn = PicturesDbConn::get_one(rocket).await.unwrap();
                conn.run(|c| database::setup_and_update(c)).await
            })
        ))
}
