//! Vigilante (/watch): arranca el protector que Windows tenga configurado cuando el sistema
//! debió hacerlo y no lo hizo, por ejemplo porque una app en primer plano se traga
//! `SC_SCREENSAVE`. Lee siempre los ajustes del sistema (también las directivas de grupo) y
//! respeta lo que Windows respeta: vídeo o presentaciones activas, juegos a pantalla completa
//! y sesión bloqueada. Si está marcado «Mostrar la pantalla de inicio de sesión», bloquea al volver.
//!
//! Casi no consume: duerme hasta el momento en que tocaría el protector (como mucho un minuto)
//! y no tiene ventanas. `/unwatch` lo detiene (lo usa el instalador al actualizar o desinstalar).

use crate::sys::wide;
use std::os::windows::io::AsRawHandle;
use std::process::Command;
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx;
use windows_sys::Win32::System::Power::{CallNtPowerInformation, SystemExecutionState, ES_DISPLAY_REQUIRED};
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Shutdown::LockWorkStation;
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_ACCEPTS_NOTIFICATIONS, QUNS_QUIET_TIME};
use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETSCREENSAVEACTIVE, SPI_GETSCREENSAVERRUNNING};

const QUIT_EVENT: &str = "Local\\ScreensaverFX.Watch.Quit";
const MUTEX: &str = "Local\\ScreensaverFX.Watch";
/// Margen tras el tiempo de espera para dejar que Windows lo lance primero.
const MARGIN_MS: u32 = 5_000;
/// Cada cuánto vuelve a mirar como mucho (ajustes cambiados, vídeo que terminó…).
const MAX_SLEEP_MS: u32 = 60_000;

const POLICY: &str = "Software\\Policies\\Microsoft\\Windows\\Control Panel\\Desktop";
const DESKTOP: &str = "Control Panel\\Desktop";

fn reg_sz(root: HKEY, sub: &str, name: &str) -> Option<String> {
    let (sub, name) = (wide(sub), wide(name));
    let mut buf = [0u16; 520];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe { RegGetValueW(root, sub.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ, core::ptr::null_mut(), buf.as_mut_ptr() as *mut _, &mut size) };
    if r != ERROR_SUCCESS {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..n]).trim().to_string())
}

/// Directiva de grupo primero; si no hay, el ajuste del usuario.
fn desktop(name: &str) -> Option<String> {
    reg_sz(HKEY_CURRENT_USER, POLICY, name).or_else(|| reg_sz(HKEY_CURRENT_USER, DESKTOP, name))
}

fn expand(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let src = wide(s);
    let mut buf = [0u16; 1024];
    let n = unsafe { ExpandEnvironmentStringsW(src.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n > buf.len() {
        return s.to_string();
    }
    String::from_utf16_lossy(&buf[..n - 1])
}

struct Settings {
    exe: String,
    timeout_ms: u32,
    secure: bool,
}

fn settings() -> Option<Settings> {
    let active = match reg_sz(HKEY_CURRENT_USER, POLICY, "ScreenSaveActive") {
        Some(v) => v == "1",
        None => {
            let mut on: BOOL = 0;
            unsafe { SystemParametersInfoW(SPI_GETSCREENSAVEACTIVE, 0, &mut on as *mut _ as *mut _, 0) };
            on != 0
        }
    };
    let exe = expand(&desktop("SCRNSAVE.EXE")?);
    let secs: u32 = desktop("ScreenSaveTimeOut")?.parse().ok()?;
    if !active || exe.is_empty() || secs == 0 || !std::path::Path::new(&exe).is_file() {
        return None;
    }
    let secure = desktop("ScreenSaverIsSecure").as_deref() == Some("1");
    Some(Settings { exe, timeout_ms: secs.saturating_mul(1000), secure })
}

/// Tick de la última entrada del usuario (teclado, ratón, táctil) en esta sesión.
fn last_input() -> u32 {
    let mut li = LASTINPUTINFO { cbSize: core::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe { GetLastInputInfo(&mut li) };
    li.dwTime
}

/// Lo mismo que hace que Windows no lance el protector, más el caso de que ya esté en marcha.
fn system_allows() -> bool {
    unsafe {
        let mut running: BOOL = 0;
        SystemParametersInfoW(SPI_GETSCREENSAVERRUNNING, 0, &mut running as *mut _ as *mut _, 0);
        if running != 0 {
            return false;
        }
        // Vídeo, videollamadas, presentaciones: alguien pidió mantener la pantalla encendida.
        let mut state: u32 = 0;
        if CallNtPowerInformation(SystemExecutionState, core::ptr::null(), 0, &mut state as *mut _ as *mut _, 4) == 0 && state & ES_DISPLAY_REQUIRED != 0 {
            return false;
        }
        // Pantalla completa (juegos, D3D), modo presentación, sesión bloqueada o desconectada.
        let mut q = 0;
        if SHQueryUserNotificationState(&mut q) >= 0 && q != QUNS_ACCEPTS_NOTIFICATIONS && q != QUNS_QUIET_TIME {
            return false;
        }
    }
    true
}

fn grace_ms() -> u32 {
    reg_sz(HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon", "ScreenSaverGracePeriod")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(5)
        .min(60)
        * 1000
}

/// Lanza el protector y espera a que termine (o a que pidan parar). `false` si hay que salir.
fn launch(s: &Settings, quit: HANDLE) -> bool {
    let Ok(mut child) = Command::new(&s.exe).arg("/s").spawn() else { return true };
    let started = Instant::now();
    let handles = [quit, child.as_raw_handle() as HANDLE];
    let r = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), FALSE, INFINITE) };
    if r == WAIT_OBJECT_0 {
        // Actualizando o desinstalando: cierra también el protector para liberar el .scr.
        let _ = child.kill();
        return false;
    }
    if s.secure && started.elapsed().as_millis() as u32 >= grace_ms() {
        unsafe { LockWorkStation() };
    }
    true
}

pub fn run() {
    unsafe {
        let name = wide(MUTEX);
        let _mutex = CreateMutexW(core::ptr::null(), FALSE, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return; // ya hay uno en esta sesión
        }
        let ev = wide(QUIT_EVENT);
        let quit = CreateEventW(core::ptr::null(), TRUE, FALSE, ev.as_ptr());
        if quit.is_null() {
            return;
        }
        ResetEvent(quit); // por si quedó señalado de un /unwatch anterior
        // Devuelve al sistema lo que haya tocado el arranque: se queda en unos cientos de KB.
        SetProcessWorkingSetSizeEx(GetCurrentProcess(), usize::MAX, usize::MAX, 0);

        // Entrada que ya provocó un lanzamiento: no relanzar hasta que el usuario vuelva.
        let mut fired: Option<u32> = None;
        loop {
            let wait = match settings() {
                None => MAX_SLEEP_MS,
                Some(s) => {
                    let input = last_input();
                    let idle = GetTickCount().wrapping_sub(input);
                    let due = s.timeout_ms.saturating_add(MARGIN_MS);
                    if idle < due {
                        due - idle
                    } else if fired == Some(input) || !system_allows() {
                        MAX_SLEEP_MS.min(15_000)
                    } else {
                        fired = Some(input);
                        if !launch(&s, quit) {
                            return;
                        }
                        SetProcessWorkingSetSizeEx(GetCurrentProcess(), usize::MAX, usize::MAX, 0);
                        continue;
                    }
                }
            };
            if WaitForSingleObject(quit, wait.clamp(1_000, MAX_SLEEP_MS)) == WAIT_OBJECT_0 {
                return;
            }
        }
    }
}

/// Pide al vigilante de esta sesión que termine.
pub fn stop() {
    unsafe {
        let ev = wide(QUIT_EVENT);
        let h = OpenEventW(EVENT_MODIFY_STATE, FALSE, ev.as_ptr());
        if !h.is_null() {
            SetEvent(h);
            CloseHandle(h);
        }
    }
}
