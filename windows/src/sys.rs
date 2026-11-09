//! Datos del sistema: carpeta de configuración, color de acento, idioma y hora local.

use ssfx_core::WallTime;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, SYSTEMTIME};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
use windows_sys::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join("ScreensaverFX")
}

/// `SSFX_CONFIG` permite que el instalador muestre una vista previa con una configuración temporal.
pub fn ini_path() -> PathBuf {
    match std::env::var_os("SSFX_CONFIG") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => data_dir().join("config.ini"),
    }
}

pub fn art_path() -> PathBuf {
    data_dir().join("screensaver.txt")
}

pub fn read_ini() -> String {
    let s = std::fs::read_to_string(ini_path()).unwrap_or_default();
    s.strip_prefix('\u{feff}').map(str::to_owned).unwrap_or(s)
}

pub fn read_art() -> Option<String> {
    std::fs::read_to_string(art_path()).ok()
}

/// Color de acento de Windows como 0x00RRGGBB.
pub fn accent() -> u32 {
    unsafe {
        let (sub, name) = (wide("Software\\Microsoft\\Windows\\DWM"), wide("AccentColor"));
        let (mut v, mut size) = (0u32, 4u32);
        let r = RegGetValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr(), RRF_RT_REG_DWORD, core::ptr::null_mut(), &mut v as *mut u32 as *mut _, &mut size);
        if r == ERROR_SUCCESS {
            // El registro lo guarda como 0xAABBGGRR.
            return (v & 0xFF) << 16 | (v >> 8 & 0xFF) << 8 | (v >> 16 & 0xFF);
        }
        let (mut c, mut opaque) = (0u32, 0);
        if DwmGetColorizationColor(&mut c, &mut opaque) >= 0 {
            return c & 0x00FF_FFFF;
        }
    }
    0x0078D4
}

pub fn system_is_spanish() -> bool {
    unsafe { GetUserDefaultUILanguage() & 0x3FF == 0x0A }
}

pub fn now() -> WallTime {
    let mut st: SYSTEMTIME = unsafe { core::mem::zeroed() };
    unsafe { GetLocalTime(&mut st) };
    WallTime {
        year: st.wYear as i32,
        month: st.wMonth as i32,
        day: st.wDay as i32,
        weekday: (st.wDayOfWeek as i32 + 6) % 7, // Windows: 0 = domingo
        hour: st.wHour as i32,
        minute: st.wMinute as i32,
        second: st.wSecond as i32,
    }
}

pub fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x5EED) ^ std::process::id() as u64
}

/// Ruta larga (Windows guarda a veces el protector como C:\WINDOWS\system32\SCREEN~1.SCR).
pub fn long_path(p: &str) -> String {
    let w = wide(p);
    let mut buf = vec![0u16; 1024];
    let n = unsafe { windows_sys::Win32::Storage::FileSystem::GetLongPathNameW(w.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n >= buf.len() {
        return p.to_string();
    }
    String::from_utf16_lossy(&buf[..n])
}

/// Mismo archivo, sin importar mayúsculas ni nombres cortos 8.3.
pub fn same_path(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || long_path(a).eq_ignore_ascii_case(&long_path(b))
}
