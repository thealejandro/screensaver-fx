//! Actualizaciones automáticas desde GitHub Releases.
//!
//! - Instalación «solo para mí»: el propio vigilante comprueba a los pocos minutos de iniciar
//!   sesión y luego cada 24 h; si hay versión nueva lanza el instalador en silencio (que detiene
//!   este vigilante y arranca el nuevo).
//! - Instalación «para todos»: hace falta ser administrador, así que el instalador registra la
//!   tarea programada «ScreensaverFX\Update», que ejecuta `/update` como SYSTEM al arrancar y una
//!   vez al día. Detiene los vigilantes de cada sesión, actualiza y los vuelve a lanzar.
//!
//! Nunca se instala con el protector o la configuración abiertos (`Global\ScreensaverFX.Busy`).
//! El instalador se descarga por HTTPS y se comprueba con la suma SHA-256 publicada en la release.

use crate::sys::wide;
use std::path::PathBuf;
use std::process::Command;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Networking::WinHttp::*;
use windows_sys::Win32::Security::Cryptography::{BCryptHash, BCRYPT_SHA256_ALG_HANDLE};
use windows_sys::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows_sys::Win32::System::RemoteDesktop::{ProcessIdToSessionId, WTSEnumerateSessionsW, WTSFreeMemory, WTSQueryUserToken, WTS_SESSION_INFOW};
use windows_sys::Win32::System::Threading::*;

/// Versión de este ejecutable: la de la etiqueta en el CI (SSFX_VERSION) o la de Cargo.toml.
pub const VERSION: &str = match option_env!("SSFX_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};
const RELEASES: &str = "https://github.com/thealejandro/screensaver-fx/releases";
const BUSY: &str = "Global\\ScreensaverFX.Busy";
const FIRST_CHECK_SECS: u64 = 3 * 60;
const EVERY_SECS: u64 = 24 * 60 * 60;

pub type Version = (u32, u32, u32);

pub fn parse_version(v: &str) -> Option<Version> {
    let mut it = v.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u32>().ok());
    let r = (it.next()??, it.next()??, it.next()??);
    it.next().is_none().then_some(r)
}

/// La versión más reciente del instalador que lista SHA256SUMS.txt, con su suma.
pub fn newest(sums: &str) -> Option<(Version, String, String)> {
    sums.lines()
        .filter_map(|l| {
            let (hash, name) = l.trim().split_once(char::is_whitespace)?;
            let name = name.trim().trim_start_matches('*');
            let ver = name.strip_prefix("ScreensaverFX-Setup-")?.strip_suffix(".exe")?;
            let v = parse_version(ver)?;
            (hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| (v, ver.to_string(), hash.to_ascii_lowercase()))
        })
        .max_by_key(|x| x.0)
}

fn exe() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn under(env: &str) -> bool {
    std::env::var_os(env).is_some_and(|d| !d.is_empty() && exe().starts_with(PathBuf::from(d)))
}

/// Instalado para todos (Program Files). Una copia suelta (p. ej. target\release) no cuenta.
pub fn all_users_install() -> bool {
    installed() && (under("ProgramFiles") || under("ProgramW6432") || under("ProgramFiles(x86)"))
}

fn installed() -> bool {
    exe().with_file_name("unins000.exe").is_file()
}

/// Marca «hay una ventana del protector abierta» mientras viva el proceso (protector, vista
/// previa o configuración): con ella, la actualización espera.
pub fn hold_busy() {
    let n = wide(BUSY);
    unsafe { CreateEventW(core::ptr::null(), TRUE, FALSE, n.as_ptr()) };
}

fn busy() -> bool {
    let n = wide(BUSY);
    let h = unsafe { OpenEventW(0x0010_0000 /* SYNCHRONIZE */, FALSE, n.as_ptr()) };
    if h.is_null() {
        return false;
    }
    unsafe { CloseHandle(h) };
    true
}

/// Nombre del evento con el que la tarea de SYSTEM pide salir al vigilante de una sesión.
pub fn global_quit_name(session: u32) -> String {
    format!("Global\\ScreensaverFX.Watch.Quit.{session}")
}

pub fn current_session() -> u32 {
    let mut s = 0;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut s) };
    s
}

fn base() -> String {
    // Solo para pruebas: un servidor local en lugar de GitHub.
    std::env::var("SSFX_UPDATE_BASE").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| RELEASES.into()).trim_end_matches('/').to_string()
}

struct Handle(*mut core::ffi::c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

/// GET con WinHTTP (proxy del sistema, sigue redirecciones). http solo hacia localhost.
fn http_get(url: &str, max: usize) -> Result<Vec<u8>, String> {
    let (secure, rest) = match (url.strip_prefix("https://"), url.strip_prefix("http://")) {
        (Some(r), _) => (true, r),
        (_, Some(r)) => (false, r),
        _ => return Err(format!("URL no válida: {url}")),
    };
    let (hostport, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, "/".into()),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| format!("puerto no válido: {url}"))?),
        None => (hostport, if secure { 443 } else { 80 }),
    };
    if !secure && host != "localhost" && host != "127.0.0.1" {
        return Err("solo se descarga por https".into());
    }
    let fail = |what: &str| format!("{what} {url}: error {}", unsafe { GetLastError() });
    unsafe {
        let ua = wide(&format!("ScreensaverFX/{VERSION}"));
        let mut s = Handle(WinHttpOpen(ua.as_ptr(), 4 /* AUTOMATIC_PROXY */, core::ptr::null(), core::ptr::null(), 0));
        if s.0.is_null() {
            s = Handle(WinHttpOpen(ua.as_ptr(), 0 /* DEFAULT_PROXY */, core::ptr::null(), core::ptr::null(), 0));
        }
        if s.0.is_null() {
            return Err(fail("WinHttpOpen"));
        }
        WinHttpSetTimeouts(s.0, 15_000, 15_000, 30_000, 60_000);
        let h = wide(host);
        let c = Handle(WinHttpConnect(s.0, h.as_ptr(), port, 0));
        if c.0.is_null() {
            return Err(fail("conexión con"));
        }
        let (verb, p) = (wide("GET"), wide(&path));
        let r = Handle(WinHttpOpenRequest(c.0, verb.as_ptr(), p.as_ptr(), core::ptr::null(), core::ptr::null(), core::ptr::null(), if secure { WINHTTP_FLAG_SECURE } else { 0 }));
        if r.0.is_null() || WinHttpSendRequest(r.0, core::ptr::null(), 0, core::ptr::null(), 0, 0, 0) == 0 || WinHttpReceiveResponse(r.0, core::ptr::null_mut()) == 0 {
            return Err(fail("petición a"));
        }
        let (mut code, mut len) = (0u32, 4u32);
        WinHttpQueryHeaders(r.0, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, core::ptr::null(), &mut code as *mut u32 as *mut _, &mut len, core::ptr::null_mut());
        if code != 200 {
            return Err(format!("HTTP {code} en {url}"));
        }
        let mut out = Vec::new();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let mut n = 0u32;
            if WinHttpReadData(r.0, buf.as_mut_ptr() as *mut _, buf.len() as u32, &mut n) == 0 {
                return Err(fail("lectura de"));
            }
            if n == 0 {
                return Ok(out);
            }
            if out.len() + n as usize > max {
                return Err(format!("respuesta demasiado grande en {url}"));
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut out = [0u8; 32];
    let st = unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, core::ptr::null(), 0, data.as_ptr(), data.len() as u32, out.as_mut_ptr(), 32) };
    if st != 0 {
        return String::new();
    }
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// Sesiones con usuario donde hay un vigilante de esta instalación: les pide salir y espera.
fn stop_watchers() -> Vec<u32> {
    let mut stopped = Vec::new();
    unsafe {
        let (mut p, mut n): (*mut WTS_SESSION_INFOW, u32) = (core::ptr::null_mut(), 0);
        if WTSEnumerateSessionsW(core::ptr::null_mut(), 0, 1, &mut p, &mut n) == 0 {
            return stopped;
        }
        let ids: Vec<u32> = std::slice::from_raw_parts(p, n as usize).iter().map(|s| s.SessionId).collect();
        WTSFreeMemory(p as *mut _);
        for id in ids {
            let name = wide(&global_quit_name(id));
            let h = OpenEventW(EVENT_MODIFY_STATE, FALSE, name.as_ptr());
            if !h.is_null() {
                SetEvent(h);
                CloseHandle(h);
                stopped.push(id);
            }
        }
        // El evento desaparece cuando su vigilante termina.
        for _ in 0..50 {
            let alive = stopped.iter().any(|&id| {
                let name = wide(&global_quit_name(id));
                let h = OpenEventW(0x0010_0000 /* SYNCHRONIZE */, FALSE, name.as_ptr());
                !h.is_null() && CloseHandle(h) != 0
            });
            if !alive {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    stopped
}

/// Vuelve a lanzar el vigilante en cada sesión, como su usuario (desde SYSTEM).
fn relaunch_watchers(sessions: &[u32]) -> usize {
    let mut ok = 0;
    for &id in sessions {
        unsafe {
            let mut tok: HANDLE = core::ptr::null_mut();
            if WTSQueryUserToken(id, &mut tok) == 0 {
                continue;
            }
            let mut env: *mut core::ffi::c_void = core::ptr::null_mut();
            let have_env = CreateEnvironmentBlock(&mut env, tok, FALSE) != 0;
            let mut desktop = wide("winsta0\\default");
            let si = STARTUPINFOW { cb: core::mem::size_of::<STARTUPINFOW>() as u32, lpDesktop: desktop.as_mut_ptr(), ..core::mem::zeroed() };
            let mut pi: PROCESS_INFORMATION = core::mem::zeroed();
            let mut cmd = wide(&format!("\"{}\" /watch", exe().display()));
            let flags = CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW;
            if CreateProcessAsUserW(tok, core::ptr::null(), cmd.as_mut_ptr(), core::ptr::null(), core::ptr::null(), FALSE, flags, if have_env { env } else { core::ptr::null() }, core::ptr::null(), &si, &mut pi) != 0 {
                CloseHandle(pi.hProcess);
                CloseHandle(pi.hThread);
                ok += 1;
            }
            if have_env {
                DestroyEnvironmentBlock(env);
            }
            CloseHandle(tok);
        }
    }
    ok
}

/// Comprueba e instala si hay algo nuevo. `system`: ejecutado por la tarea programada.
fn update(system: bool) -> Result<String, String> {
    let Some(cur) = parse_version(VERSION).filter(|&v| v != (0, 0, 0)) else {
        return Ok(format!("compilación de desarrollo ({VERSION}): sin actualizaciones"));
    };
    let sums = http_get(&format!("{}/latest/download/SHA256SUMS.txt", base()), 64 * 1024)?;
    let (v, ver, hash) = newest(&String::from_utf8_lossy(&sums)).ok_or("SHA256SUMS.txt sin instalador")?;
    if v <= cur {
        return Ok(format!("al día ({VERSION})"));
    }
    if busy() {
        return Err(format!("{ver} disponible, pero el protector o la configuración están abiertos: se reintentará"));
    }
    let data = http_get(&format!("{}/download/v{ver}/ScreensaverFX-Setup-{ver}.exe", base()), 64 << 20)?;
    if sha256_hex(&data) != hash {
        return Err(format!("el instalador {ver} no coincide con su SHA-256: descartado"));
    }
    // Para todos: en la carpeta del programa (solo la escriben administradores).
    let dir = if system { exe().with_file_name("update") } else { std::env::temp_dir().join("ScreensaverFX-update") };
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let setup = dir.join(format!("ScreensaverFX-Setup-{ver}.exe"));
    std::fs::write(&setup, &data).map_err(|e| format!("{}: {e}", setup.display()))?;
    drop(data);
    let log = format!("/LOG={}", dir.join("setup.log").display());
    let mode = if system { "/ALLUSERS" } else { "/CURRENTUSER" };
    let args = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/AUTOUPDATE", mode, &log];
    if !system {
        // El instalador detiene este vigilante y arranca el nuevo.
        Command::new(&setup).args(args).spawn().map_err(|e| format!("no se pudo lanzar el instalador: {e}"))?;
        return Ok(format!("instalando {ver} (desde {VERSION})"));
    }
    let sessions = stop_watchers();
    let status = Command::new(&setup).args(args).status();
    let relaunched = relaunch_watchers(&sessions);
    let _ = std::fs::remove_file(&setup);
    match status {
        Ok(s) if s.success() => Ok(format!("actualizado {VERSION} → {ver}; vigilantes relanzados: {relaunched}/{}", sessions.len())),
        Ok(s) => Err(format!("el instalador de {ver} terminó con código {:?} (ver {})", s.code(), dir.join("setup.log").display())),
        Err(e) => Err(format!("no se pudo ejecutar el instalador de {ver}: {e}")),
    }
}

fn system_log(msg: &str) {
    use std::io::Write;
    let dir = PathBuf::from(std::env::var_os("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into())).join("ScreensaverFX");
    let path = dir.join("update.log");
    if std::fs::metadata(&path).map(|m| m.len() > 64 * 1024).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("update.old.log"));
    }
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let t = crate::sys::now();
        let _ = writeln!(f, "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {msg}", t.year, t.month, t.day, t.hour, t.minute, t.second);
    }
}

/// `/update`: lo que ejecuta la tarea programada (o a mano, en cualquier instalación).
pub fn run_cli() {
    let system = all_users_install();
    let log: fn(&str) = if system { system_log } else { crate::watch::log };
    log(&match update(system) {
        Ok(m) => format!("actualizaciones: {m}"),
        Err(e) => format!("actualizaciones: {e}"),
    });
}

/// En el vigilante de una instalación «solo para mí»: comprobación periódica en segundo plano.
pub fn spawn_background(log: fn(&str)) {
    if !installed() || all_users_install() {
        return;
    }
    let _ = std::thread::Builder::new().stack_size(256 * 1024).spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(FIRST_CHECK_SECS));
        loop {
            log(&match update(false) {
                Ok(m) => format!("actualizaciones: {m}"),
                Err(e) => format!("actualizaciones: {e}"),
            });
            crate::watch::trim();
            std::thread::sleep(std::time::Duration::from_secs(EVERY_SECS));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("0.1.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("v1.20.3"), Some((1, 20, 3)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("1.x.3"), None);
        assert!(parse_version("0.10.0") > parse_version("0.9.9"));
    }

    #[test]
    fn newest_from_sums() {
        let h = |c: char| c.to_string().repeat(64);
        let sums = format!(
            "{}  ScreensaverFX-0.2.0.pkg\n{}  ScreensaverFX-Setup-0.1.9.exe\n{}  ScreensaverFX-Setup-0.10.0.exe\n{}  ScreensaverFX-Setup.exe\n{} *ScreensaverFX-Setup-0.2.0.exe\r\nbasura\n",
            h('a'),
            h('b'),
            h('C'),
            h('d'),
            h('e')
        );
        let (v, ver, hash) = newest(&sums).unwrap();
        assert_eq!((v, ver.as_str(), hash), ((0, 10, 0), "0.10.0", h('c')));
        assert!(newest("zz  ScreensaverFX-Setup-1.0.0.exe\n").is_none());
        assert!(newest("").is_none());
    }

    #[test]
    fn sha256_known() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
