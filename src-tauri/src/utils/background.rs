//! Native acrylic, using the same accent policy as Gizmo's background implementation.
use std::{ffi::c_void, mem::size_of};
use windows::{
    core::{s, w},
    Win32::{
        Foundation::{BOOL, HWND},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
        },
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
    },
};

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct CompositionAttributeData {
    attribute: u32,
    data: *mut c_void,
    size: usize,
}

pub fn set_acrylic(hwnd: HWND, enabled: bool) -> Result<(), String> {
    type SetCompositionAttribute =
        unsafe extern "system" fn(HWND, *mut CompositionAttributeData) -> BOOL;

    unsafe {
        let user32 = GetModuleHandleW(w!("user32.dll")).map_err(|error| error.to_string())?;
        let address = GetProcAddress(user32, s!("SetWindowCompositionAttribute"))
            .ok_or_else(|| "Windows 不支持窗口磨砂效果".to_string())?;
        // Resolve the symbol before converting it; never transmute a missing function.
        let set_composition_attribute: SetCompositionAttribute = std::mem::transmute(address);
        let mut policy = AccentPolicy {
            // ACCENT_ENABLE_ACRYLICBLURBEHIND / ACCENT_DISABLED.
            state: if enabled { 4 } else { 0 },
            flags: 0,
            // ABGR: white with 50% opacity, matching Gizmo's light acrylic.
            gradient_color: if enabled { 0x7fff_ffff } else { 0 },
            animation_id: 0,
        };
        let mut data = CompositionAttributeData {
            attribute: 0x13, // WCA_ACCENT_POLICY
            data: &mut policy as *mut AccentPolicy as *mut c_void,
            size: size_of::<AccentPolicy>(),
        };
        if !set_composition_attribute(hwnd, &mut data).as_bool() {
            return Err("Windows 拒绝应用窗口磨砂效果".to_string());
        }

        // DWM clips the acrylic backdrop. Other skins draw their own transparent
        // 15px corners; DEFAULT can restore an extra system outline after a switch.
        // Keep the window's region and style flags untouched.
        let corners = if enabled {
            DWMWCP_ROUND
        } else {
            DWMWCP_DONOTROUND
        };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corners as *const _ as *const c_void,
            size_of::<windows::Win32::Graphics::Dwm::DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
    Ok(())
}
