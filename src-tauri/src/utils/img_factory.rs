use super::string_factory;
use anyhow::{anyhow, Result};
use arboard::ImageData;
use base64::engine::general_purpose;
use base64::Engine;
use image::codecs::bmp::BmpDecoder;
use image::ColorType::Rgba8;
use image::{ColorType, DynamicImage, ExtendedColorType, ImageEncoder};
use std::borrow::Cow;
use std::io::{BufReader, BufWriter, Cursor};

pub fn rgba8_to_base64(img: &ImageData) -> String {
    let mut bytes: Vec<u8> = Vec::new();
    image::codecs::png::PngEncoder::new(BufWriter::new(Cursor::new(&mut bytes)))
        .write_image(
            &img.bytes,
            img.width as u32,
            img.height as u32,
            ExtendedColorType::from(image::ColorType::Rgba8),
            // Rgba8,
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
    let cursor = Cursor::new(&mut bytes);
    let writer = BufWriter::new(cursor);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, quality);
    encoder
        .write_image(
            &rgb_bytes,
            img.width as u32,
            img.height as u32,
            ExtendedColorType::Rgb8,
        )
        .unwrap();
    string_factory::base64_encode(&bytes)
}

pub fn base64_to_rgba8(base64_str: &str) -> Result<ImageData<'_>> {
    let slice = general_purpose::STANDARD.decode(base64_str)?;
    let img = image::load_from_memory(&slice).expect("Error");
    Ok(ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: Cow::from(img.into_rgba8().into_raw()),
    })
}
