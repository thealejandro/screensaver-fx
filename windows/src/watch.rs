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

/// Por qué no toca lanzarlo ahora, si hay motivo: lo mismo que hace que Windows no lance el
/// protector, más el caso de que ya esté en marcha.
fn blocked() -> Option<String> {
    unsafe {
        let mut running: BOOL = 0;
        SystemParametersInfoW(SPI_GETSCREENSAVERRUNNING, 0, &mut running as *mut _ as *mut _, 0);
        if running != 0 {
            return Some("Windows ya tiene un protector en marcha".into());
        }
        // Vídeo, videollamadas, presentaciones: alguien pidió mantener la pantalla encendida.
        let mut state: u32 = 0;
        if CallNtPowerInformation(SystemExecutionState, core::ptr::null(), 0, &mut state as *mut _ as *mut _, 4) == 0 && state & ES_DISPLAY_REQUIRED != 0 {
            return Some("una app pide mantener la pantalla encendida (vídeo, llamada…)".into());
        }
        // Pantalla completa (juegos, D3D), modo presentación, sesión bloqueada o desconectada.
        let mut q = 0;
        if SHQueryUserNotificationState(&mut q) >= 0 && q != QUNS_ACCEPTS_NOTIFICATIONS && q != QUNS_QUIET_TIME {
            let why = match q {
                1 => "sesión bloqueada o desconectada",
                2 => "app a pantalla completa",
                3 => "juego a pantalla completa",
                4 => "modo presentación",
                7 => "app de la Tienda a pantalla completa",
                _ => "Windows no acepta interrupciones",
            };
            return Some(format!("{why} (estado {q})"));
        }
    }
    None
}

fn grace_ms() -> u32 {
    reg_sz(HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon", "ScreenSaverGracePeriod")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(5)
        .min(60)
        * 1000
}

/// Registro breve en %APPDATA%\ScreensaverFX\watch.log (rota a los 64 KB) para diagnosticar.
pub fn log(msg: &str) {
    use std::io::Write;
    let dir = crate::sys::data_dir();
    let path = dir.join("watch.log");
    if std::fs::metadata(&path).map(|m| m.len() > 64 * 1024).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("watch.old.log"));
    }
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let t = crate::sys::now();
        let _ = writeln!(f, "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {msg}", t.year, t.month, t.day, t.hour, t.minute, t.second);
    }
}

/// Lanza el protector y espera a que termine (o a que pidan parar). `false` si hay que salir.
fn launch(s: &Settings, quit: HANDLE) -> bool {
    let mut child = match Command::new(&s.exe).arg("/s").spawn() {
        Ok(c) => c,
        Err(e) => {
            log(&format!("no se pudo lanzar {}: {e}", s.exe));
            return true;
        }
    };
    log(&format!("lanzado {} (pid {})", s.exe, child.id()));
    let started = Instant::now();
    let handles = [quit, child.as_raw_handle() as HANDLE];
    let r = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), FALSE, INFINITE) };
    if r == WAIT_OBJECT_0 {
        // Actualizando o desinstalando: cierra también el protector para liberar el .scr.
        let _ = child.kill();
        log("detenido con el protector abierto");
        return false;
    }
    let secs = started.elapsed().as_secs();
    let code = child.try_wait().ok().flatten().and_then(|c| c.code()).unwrap_or(-1);
    log(&format!("protector cerrado tras {secs} s (código {code})"));
    if s.secure && started.elapsed().as_millis() as u32 >= grace_ms() {
        unsafe { LockWorkStation() };
        log("sesión bloqueada (Mostrar pantalla de inicio de sesión al reanudar)");
    }
    true
}

/// Cada cuánto vuelve a mirar mientras algo lo impide o tras un lanzamiento sin actividad nueva.
const RECHECK_MS: u32 = 15_000;

#[derive(Debug, PartialEq)]
enum Step {
    /// Esperar `ms`; `active`: el usuario acaba de usar el PC (para anotarlo una vez).
    Wait { ms: u32, active: bool },
    Blocked(String),
    Launch { idle_ms: u32 },
}

/// Decisión pura (sin efectos) para poder probarla: `now` e `input` son ticks de 32 bits que dan
/// la vuelta cada ~49 días; `fired`, la entrada que ya provocó un lanzamiento. `blocked` solo se
/// consulta cuando tocaría lanzar.
fn decide(now: u32, input: u32, timeout_ms: u32, fired: Option<u32>, blocked: impl FnOnce() -> Option<String>) -> Step {
    let idle = now.wrapping_sub(input);
    let due = timeout_ms.saturating_add(MARGIN_MS);
    if idle < due {
        return Step::Wait { ms: (due - idle).clamp(1_000, MAX_SLEEP_MS), active: idle < 2_000 };
    }
    if fired == Some(input) {
        return Step::Wait { ms: RECHECK_MS, active: false };
    }
    match blocked() {
        Some(why) => Step::Blocked(why),
        None => Step::Launch { idle_ms: idle },
    }
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
        if crate::update::all_users_install() {
            // La tarea de actualización (SYSTEM, otra sesión) no ve los nombres Local\: este
            // evento global le permite pedirnos salir antes de reemplazar el ejecutable.
            let g = wide(&crate::update::global_quit_name(crate::update::current_session()));
            let global = CreateEventW(core::ptr::null(), TRUE, FALSE, g.as_ptr());
            if !global.is_null() {
                ResetEvent(global);
                let (g, q) = (global as usize, quit as usize);
                let _ = std::thread::Builder::new().stack_size(64 * 1024).spawn(move || {
                    if WaitForSingleObject(g as HANDLE, INFINITE) == WAIT_OBJECT_0 {
                        log("actualización en curso: el vigilante se reinicia");
                        SetEvent(q as HANDLE);
                    }
                });
            }
        }
        crate::update::spawn_background(log);
        log(&format!("vigilante iniciado ({})", std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default()));
        // Devuelve al sistema lo que haya tocado el arranque: se queda en unos cientos de KB.
        SetProcessWorkingSetSizeEx(GetCurrentProcess(), usize::MAX, usize::MAX, 0);

        // Entrada que ya provocó un lanzamiento: no relanzar hasta que el usuario vuelva.
        let mut fired: Option<u32> = None;
        // Último estado anotado, para no repetir la misma línea cada 15 s.
        let mut noted = String::new();
        let note = |noted: &mut String, s: String| {
            if *noted != s {
                log(&s);
                *noted = s;
            }
        };
        loop {
            let wait = match settings() {
                None => {
                    note(&mut noted, "sin protector activo en Windows: nada que vigilar".into());
                    MAX_SLEEP_MS
                }
                Some(s) => {
                    let input = last_input();
                    match decide(GetTickCount(), input, s.timeout_ms, fired, blocked) {
                        Step::Wait { ms, active } => {
                            if active {
                                note(&mut noted, format!("vigilando {} tras {} s", s.exe, s.timeout_ms / 1000));
                            }
                            ms
                        }
                        Step::Blocked(why) => {
                            note(&mut noted, format!("no se lanza: {why}"));
                            RECHECK_MS
                        }
                        Step::Launch { idle_ms } => {
                            fired = Some(input);
                            note(&mut noted, format!("Windows no lo lanzó tras {} s de inactividad", idle_ms / 1000));
                            if !launch(&s, quit) {
                                return;
                            }
                            noted.clear();
                            SetProcessWorkingSetSizeEx(GetCurrentProcess(), usize::MAX, usize::MAX, 0);
                            continue;
                        }
                    }
                }
            };
            if WaitForSingleObject(quit, wait.clamp(1_000, MAX_SLEEP_MS)) == WAIT_OBJECT_0 {
                log("vigilante detenido");
                return;
            }
        }
    }
}

/// Devuelve al sistema la memoria que ya no se usa (tras lanzar el protector o descargar).
pub fn trim() {
    unsafe { SetProcessWorkingSetSizeEx(GetCurrentProcess(), usize::MAX, usize::MAX, 0) };
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

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u32 = 60_000;
    fn free() -> Option<String> {
        None
    }

    #[test]
    fn espera_hasta_el_tiempo_mas_margen() {
        // 10 s inactivo con 5 min: faltan 4:50 + margen, pero nunca más de un minuto de sueño.
        assert_eq!(decide(20_000, 10_000, 5 * MIN, None, free), Step::Wait { ms: MAX_SLEEP_MS, active: false });
        // Justo antes del margen: espera lo que falta.
        assert_eq!(decide(MIN + 4_000, 0, MIN, None, free), Step::Wait { ms: 1_000, active: false });
        // Recién usado: se anota como activo.
        assert_eq!(decide(1_000, 500, MIN, None, free), Step::Wait { ms: MAX_SLEEP_MS, active: true });
    }

    #[test]
    fn lanza_al_cumplirse() {
        assert_eq!(decide(MIN + MARGIN_MS, 0, MIN, None, free), Step::Launch { idle_ms: MIN + MARGIN_MS });
    }

    #[test]
    fn no_relanza_sin_actividad_nueva() {
        assert_eq!(decide(10 * MIN, 0, MIN, Some(0), free), Step::Wait { ms: RECHECK_MS, active: false });
        // Con actividad nueva (otra marca de entrada) sí vuelve a lanzar.
        assert!(matches!(decide(10 * MIN, 5, MIN, Some(0), free), Step::Launch { .. }));
    }

    #[test]
    fn respeta_los_bloqueos_y_solo_pregunta_cuando_toca() {
        let r = decide(10 * MIN, 0, MIN, None, || Some("vídeo".into()));
        assert_eq!(r, Step::Blocked("vídeo".into()));
        // Si aún no toca, no consulta el estado del sistema.
        let _ = decide(1_000, 0, MIN, None, || panic!("no debía consultarse"));
        let _ = decide(10 * MIN, 0, MIN, Some(0), || panic!("no debía consultarse"));
    }

    #[test]
    fn soporta_la_vuelta_del_contador() {
        // La última entrada fue justo antes de que GetTickCount diera la vuelta.
        let input = u32::MAX - 1_000;
        assert!(matches!(decide(MIN + MARGIN_MS, input, MIN, None, free), Step::Launch { .. }));
        assert!(matches!(decide(500, input, MIN, None, free), Step::Wait { active: true, .. }));
    }

    #[test]
    fn tiempos_enormes_no_desbordan() {
        assert!(matches!(decide(0, 0, u32::MAX, None, free), Step::Wait { .. }));
    }
}
