use rocket::*;
use rocket_dyn_templates::{Template, context};
use rocket::fairing::AdHoc;
use anyhow::Result;

use std::sync::{Arc, Mutex};

#[macro_use]
extern crate diesel;

mod words;
mod database;
mod schema;
mod models;
mod uploader;
mod image_process;

use rocket_sync_db_pools::database;
use rocket_auth::{Users, User, Auth, Login};

#[database("pictures")]
struct PicturesDbConn(diesel::SqliteConnection);

use crate::uploader::FileUploader;

pub struct AppState {

    uploader: Arc<Mutex<FileUploader>>,
}

use rocket::response::Redirect;
use rocket::form::Form;

fn side_from_email(email: &str) -> i32 {
    if email == "jd" {
        1
    }
    else {
        0
    }
}


#[post("/login", data="<form>")]
async fn login(form: Form<Login>, auth: Auth<'_>) -> Redirect {
    let r = auth.login(&form).await;

    if r.is_ok() {
        Redirect::to("/")
    }
    else {
        Redirect::to("/auth")
    }
}

#[get("/logout")]
fn logout(auth: Auth<'_>) -> Redirect {
    auth.logout().ok();

    Redirect::to("/")
}

#[catch(401)]
async fn catch_401() -> Redirect {
    Redirect::to("/auth")
}


#[get("/auth")]
async fn no_auth()
        -> Result<Template, rocket::response::Debug<anyhow::Error>> {

    Ok(Template::render("auth", context! {
    }))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct PictureWithShows {
    id: i32,
    word: String,
    shows: Vec<bool>,
    hashs: Vec<i32>,
}

#[get("/")]
async fn index(conn: PicturesDbConn, user:User)
        -> Result<Template, rocket::response::Debug<anyhow::Error>> {

    let (pictures, shows) = conn.run(move |c| { 
        let pictures = database::list_pictures(c);
        let shows = database::list_blobs_show(c).unwrap();
        (pictures,shows)
    } ).await;

    let mut pws = vec![];
    for pic in pictures {

        let mut ps = vec![false; 2];
        let mut hashs = vec![0; 2];
        for (pic_id, pic_side, show, hash) in &shows {
            if *pic_id == pic.id {
                ps[*pic_side as usize] = *show;
                hashs[*pic_side as usize] = *hash;
            }
        }

        pws.push(PictureWithShows {
            id: pic.id,
            word: pic.word,
            shows: ps,
            hashs,
        });
    }

    Ok(Template::render("index", context! {
        pictures: pws,
        my_side: side_from_email(user.email()),
    }))
}

#[get("/edit/<side>/<n>")]
async fn edit(conn: PicturesDbConn, _user:User, side: u32, n: u32) -> Template {
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
async fn upload_prepare(state: &State<AppState>, _user:User, params: Json<UploadPrepare>) -> Json<UploadStatus> {

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
async fn upload_chunk(state: &State<AppState>, _user:User, chunk: Json<ChunkUpload<'_>>) -> Result<(), rocket::response::Debug<anyhow::Error>> {

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
async fn upload_finish(state: &State<AppState>, _user:User, conn: PicturesDbConn, params: Json<UploadFinish>)
        -> Result<(), rocket::response::Debug<anyhow::Error>> {

    let uid = params.uid;

    // fetch pictures bytes from uploader
    let (pic_id, pic_side, bytes) = {
        let mut uploader = state.uploader.lock().unwrap();
        uploader.take(uid)?
    };

    let (image_bytes, thumb_bytes, hash) = image_process::process(bytes)?;

    conn.run(move |c| {

        // remove previous images from dbb
        database::remove_picture_blobs(c, pic_id as i32, pic_side as i32).unwrap();

        // push images to database
        database::insert_picture_blob(c, pic_id as i32, pic_side as i32, false, image_bytes, hash).unwrap();
        database::insert_picture_blob(c, pic_id as i32, pic_side as i32, true, thumb_bytes, hash).unwrap();

    }).await;

    Ok(())
}

use rocket::http::ContentType;

#[get("/image/<side>/<n>?<thumb>")]
async fn image(conn: PicturesDbConn, _user:User, side: u32, n: u32, thumb: Option<u8>) 
    -> Result<(ContentType, Vec<u8>), rocket::response::Debug<anyhow::Error>> {

    let thumb = thumb.unwrap_or(0) != 0;
    let blob = conn.run(move |c| {
        database::fetch_picture_blob(c, side as i32, n as i32, thumb).unwrap()
    }).await;

    Ok((ContentType::JPEG, blob))
}

use rocket::fs::{relative, FileServer};

#[launch]
async fn rocket() -> _ {

    let users = Users::open_rusqlite("./users.db").unwrap();
    users.create_user("jd", "loutre42", true).await.ok();
    users.create_user("el", "loutre42", true).await.ok();

    build()
        .manage(AppState {
            uploader: Arc::new(Mutex::new(FileUploader::new())),
        })
        .manage(users)
        .mount("/static", FileServer::from(relative!("./static")))
        .mount("/", routes![login, logout, no_auth, index, edit, upload_prepare, upload_chunk, upload_finish, image])
        .register("/", catchers![catch_401])
        .attach(Template::fairing())
        .attach(PicturesDbConn::fairing())
        .attach(AdHoc::on_liftoff("Database update", |rocket| 
            Box::pin(async move {
                let conn = PicturesDbConn::get_one(rocket).await.unwrap();
                conn.run(|c| database::setup_and_update(c)).await
            })
        ))
}
