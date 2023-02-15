use rocket::*;
use rocket_dyn_templates::{Template, context};

use std::path::Path;

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {
        test:"loutre",
    })
}

#[get("/show/<n>")]
fn show(n: u64) -> Template {
    Template::render("show", context! {
        n,
    })
}

#[get("/image/<n>")]
async fn image(n: u64) -> Option<fs::NamedFile> {
    fs::NamedFile::open(Path::new("/home/jdam/Downloads/unknown.png")).await.ok()
}

#[launch]
fn rocket() -> _ {
    build()
        .mount("/", routes![index, show, image])
        .attach(Template::fairing())
}
