use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCapture {
    /// 原始 RGBA 像素的 Base64 编码。
    ///
    /// Tauri command 的 JSON IPC 会把 Vec<u8> 序列化为一个巨大的数字数组，
    /// 在 4K/多显示器下会产生非常高的序列化和反序列化开销。Base64 保持了
    /// 同样的数据内容，但让 IPC 只处理一个字符串。
    pub pixels: String,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
}

struct RawScreenCapture {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveScreenshotRequest {
    pub data_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteScreenshotRequest {
    pub path: String,
}

/// Captures the complete Windows virtual desktop. The returned image is kept in
/// memory so the selection layer can be shown without writing a user's screen
/// to disk before they make a selection.
#[tauri::command]
pub fn capture_screen() -> Result<ScreenCapture, String> {
    let capture = capture_screen_raw()?;
    Ok(ScreenCapture {
        pixels: encode_pixels(capture.pixels),
        width: capture.width,
        height: capture.height,
        left: capture.left,
        top: capture.top,
    })
}

fn capture_screen_raw() -> Result<RawScreenCapture, String> {
    #[cfg(target_os = "windows")]
    {
        capture_screen_windows()
    }
    #[cfg(not(target_os = "windows"))]
    {
        capture_screen_xcap()
    }
}

/// Binary variant used by the screenshot overlay. Returning `Response` makes
/// Tauri use its binary IPC path instead of serializing every pixel as JSON.
/// Packet layout: width/height (u32), left/top (i32), pixel length (u32), RGBA.
#[tauri::command]
pub fn capture_screen_binary() -> Result<tauri::ipc::Response, String> {
    let capture = capture_screen_raw()?;
    let mut packet = Vec::with_capacity(20 + capture.pixels.len());
    packet.extend_from_slice(&capture.width.to_le_bytes());
    packet.extend_from_slice(&capture.height.to_le_bytes());
    packet.extend_from_slice(&capture.left.to_le_bytes());
    packet.extend_from_slice(&capture.top.to_le_bytes());
    packet.extend_from_slice(&(capture.pixels.len() as u32).to_le_bytes());
    packet.extend_from_slice(&capture.pixels);
    Ok(tauri::ipc::Response::new(packet))
}

fn encode_pixels(pixels: Vec<u8>) -> String {
    general_purpose::STANDARD.encode(pixels)
}

/// 使用 xcap 库的高性能截图实现（使用 Windows.Graphics.Capture API）
#[cfg(not(target_os = "windows"))]
fn capture_screen_xcap() -> Result<RawScreenCapture, String> {
    use xcap::Monitor;

    // 获取所有显示器
    let monitors = Monitor::all().map_err(|e| format!("获取显示器失败: {}", e))?;

    if monitors.is_empty() {
        return Err("未找到任何显示器".to_string());
    }

    // 计算虚拟桌面的边界
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for monitor in &monitors {
        let x = monitor
            .x()
            .map_err(|e| format!("获取显示器位置失败: {}", e))?;
        let y = monitor
            .y()
            .map_err(|e| format!("获取显示器位置失败: {}", e))?;
        let width = monitor
            .width()
            .map_err(|e| format!("获取显示器尺寸失败: {}", e))?;
        let height = monitor
            .height()
            .map_err(|e| format!("获取显示器尺寸失败: {}", e))?;

        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width as i32);
        max_y = max_y.max(y + height as i32);
    }

    let full_width = (max_x - min_x) as u32;
    let full_height = (max_y - min_y) as u32;

    // 创建空白画布
    let mut pixels = vec![0u8; (full_width * full_height * 4) as usize];

    // 截取每个显示器并拼接
    for monitor in monitors {
        let image = monitor
            .capture_image()
            .map_err(|e| format!("截图失败: {}", e))?;

        let monitor_x = (monitor
            .x()
            .map_err(|e| format!("获取显示器位置失败: {}", e))?
            - min_x) as usize;
        let monitor_y = (monitor
            .y()
            .map_err(|e| format!("获取显示器位置失败: {}", e))?
            - min_y) as usize;
        let monitor_width = monitor
            .width()
            .map_err(|e| format!("获取显示器尺寸失败: {}", e))?
            as usize;
        let monitor_height = monitor
            .height()
            .map_err(|e| format!("获取显示器尺寸失败: {}", e))?
            as usize;

        // 将显示器像素拷贝到画布对应位置
        let img_data = image.as_raw();
        for y in 0..monitor_height {
            let src_offset = y * monitor_width * 4;
            let dst_offset = ((monitor_y + y) * full_width as usize + monitor_x) * 4;
            let len = monitor_width * 4;
            if src_offset + len <= img_data.len() && dst_offset + len <= pixels.len() {
                pixels[dst_offset..dst_offset + len]
                    .copy_from_slice(&img_data[src_offset..src_offset + len]);
            }
        }
    }

    Ok(RawScreenCapture {
        pixels,
        width: full_width,
        height: full_height,
        left: min_x,
        top: min_y,
    })
}

#[cfg(target_os = "windows")]
fn capture_screen_windows() -> Result<RawScreenCapture, String> {
    use std::mem::size_of;
    use std::ptr::null_mut;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, CAPTUREBLT,
        DIB_RGB_COLORS, SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if width <= 0 || height <= 0 {
        return Err("读取桌面尺寸失败".to_string());
    }

    let screen_dc = unsafe { GetDC(HWND(null_mut())) };
    if screen_dc.is_invalid() {
        return Err("获取桌面设备上下文失败".to_string());
    }
    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
    if memory_dc.is_invalid() || bitmap.is_invalid() {
        unsafe {
            if !memory_dc.is_invalid() {
                let _ = DeleteDC(memory_dc);
            }
            ReleaseDC(HWND(null_mut()), screen_dc);
            if !bitmap.is_invalid() {
                let _ = DeleteObject(bitmap);
            }
        }
        return Err("创建截图缓冲区失败".to_string());
    }

    let previous = unsafe { SelectObject(memory_dc, bitmap) };
    let copied = unsafe {
        BitBlt(
            memory_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            left,
            top,
            SRCCOPY | CAPTUREBLT,
        )
        .is_ok()
    };
    if !previous.is_invalid() {
        unsafe { SelectObject(memory_dc, previous) };
    }
    if !copied {
        unsafe {
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(memory_dc);
            ReleaseDC(HWND(null_mut()), screen_dc);
        }
        return Err("读取桌面像素失败".to_string());
    }

    // 直接读取 32 位像素，避免先读取 24 位 BGR、再逐像素扩展到 RGBA。
    // BI_RGB 的 32 位结果是 BGRX，下面只需原地交换红蓝通道并补上 alpha。
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut pixels_rgba = vec![0u8; (width as usize) * (height as usize) * 4];

    let lines = unsafe {
        GetDIBits(
            memory_dc,
            bitmap,
            0,
            height as u32,
            Some(pixels_rgba.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    unsafe {
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(memory_dc);
        ReleaseDC(HWND(null_mut()), screen_dc);
    }
    if lines == 0 {
        return Err("读取桌面像素失败".to_string());
    }

    for pixel in pixels_rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }

    Ok(RawScreenCapture {
        pixels: pixels_rgba,
        width: width as u32,
        height: height as u32,
        left,
        top,
    })
}

/// Writes a selected screenshot to the application data directory. This is a
/// temporary artifact for the next OCR stage and is deliberately not logged.
#[tauri::command]
pub fn save_screenshot(request: SaveScreenshotRequest) -> Result<String, String> {
    let encoded = request
        .data_url
        .split_once(',')
        .map(|(_, value)| value)
        .ok_or_else(|| "截图数据格式无效".to_string())?;
    let bytes = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("截图数据解码失败：{error}"))?;
    let dir = crate::utils::dirs::app_data_dir()
        .map_err(|error| format!("创建截图目录失败：{error}"))?
        .join("screenshots");
    std::fs::create_dir_all(&dir).map_err(|error| format!("创建截图目录失败：{error}"))?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("读取系统时间失败：{error}"))?
        .as_millis();
    let path = dir.join(format!("lark-screenshot-{timestamp}.png"));
    std::fs::write(&path, bytes).map_err(|error| format!("保存截图失败：{error}"))?;
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "截图路径无效".to_string())
}

/// Returns bytes for a screenshot created by `save_screenshot`.
///
/// OCR is intentionally limited to the temporary screenshot directory. The
/// path comes from the renderer, so canonicalising it and checking the file
/// name here prevents the OCR command from becoming an arbitrary file reader.
pub(crate) fn read_screenshot_bytes(path: &str) -> Result<Vec<u8>, String> {
    let root = crate::utils::dirs::app_data_dir()
        .map_err(|error| format!("读取截图目录失败：{error}"))?
        .join("screenshots");
    let candidate = std::path::PathBuf::from(path);
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| "截图路径无效或不存在".to_string())?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("读取截图目录失败：{error}"))?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err("截图路径无效：不在允许的目录内".to_string());
    }
    let file_name = canonical_candidate
        .file_name()
        .ok_or_else(|| "截图路径无效".to_string())?
        .to_string_lossy();
    if !file_name.starts_with("lark-screenshot-") || !file_name.ends_with(".png") {
        return Err("截图文件名格式无效".to_string());
    }
    std::fs::read(canonical_candidate).map_err(|error| format!("读取截图失败：{error}"))
}

#[tauri::command]
pub fn delete_screenshot(request: DeleteScreenshotRequest) -> Result<(), String> {
    let root = crate::utils::dirs::app_data_dir()
        .map_err(|error| format!("读取截图目录失败：{error}"))?
        .join("screenshots");
    let candidate = std::path::PathBuf::from(&request.path);

    // 规范化路径并检查是否在允许的目录内，防止路径遍历攻击
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| "截图路径无效或不存在".to_string())?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("读取截图目录失败：{error}"))?;

    if !canonical_candidate.starts_with(&canonical_root) {
        return Err("截图路径无效：不在允许的目录内".to_string());
    }

    // 额外检查：确保文件名符合预期格式
    if let Some(file_name) = canonical_candidate.file_name() {
        let name = file_name.to_string_lossy();
        if !name.starts_with("lark-screenshot-") || !name.ends_with(".png") {
            return Err("截图文件名格式无效".to_string());
        }
    } else {
        return Err("截图路径无效".to_string());
    }

    match std::fs::remove_file(canonical_candidate) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("删除截图失败：{error}")),
    }
}
