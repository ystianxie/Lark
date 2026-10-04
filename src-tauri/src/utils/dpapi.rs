//! Small Windows DPAPI wrapper for protecting secrets at rest.

#[cfg(windows)]
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[cfg(windows)]
use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};
#[cfg(windows)]
use windows::Win32::Foundation::{HLOCAL, LocalFree};

#[cfg(windows)]
pub fn protect(value: &str) -> Result<String, String> {
    let input = value.as_bytes();
    let input_blob = CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe { CryptProtectData(&input_blob, None, None, None, None, 0, &mut output) }
        .map_err(|e| format!("DPAPI 加密失败：{e}"))?;
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { let _ = LocalFree(HLOCAL(output.pbData.cast())); }
    Ok(STANDARD.encode(bytes))
}

#[cfg(windows)]
pub fn unprotect(value: &str) -> Result<String, String> {
    let input = STANDARD.decode(value).map_err(|e| format!("DPAPI 密文格式无效：{e}"))?;
    let input_blob = CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe { CryptUnprotectData(&input_blob, None, None, None, None, 0, &mut output) }
        .map_err(|e| format!("DPAPI 解密失败：{e}"))?;
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { let _ = LocalFree(HLOCAL(output.pbData.cast())); }
    String::from_utf8(bytes).map_err(|e| format!("DPAPI 明文不是有效 UTF-8：{e}"))
}

#[cfg(not(windows))]
pub fn protect(_: &str) -> Result<String, String> { Err("DPAPI 仅支持 Windows".into()) }
#[cfg(not(windows))]
pub fn unprotect(_: &str) -> Result<String, String> { Err("DPAPI 仅支持 Windows".into()) }
