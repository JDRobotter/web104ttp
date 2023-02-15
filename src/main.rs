use rocket::*;
use rocket_dyn_templates::{Template, context};

use std::path::Path;

mod words;
use words::WORDS;

struct AppState {
    words: Vec<(u64, &'static str)>,
}

#[get("/")]
fn index(state: &State<AppState>) -> Template {
    Template::render("index", context! {
        words: &state.words,
    })
}

#[get("/image/<_side>/<n>")]
async fn image(_side: u64, n: u64) -> Option<fs::NamedFile> {
    let path = format!("/home/jdam/Pictures/STJLZ_{}.png", n);
    let path = Path::new(&path);
    fs::NamedFile::open(path).await.ok()
}

#[launch]
fn rocket() -> _ {
    build()
        .manage(AppState {
            words: Vec::from(WORDS),
        })
        .mount("/", routes![index, image])
        .attach(Template::fairing())
}
