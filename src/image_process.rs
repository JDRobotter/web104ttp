use anyhow::Result;

use std::io::Cursor;
use ::image::{io::Reader, ImageOutputFormat, imageops::FilterType};

pub fn process(bytes: Vec<u8>) -> Result<(Vec<u8>,Vec<u8>)> {

    let reader = Reader::new(Cursor::new(bytes))
                        .with_guessed_format()
                        .unwrap();

    println!("decoding received image");
    let im = reader.decode()?;

    println!("resizing to 1920x1080");
    let main = im.resize(1920, 1080, FilterType::Triangle);
    println!("resizing to 640x480");
    let thumb = im.resize(640, 480, FilterType::Triangle);

    println!("writing");
    let mut thumb_buf = Cursor::new(vec![]);
    thumb.write_to(&mut thumb_buf, ImageOutputFormat::Jpeg(95))?;

    let mut main_buf = Cursor::new(vec![]);
    main.write_to(&mut main_buf, ImageOutputFormat::Jpeg(95))?;

    Ok((main_buf.into_inner(), thumb_buf.into_inner()))
}
