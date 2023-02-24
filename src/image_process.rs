use anyhow::Result;

use std::io::Cursor;
use ::image::{io::Reader, ImageOutputFormat, imageops::FilterType};

fn hash(bytes: &Vec<u8>) -> i32 {
    (seahash::hash(bytes) & 0x7fffffff) as i32
}

pub fn process(bytes: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>, i32)> {

    let hash = hash(&bytes);

    let reader = Reader::new(Cursor::new(bytes))
                        .with_guessed_format()
                        .unwrap();

    let im = reader.decode()?;

    let main = im.resize(1920, 1080, FilterType::Triangle);
    let thumb = im.resize(640, 480, FilterType::Triangle);

    let mut thumb_buf = Cursor::new(vec![]);
    thumb.write_to(&mut thumb_buf, ImageOutputFormat::Jpeg(95))?;

    let mut main_buf = Cursor::new(vec![]);
    main.write_to(&mut main_buf, ImageOutputFormat::Jpeg(95))?;

    Ok((main_buf.into_inner(), thumb_buf.into_inner(), hash))
}
