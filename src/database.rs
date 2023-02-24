use diesel::prelude::*;
use crate::models::*;

use anyhow::Result;

use crate::schema::pictures::table as pictures_table;
use crate::schema::blobs::table as blobs_table;
use crate::words::WORDS;

pub fn fetch_picture_blob(conn: &mut diesel::SqliteConnection,
                          picture_side: i32,
                          picture_id: i32,
                          is_thumbnail: bool) -> Result<Vec<u8>> {

    use crate::schema::blobs::dsl::*;
    let blob = blobs_table
        .select((id, side, thumbnail, data, mime, show, hash))
        .filter(id.eq(picture_id)
                .and(side.eq(picture_side))
                .and(thumbnail.eq(is_thumbnail)))
        .first::<Blob>(conn)?;

    Ok(blob.data)
}

pub fn remove_picture_blobs(conn: &mut diesel::SqliteConnection,
                                picture_id: i32,
                                picture_side: i32) -> Result<()> {

    use crate::schema::blobs::dsl::*;
    diesel::delete(blobs_table)
        .filter(id.eq(picture_id)
                .and(side.eq(picture_side))
        )
        .execute(conn)?;
    
    Ok(())
}

pub fn insert_picture_blob(conn: &mut diesel::SqliteConnection,
                            picture_id: i32,
                            picture_side: i32,
                            is_thumbnail: bool,
                            blob: Vec<u8>,
                            hash: i32) -> Result<()> {

    diesel::insert_into(blobs_table)
        .values(Blob {
            id: picture_id,
            side: picture_side,
            thumbnail: is_thumbnail,
            data: blob,
            mime: String::new(),
            show: true,
            hash,
        })
        .execute(conn)?;

    Ok(())
}

pub fn fetch_blob_show(conn: &mut diesel::SqliteConnection, picture_side: i32, picture_id: i32) -> Result<bool> {

    use crate::schema::blobs::dsl::*;
    let b = blobs_table
        .select(show)
        .filter(id.eq(picture_id)
                .and(side.eq(picture_side))
        )
        .first::<bool>(conn)
        .unwrap_or(false);

    Ok(b)
}

pub fn list_blobs_show(conn: &mut diesel::SqliteConnection) -> Result<Vec<(i32, i32, bool, i32)>> {

    use crate::schema::blobs::dsl::*;
    let vec = blobs_table
        .select((id, side, show, hash))
        .load::<(i32, i32, bool, i32)>(conn)?;

    Ok(vec)
}

pub fn list_pictures(conn: &mut diesel::SqliteConnection) -> Vec<Picture> {
    
    pictures_table
        .load::<Picture>(conn)
        .expect("Error loading pictures")
}

pub fn setup_and_update(conn: &mut diesel::SqliteConnection) {

    for (s_id, s_word) in WORDS {

        let s_id = s_id as i32;

        // check if this picture ID is declared in database
        use crate::schema::pictures::dsl::*;
        use diesel::dsl::count;
        let count:i64 = pictures
                .filter(id.eq(s_id))
                .select(count(id))
                .first(conn)
                .expect("unable to request pictures");

        if count > 0 {
            // entry already exist, update word if needed
            diesel::update(pictures_table)
                .filter(id.eq(s_id))
                .set(word.eq(s_word))
                .execute(conn)
                .expect("unable to update pictures rows");
        }
        else {
            // entry does not exist push a new default entry
            diesel::insert_into(pictures_table)
                .values(Picture {
                    id: s_id,
                    word: s_word.to_string(),
                })
                .execute(conn)
                .expect("unable to insert new picture row");

        }
    }

}
