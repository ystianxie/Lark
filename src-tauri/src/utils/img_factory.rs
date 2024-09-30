use super::string_factory;
use anyhow::Result;
use arboard::ImageData;
use image::{ColorType, ExtendedColorType, ImageEncoder};
use std::io::{BufReader, BufWriter, Cursor};
use image::ColorType::Rgba8;

pub fn rgba8_to_base64(img: &ImageData) -> String {
    let mut bytes: Vec<u8> = Vec::new();
    image::codecs::png::PngEncoder::new(BufWriter::new(Cursor::new(&mut bytes)))
        .write_image(
            &img.bytes,
            img.width as u32,
            img.height as u32,
            // ExtendedColorType::from(image::ColorType::Rgba8),
            Rgba8
        )
        .unwrap();
    string_factory::base64_encode(bytes.as_slice())
}

pub fn rgba8_to_jpeg_base64(img: &ImageData, quality: u8) -> String {
    // 创建一个没有 alpha 通道的 RGB 图像缓冲区
    let mut rgb_bytes: Vec<u8> = Vec::with_capacity((img.width * img.height * 3) as usize);
    for chunk in img.bytes.chunks(4) {
        rgb_bytes.push(chunk[0]); // R
        rgb_bytes.push(chunk[1]); // G
        rgb_bytes.push(chunk[2]); // B
        // 丢弃 alpha 通道 chunk[3]
    }
    let mut bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    // let writer = BufWriter::new(cursor);
    // let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, quality);
    // encoder.write_image(&rgb_bytes, img.width as u32, img.height as u32, ExtendedColorType::Rgb8).unwrap();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
    string_factory::base64_encode(&bytes)
}

pub fn base64_to_rgba8(base64: &str) -> Result<ImageData> {
    let bytes = string_factory::base64_decode(base64);
    let reader =
        image::io::Reader::with_format(BufReader::new(Cursor::new(bytes)), image::ImageFormat::Png);
    match reader.decode() {
        Ok(img) => {
            let rgba = img.into_rgba8();
            let (width, height) = rgba.dimensions();
            Ok(ImageData {
                width: width as usize,
                height: height as usize,
                bytes: rgba.into_raw().into(),
            })
        }
        Err(_) => Err(anyhow::anyhow!("decode image error")),
    }
}
