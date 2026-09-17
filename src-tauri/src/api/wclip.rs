//! 写入到剪切板

use crate::utils::img_factory;
use std::{error::Error, ptr::copy_nonoverlapping};
use windows::Win32::{
    Foundation::{BOOL, HWND},
    Graphics::Gdi::{
        CreateDIBitmap, GetDC, BITMAPV4HEADER, BI_BITFIELDS, CBM_INIT, CIEXYZTRIPLE, DIB_RGB_COLORS,
    },
    System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
    UI::Shell::DROPFILES,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let format = args[1].as_str();
    println!("{:?}", format);

    unsafe {
        OpenClipboard(HWND(0x0 as _))?;
        EmptyClipboard()?;
        match format {
            "CF_TEXT" | "CF_UNICODETEXT" => {
                let text = "Hello Rust, 你好世界!";
                let mut wide = text.encode_utf16().collect::<Vec<u16>>();
                wide.push(0);
                SetClipboardData(13, HWND(wide.as_ptr() as *mut _))?;
            }
            "CF_BITMAP" | "CF_DIB" | "CF_DIBV4" | "CF_DIBV5" => {
                let data = std::fs::read(
                    r"C:\Users\muzi\Downloads\DALL·E 2024-12-11 11.03.16 - A vector-style cartoon icon of a cute cat, designed in a minimalistic style, with a single-colored background, and the cat depicted in no more than th.webp",
                )?;
                let image = image::load_from_memory(&data)?;
                let bytes: Vec<u8> = image.to_rgba8().into_raw();
                let header = BITMAPV4HEADER {
                    bV4Size: std::mem::size_of::<BITMAPV4HEADER>() as _,
                    bV4Width: image.width() as i32,
                    bV4Height: -(image.height() as i32),
                    bV4Planes: 1,
                    bV4BitCount: 32,
                    bV4V4Compression: BI_BITFIELDS,
                    bV4SizeImage: (4 * image.width() * image.height()),
                    bV4XPelsPerMeter: 0,
                    bV4YPelsPerMeter: 0,
                    bV4ClrUsed: 0,
                    bV4ClrImportant: 0,
                    bV4RedMask: u32::from_le(0x0000ff),
                    bV4GreenMask: u32::from_le(0x00ff00),
                    bV4BlueMask: u32::from_le(0xff0000),
                    bV4AlphaMask: u32::from_le(0x000000),
                    bV4CSType: 0,
                    bV4Endpoints: std::mem::MaybeUninit::<CIEXYZTRIPLE>::zeroed().assume_init(),
                    bV4GammaRed: 0,
                    bV4GammaGreen: 0,
                    bV4GammaBlue: 0,
                };
                let hdc = GetDC(HWND(0x0 as _));
                let hbitmap = CreateDIBitmap(
                    hdc,
                    Some(&header as *const BITMAPV4HEADER as *const _),
                    CBM_INIT as u32,
                    Some(bytes.as_ptr() as *const _),
                    Some(&header as *const BITMAPV4HEADER as *const _),
                    DIB_RGB_COLORS,
                );
                SetClipboardData(2, HWND(hbitmap.0))?;
            }
            "CF_HDROP" => {
                let paths = [
                    r"c:\Users\muzi\Downloads\Snipaste_2025-01-03_13-52-57.png",
                    r"C:\Users\muzi\Downloads\GWS6WqcWQAEt6Ib.jpg",
                ];
                // buffer 存储文件路径, 并在每个文件最后添加结束符 `\0`
                let mut buffer: Vec<u16> = Vec::new();
                for path in paths.into_iter() {
                    buffer.append(&mut path.encode_utf16().collect::<Vec<u16>>());
                    buffer.push(0);
                }

                // 获取结构体大小
                let p_files = size_of::<DROPFILES>();

                // 用以存储 DROPFILES 结构体和文件路径数据, u16 类型每字符占 2 字节, 故需要 *2
                let mut h_global = vec![0u8; buffer.len() * 2 + p_files];

                // 创建指向 DROPFILES 结构体的指针
                let dropfiles: *mut DROPFILES = h_global.as_mut_ptr() as *mut DROPFILES;
                (*dropfiles).pFiles = p_files as _;
                (*dropfiles).fWide = BOOL(1);

                // 将文件路径数据复制到 h_global 缓冲区
                copy_nonoverlapping(
                    buffer.as_ptr(),
                    h_global.as_mut_ptr().offset(p_files as _) as *mut u16,
                    buffer.len(),
                );
                SetClipboardData(15, HWND(h_global.as_mut_ptr() as *mut _))?;
            }
            _ => {}
        }
        CloseClipboard()?;
    }
    Ok(())
}

#[test]
fn write_clipboard() -> Result<(), Box<dyn Error>> {
    let format = "CF_TEXT";
    let text = "Hello Rust, 你好世界!";
    let paths = [r"D:\settings.py", r"D:\spiders.py"];
    let bs64 = "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";
    unsafe {
        OpenClipboard(HWND(0x0 as _))?;
        EmptyClipboard()?;
        match format {
            "CF_TEXT" | "CF_UNICODETEXT" => {
                let mut wide = text.encode_utf16().collect::<Vec<u16>>();
                wide.push(0);
                SetClipboardData(13, HWND(wide.as_ptr() as *mut _))?;
            }
            "CF_BITMAP" | "CF_DIB" | "CF_DIBV4" | "CF_DIBV5" => {
                let image = img_factory::base64_to_rgba8(bs64)?;
                let header = BITMAPV4HEADER {
                    bV4Size: std::mem::size_of::<BITMAPV4HEADER>() as _,
                    bV4Width: image.width as i32,
                    bV4Height: -(image.height as i32),
                    bV4Planes: 1,
                    bV4BitCount: 32,
                    bV4V4Compression: BI_BITFIELDS,
                    bV4SizeImage: (4 * image.width * image.height) as u32,
                    bV4XPelsPerMeter: 0,
                    bV4YPelsPerMeter: 0,
                    bV4ClrUsed: 0,
                    bV4ClrImportant: 0,
                    bV4RedMask: u32::from_le(0x0000ff),
                    bV4GreenMask: u32::from_le(0x00ff00),
                    bV4BlueMask: u32::from_le(0xff0000),
                    bV4AlphaMask: u32::from_le(0x000000),
                    bV4CSType: 0,
                    bV4Endpoints: std::mem::MaybeUninit::<CIEXYZTRIPLE>::zeroed().assume_init(),
                    bV4GammaRed: 0,
                    bV4GammaGreen: 0,
                    bV4GammaBlue: 0,
                };
                let hdc = GetDC(HWND(0x0 as _));
                let hbitmap = CreateDIBitmap(
                    hdc,
                    Some(&header as *const BITMAPV4HEADER as *const _),
                    CBM_INIT as u32,
                    Some(image.bytes.as_ptr() as *const _),
                    Some(&header as *const BITMAPV4HEADER as *const _),
                    DIB_RGB_COLORS,
                );
                SetClipboardData(2, HWND(hbitmap.0))?;
            }
            "CF_HDROP" => {
                // buffer 存储文件路径, 并在每个文件最后添加结束符 `\0`
                let mut buffer: Vec<u16> = Vec::new();
                for path in paths.into_iter() {
                    buffer.append(&mut path.encode_utf16().collect::<Vec<u16>>());
                    buffer.push(0);
                }

                // 获取结构体大小
                let p_files = size_of::<DROPFILES>();

                // 用以存储 DROPFILES 结构体和文件路径数据, u16 类型每字符占 2 字节, 故需要 *2
                let mut h_global = vec![0u8; buffer.len() * 2 + p_files];

                // 创建指向 DROPFILES 结构体的指针
                let dropfiles: *mut DROPFILES = h_global.as_mut_ptr() as *mut DROPFILES;
                (*dropfiles).pFiles = p_files as _;
                (*dropfiles).fWide = BOOL(1);

                // 将文件路径数据复制到 h_global 缓冲区
                copy_nonoverlapping(
                    buffer.as_ptr(),
                    h_global.as_mut_ptr().offset(p_files as _) as *mut u16,
                    buffer.len(),
                );
                SetClipboardData(15, HWND(h_global.as_mut_ptr() as *mut _))?;
            }
            _ => {}
        }
        CloseClipboard()?;
    }
    Ok(())
}
