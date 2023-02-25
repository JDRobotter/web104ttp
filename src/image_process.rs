use anyhow::Result;

use std::io::Cursor;
use ::image::{io::Reader, ImageOutputFormat, imageops::FilterType};
use exif::{Tag, In};

fn hash(bytes: &Vec<u8>) -> i32 {
    (seahash::hash(bytes) & 0x7fffffff) as i32
}

fn get_exif_orientation(exif: exif::Exif) -> u32 {

    match exif.get_field(Tag::Orientation, In::PRIMARY) {
        Some(orientation) => {
            orientation.value.get_uint(0).unwrap_or(1)
        },
        None => 1,
    }

}

pub fn process(bytes: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>, i32)> {

    let hash = hash(&bytes);

    // -- parse exif tag
    let exif = ::exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes.clone()))?;

    let orientation = get_exif_orientation(exif);


    // -- parse image
    let reader = Reader::new(Cursor::new(&bytes))
                        .with_guessed_format()
                        .unwrap();

    let im = reader.decode()?;

    println!("image : {}x{}; orientation={}", im.width(), im.height(), orientation);

    // manage exif orientation 
    // (http://sylvana.net/jpegcrop/exif_orientation.html)
    let im = match orientation {

        1 => im,
        2 => im.flipv(),
        3 => im.rotate180(),
        4 => im.fliph(),
        5 => im.rotate90().flipv(),
        6 => im.rotate90(),
        7 => im.rotate270().fliph(),
        8 => im.rotate270(),
        _ => { panic!("unhandled EXIF orientation")/* TODO FIXME */ }
    };

    let main = im.resize(1920, 1080, FilterType::Triangle);
    let thumb = im.resize(864, 486, FilterType::Triangle);

    let mut thumb_buf = Cursor::new(vec![]);
    thumb.write_to(&mut thumb_buf, ImageOutputFormat::Jpeg(95))?;

    let mut main_buf = Cursor::new(vec![]);
    main.write_to(&mut main_buf, ImageOutputFormat::Jpeg(95))?;

    Ok((main_buf.into_inner(), thumb_buf.into_inner(), hash))
}
