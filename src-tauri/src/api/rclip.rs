//! 从系统剪切板读取内容

use base64::{engine::general_purpose, Engine};
use image::{codecs::bmp::BmpDecoder, DynamicImage};
use std::io::Cursor;
use std::panic;
use windows::Win32::{
    Foundation::{HGLOBAL, HWND},
    System::{
        DataExchange::{
            CloseClipboard, EnumClipboardFormats, GetClipboardData, IsClipboardFormatAvailable,
            OpenClipboard,
        },
        Memory::{GlobalLock, GlobalSize, GlobalUnlock},
    },
    UI::Shell::{DragQueryFileW, HDROP},
};
#[derive(Debug)]
enum ClipboardContent {
    Text(String),           // 存储文本
    FilePaths(Vec<String>), // 存储文件路径列表
    Image(String),          // 存储图片的 base64 字符串
}
#[derive(Debug)]
enum ClipboardContentType {
    Text,
    Files,
    Image,
}

#[derive(Debug)]
struct ClipboardData {
    content_type: ClipboardContentType, // 内容类型
    content: ClipboardContent,          // 存储具体内容
}

impl ClipboardData {
    fn new_text(content: String) -> Self {
        ClipboardData {
            content_type: ClipboardContentType::Text,
            content: ClipboardContent::Text(content),
        }
    }

    fn new_files(content: Vec<String>) -> Self {
        ClipboardData {
            content_type: ClipboardContentType::Files,
            content: ClipboardContent::FilePaths(content),
        }
    }

    fn new_image(content: String) -> Self {
        ClipboardData {
            content_type: ClipboardContentType::Image,
            content: ClipboardContent::Image(content),
        }
    }
}

#[test]
fn main() {
    // 在读取时需要打开剪切板、读取完成时需要关闭剪切以释放资源
    unsafe {
        OpenClipboard(HWND(0x0 as _)).expect("failure to OpenClipboard");

        // 通过枚举剪切板以获得所有的 format id，第一次传入 0 可获取下一个 format id
        let mut format = EnumClipboardFormats(0);
        while format != 0 {
            println!("- Format ID: {:?}", format);
            format = EnumClipboardFormats(format);
        }

        // format id:
        //   13: CF_UNICODETEXT (文本)
        //   15: CF_HDROP (文件)
        //   17: CF_DIBV5 (图片)
        if let Ok(_) = IsClipboardFormatAvailable(13) {
            match GetClipboardData(13 as u32) {
                Ok(handle) => {
                    println!("{:?}", handle);
                    // 通常使用 `0` 表示字符串的结束，因此可以通过遍历原始指针的方式找到第一个 `0`
                    let ptr = handle.0 as *const u16;
                    let mut length = 0;
                    while *ptr.add(length) != 0 {
                        length += 1;
                    }
                    // println!("{:?}", length);

                    let slice = std::slice::from_raw_parts(ptr, length);
                    println!("{:?}", slice);
                    println!("{:?}", String::from_utf16_lossy(slice));
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        } else if let Ok(_) = IsClipboardFormatAvailable(15) {
            match GetClipboardData(15 as u32) {
                Ok(handle) => {
                    println!("{:?}", handle);
                    let ptr = GlobalLock(HGLOBAL(handle.0));
                    let num_files = *(ptr as *mut u32);
                    for i in 0..num_files {
                        let mut buffer = vec![0u16; 1024];
                        let path_len = DragQueryFileW(HDROP(handle.0), i, Some(&mut buffer));
                        if path_len == 0 {
                            continue;
                        }
                        println!("{:?}", &buffer[..path_len as usize]);
                        let path = String::from_utf16_lossy(&buffer[..path_len as usize]);
                        println!("{:?}", path);
                    }
                    let _ = GlobalUnlock(HGLOBAL(handle.0));
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        } else if let Ok(_) = IsClipboardFormatAvailable(17) {
            match GetClipboardData(17 as u32) {
                Ok(handle) => {
                    println!("{:?}", handle);
                    let ptr = GlobalLock(HGLOBAL(handle.0));
                    let size = GlobalSize(HGLOBAL(ptr));
                    let slice = std::slice::from_raw_parts(ptr as *const u8, size);
                    let decoder = BmpDecoder::new_without_file_header(Cursor::new(slice)).unwrap();
                    let dynamic_image = DynamicImage::from_decoder(decoder).unwrap();
                    let mut buffer: Vec<u8> = Vec::new();
                    dynamic_image
                        .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                        .unwrap();
                    println!("{:?}", general_purpose::STANDARD_NO_PAD.encode(&buffer));
                    let _ = GlobalUnlock(HGLOBAL(handle.0));
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        }

        CloseClipboard().expect("failure to CloseClipboard");
    }
}

pub fn read_clipboard() -> Result<ClipboardData, String> {
    unsafe {
        let result = panic::catch_unwind(|| {
            OpenClipboard(HWND(0x0 as _)).expect("failure to OpenClipboard");
            // 通过枚举剪切板以获得所有的 format id，第一次传入 0 可获取下一个 format id
            let mut format = EnumClipboardFormats(0);
            while format != 0 {
                format = EnumClipboardFormats(format);
            }
            // format id:
            //   13: CF_UNICODETEXT (文本)
            //   15: CF_HDROP (文件)
            //   17: CF_DIBV5 (图片)
            let mut result: Option<ClipboardData> = None;
            if let Ok(_res) = IsClipboardFormatAvailable(13) {
                match GetClipboardData(13 as u32) {
                    Ok(handle) => {
                        // 通常使用 `0` 表示字符串的结束，因此可以通过遍历原始指针的方式找到第一个 `0`
                        let ptr = handle.0 as *const u16;
                        let mut length = 0;
                        while *ptr.add(length) != 0 {
                            length += 1;
                        }
                        let slice = std::slice::from_raw_parts(ptr, length);
                        // println!("{:?}", String::from_utf16_lossy(slice));
                        result = Some(ClipboardData::new_text(String::from_utf16_lossy(slice)));
                    }
                    Err(err) => {
                        println!("{:?}", err);
                        return Err(err);
                    }
                }
            } else if let Ok(_res) = IsClipboardFormatAvailable(15) {
                match GetClipboardData(15 as u32) {
                    Ok(handle) => {
                        let ptr = GlobalLock(HGLOBAL(handle.0));
                        let num_files = *(ptr as *mut u32);
                        let mut path_list = vec![];
                        for i in 0..num_files {
                            let mut buffer = vec![0u16; 1024];
                            let path_len = DragQueryFileW(HDROP(handle.0), i, Some(&mut buffer));
                            if path_len == 0 {
                                continue;
                            }
                            let path = String::from_utf16_lossy(&buffer[..path_len as usize]);
                            path_list.push(path);
                        }
                        let _ = GlobalUnlock(HGLOBAL(handle.0));
                        result = Some(ClipboardData::new_files(path_list));
                    }
                    Err(err) => {
                        println!("{:?}", err);
                        return Err(err);
                    }
                }
            } else if let Ok(_res) = IsClipboardFormatAvailable(17) {
                match GetClipboardData(17 as u32) {
                    Ok(handle) => {
                        println!("{:?}", handle);
                        let ptr = GlobalLock(HGLOBAL(handle.0));
                        let size = GlobalSize(HGLOBAL(ptr));
                        let slice = std::slice::from_raw_parts(ptr as *const u8, size);
                        let decoder =
                            BmpDecoder::new_without_file_header(Cursor::new(slice)).unwrap();
                        let dynamic_image = DynamicImage::from_decoder(decoder).unwrap();
                        let mut buffer: Vec<u8> = Vec::new();
                        dynamic_image
                            .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                            .unwrap();
                        let _ = GlobalUnlock(HGLOBAL(handle.0));
                        result = Some(ClipboardData::new_image(
                            general_purpose::STANDARD_NO_PAD.encode(&buffer),
                        ));
                    }
                    Err(err) => {
                        println!("{:?}", err);
                        return Err(err);
                    }
                }
            }
            Ok(result.unwrap())
        });

        CloseClipboard().expect("failure to CloseClipboard");
        match result {
            Ok(value) => match value {
                Ok(res) => Ok(res),
                Err(err) => Err(err.to_string()),
            },
            Err(err) => Err("读取剪贴板异常！".to_string()),
        }
    }
}

#[test]
fn test_read_clipboard() {
    let resu = read_clipboard().unwrap();
    println!("结果：{:?}", &resu.content_type);
    println!("结果：{:?}", &resu.content);
}
