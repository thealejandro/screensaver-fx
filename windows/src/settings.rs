//! Ventana de configuración (/c), con el mismo diseño que la página del instalador: cabecera,
//! opciones a la izquierda y vista previa en vivo a la derecha. Sigue el tema claro u oscuro de
//! Windows y su color de acento, con tarjetas y controles al estilo de Windows 11 dibujados con
//! GDI. Guarda config.ini y, si se activa, lo deja como protector de pantalla del sistema.

use crate::sys::{self, wide};
use crate::update;
use ssfx_core::art::{default_text, MAX_SUBTITLE, MAX_TITLE};
use ssfx_core::color::THEMES;
use ssfx_core::config::FONTS;
use ssfx_core::fx::EFFECTS;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::Controls::{SetWindowTheme, DRAWITEMSTRUCT};
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetFocus, IsWindowEnabled, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ID_BANNER: i32 = 101;
const ID_TITLE: i32 = 102;
const ID_SUBTITLE: i32 = 103;
const ID_EDIT_ART: i32 = 104;
const ID_THEME: i32 = 105;
const ID_SIZE: i32 = 106;
const ID_EFFECTS: i32 = 107;
const ID_LANG: i32 = 108;
const ID_TEST: i32 = 109;
const ID_ACTIVATE: i32 = 110;
const ID_ACTIVATE_LABEL: i32 = 111;
const ID_MINUTES: i32 = 112;
const ID_PREVIEW: i32 = 113;
const ID_LBL_TITLE: i32 = 120;
const ID_LBL_SUBTITLE: i32 = 121;
const ID_LBL_AFTER: i32 = 122;
const ID_LBL_MINUTES: i32 = 123;
const ID_UPDATE: i32 = 124;
const ID_SCREENS: i32 = 125;
const ID_IDENTIFY: i32 = 126;
const ID_FONT: i32 = 127;
const ID_CALM: i32 = 128;
const ID_LBL_FONT: i32 = 129;

const TIMER_PREVIEW: usize = 1;
const TIMER_IDENTIFY: usize = 2;
/// Cuánto se ven los números de «Identificar pantallas».
const IDENTIFY_MS: u32 = 3000;
/// Resultado del hilo que busca o descarga actualizaciones (LPARAM: `Box<Done>`).
const WM_UPDATE_DONE: u32 = WM_APP + 1;
/// Botón de actualizaciones en la cabecera.
const UPD_W: i32 = 200;
const UPD_Y: i32 = 18;
/// Igual que el instalador: espera a que se deje de escribir antes de reiniciar la vista previa.
const PREVIEW_DELAY_MS: u32 = 350;

const BANNERS: [&str; 3] = ["default", "text", "file"];
const SIZES: [&str; 3] = ["s", "m", "l"];
const LANGS: [&str; 3] = ["auto", "es", "en"];
const SCREENS: [&str; 3] = ["span", "each", "main"];
const CALMS: [&str; 2] = ["0", "1"];
const DESKTOP: &str = "Control Panel\\Desktop";

// Medidas en píxeles lógicos (96 ppp).
const W: i32 = 780;
const H: i32 = 684;
const PAD: i32 = 24;
const GAP: i32 = 12;
/// Margen interior de las tarjetas.
const CP: i32 = 16;
/// Alto de los campos, botones y desplegables.
const CTL_H: i32 = 32;
/// De una etiqueta a su campo, y de un campo al siguiente.
const LABEL_H: i32 = 20;
const STEP: i32 = 62;
const CARD_Y: i32 = 84;
const CARD_H: i32 = CP + 6 * STEP + LABEL_H + CTL_H + CP;
const LEFT_W: i32 = 340;
const RIGHT_X: i32 = PAD + LEFT_W + GAP;
const RIGHT_W: i32 = W - PAD - RIGHT_X;
/// Grosor del marco de la vista previa (como el bisel de un monitor).
const BEZEL: i32 = 6;
const ACT_Y: i32 = CARD_Y + CARD_H + GAP;
const ACT_H: i32 = 64;
const BUTTONS_Y: i32 = ACT_Y + ACT_H + 16;
/// Interruptor: la píldora de 40×20 más el hueco del anillo de foco.
const TOGGLE_W: i32 = 46;
const TOGGLE_H: i32 = 26;

const WM_MOUSELEAVE: u32 = 0x02A3;
const ODS_SELECTED: u32 = 0x1;
const ODS_FOCUS: u32 = 0x10;
const ODS_NOFOCUSRECT: u32 = 0x200;
const KIND_BUTTON: usize = 0;
const KIND_COMBO: usize = 1;

#[derive(Clone, Copy)]
struct Palette {
    dark: bool,
    bg: COLORREF,
    card: COLORREF,
    card_line: COLORREF,
    field: COLORREF,
    field_hot: COLORREF,
    field_press: COLORREF,
    field_line: COLORREF,
    field_off: COLORREF,
    field_off_line: COLORREF,
    text: COLORREF,
    dim: COLORREF,
    off: COLORREF,
}

const fn rgb(c: u32) -> COLORREF {
    (c >> 16 & 0xFF) | (c & 0xFF00) | (c & 0xFF) << 16
}

const DARK: Palette = Palette {
    dark: true,
    bg: rgb(0x202020),
    card: rgb(0x2B2B2B),
    card_line: rgb(0x353535),
    field: rgb(0x373737),
    field_hot: rgb(0x3F3F3F),
    field_press: rgb(0x323232),
    field_line: rgb(0x4A4A4A),
    field_off: rgb(0x2F2F2F),
    field_off_line: rgb(0x393939),
    text: rgb(0xFFFFFF),
    dim: rgb(0xB8B8B8),
    off: rgb(0x7A7A7A),
};
const LIGHT: Palette = Palette {
    dark: false,
    bg: rgb(0xF3F3F3),
    card: rgb(0xFBFBFB),
    card_line: rgb(0xE5E5E5),
    field: rgb(0xFFFFFF),
    field_hot: rgb(0xF5F5F5),
    field_press: rgb(0xEEEEEE),
    field_line: rgb(0xD2D2D2),
    field_off: rgb(0xF6F6F6),
    field_off_line: rgb(0xE6E6E6),
    text: rgb(0x1B1B1B),
    dim: rgb(0x5F5F5F),
    off: rgb(0xA3A3A3),
};

struct Ui {
    pal: Palette,
    s: f32,
    accent: COLORREF,
    on_accent: COLORREF,
    font: HFONT,
    font_small: HFONT,
    font_title: HFONT,
    bg: HBRUSH,
    card: HBRUSH,
    field: HBRUSH,
    field_off: HBRUSH,
    icon: HICON,
    /// Marcos redondeados (en píxeles reales) que la ventana pinta alrededor de cada EDIT.
    frames: Vec<(i32, RECT)>,
    bezel: RECT,
    preview_host: HWND,
    preview: Option<Child>,
    preview_ini: PathBuf,
    scr: String,
}

impl Ui {
    fn px(&self, v: i32) -> i32 {
        (v as f32 * self.s).round() as i32
    }
}

thread_local! {
    static ES: Cell<bool> = const { Cell::new(true) };
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    /// Control bajo el ratón (para el resaltado) y estado del interruptor.
    static HOT: Cell<HWND> = const { Cell::new(core::ptr::null_mut()) };
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    /// Guardado sin cambios desde entonces (muestra «Cambios guardados» y «Cerrar»).
    static SAVED: Cell<bool> = const { Cell::new(false) };
    /// Hay cambios sin guardar.
    static DIRTY: Cell<bool> = const { Cell::new(false) };
    /// Aviso sobre la configuración leída (p. ej. un tema que no existe); se borra al guardar.
    static NOTE: RefCell<String> = const { RefCell::new(String::new()) };
    static UPD: RefCell<Upd> = RefCell::new(Upd::default());
}

/// Lo que muestra la zona de actualizaciones de la cabecera.
#[derive(Default)]
struct Upd {
    status: String,
    /// Mientras se busca o se descarga, el botón queda deshabilitado.
    busy: bool,
    /// Versión nueva disponible y la suma SHA-256 de su instalador.
    new: Option<(String, String)>,
}

enum Done {
    Checked(Result<update::Latest, String>),
    Downloaded(String, Result<PathBuf, String>),
}

unsafe fn set_upd(hwnd: HWND, status: String, busy: bool, new: Option<(String, String)>) {
    let label = match &new {
        Some((v, _)) if update::is_installed() => format!("{} {v}", tr("Actualizar a", "Update to")),
        Some(_) => tr("Abrir la descarga", "Open the download").to_string(),
        None => tr("Buscar actualizaciones", "Check for updates").to_string(),
    };
    let b = item(hwnd, ID_UPDATE);
    let t = wide(&label);
    SetWindowTextW(b, t.as_ptr());
    EnableWindow(b, (!busy) as BOOL);
    UPD.with(|u| *u.borrow_mut() = Upd { status, busy, new });
    InvalidateRect(hwnd, core::ptr::null(), FALSE);
}

/// Ejecuta `job` en otro hilo y entrega su resultado a la ventana como WM_UPDATE_DONE.
fn in_background(hwnd: HWND, job: impl FnOnce() -> Done + Send + 'static) {
    let h = hwnd as usize;
    let _ = std::thread::Builder::new().stack_size(256 * 1024).spawn(move || {
        let p = Box::into_raw(Box::new(job()));
        if unsafe { PostMessageW(h as HWND, WM_UPDATE_DONE, 0, p as LPARAM) } == 0 {
            // La ventana ya no existe.
            drop(unsafe { Box::from_raw(p) });
        }
    });
}

unsafe fn update_clicked(hwnd: HWND) {
    let (busy, new) = UPD.with(|u| (u.borrow().busy, u.borrow().new.clone()));
    if busy {
        return;
    }
    let Some((ver, hash)) = new else {
        set_upd(hwnd, tr("Buscando actualizaciones…", "Checking for updates…").into(), true, None);
        in_background(hwnd, || Done::Checked(update::latest()));
        return;
    };
    if !update::is_installed() {
        // Copia suelta (sin instalador): se descarga a mano.
        let (op, url) = (wide("open"), wide(&format!("{}/latest", update::RELEASES)));
        ShellExecuteW(hwnd, op.as_ptr(), url.as_ptr(), core::ptr::null(), core::ptr::null(), SW_SHOWNORMAL);
        return;
    }
    if DIRTY.get() {
        let t = wide(&format!(
            "{} {ver} {}",
            tr("Para instalar la versión", "To install version"),
            tr("se cerrará esta ventana y se perderán los cambios sin guardar. ¿Continuar?", "this window will close and unsaved changes will be lost. Continue?")
        ));
        let c = wide("Screensaver FX");
        if MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_YESNO | MB_ICONQUESTION) != IDYES {
            return;
        }
    }
    set_upd(hwnd, format!("{} {ver}…", tr("Descargando la versión", "Downloading version")), true, Some((ver.clone(), hash.clone())));
    in_background(hwnd, move || {
        let r = update::download(&ver, &hash, &update::user_dir());
        Done::Downloaded(ver, r)
    });
}

unsafe fn update_done(hwnd: HWND, done: Done) {
    let cur = update::VERSION;
    match done {
        Done::Checked(Ok(update::Latest::New { ver, hash })) => {
            let s = format!("{} {ver} ({} {cur})", tr("Hay una versión nueva:", "A new version is out:"), tr("tienes la", "you have"));
            set_upd(hwnd, s, false, Some((ver, hash)));
        }
        Done::Checked(Ok(update::Latest::UpToDate)) => {
            set_upd(hwnd, format!("{} {cur} · {}", tr("Versión", "Version"), tr("es la más reciente", "it's the latest")), false, None);
        }
        Done::Checked(Ok(update::Latest::Dev)) => set_upd(hwnd, tr("Versión de desarrollo: sin actualizaciones", "Development build: no updates").into(), false, None),
        Done::Checked(Err(e)) => {
            crate::watch::log(&format!("configuración: no se pudo buscar actualizaciones: {e}"));
            set_upd(hwnd, tr("No se pudo comprobar. Revisa la conexión.", "Couldn't check. Check your connection.").into(), false, None);
        }
        Done::Downloaded(ver, Err(e)) => {
            crate::watch::log(&format!("configuración: no se pudo descargar {ver}: {e}"));
            let new = UPD.with(|u| u.borrow().new.clone());
            set_upd(hwnd, format!("{} {ver}.", tr("No se pudo descargar la versión", "Couldn't download version")), false, new);
        }
        Done::Downloaded(ver, Ok(setup)) => {
            // El instalador sustituye este ejecutable: nada suyo puede seguir abierto.
            stop_preview();
            match update::launch_from_settings(&setup) {
                Ok(()) => {
                    DestroyWindow(hwnd);
                }
                Err((cancelled, e)) => {
                    let new = UPD.with(|u| u.borrow().new.clone());
                    let s = if cancelled {
                        tr("Instalación cancelada.", "Installation cancelled.").to_string()
                    } else {
                        crate::watch::log(&format!("configuración: {e}"));
                        format!("{} {ver}.", tr("No se pudo instalar la versión", "Couldn't install version"))
                    };
                    set_upd(hwnd, s, false, new);
                    restart_preview(hwnd);
                }
            }
        }
    }
}

/// Algo cambió: ya no está «guardado».
unsafe fn mark_dirty(hwnd: HWND) {
    DIRTY.set(true);
    if SAVED.replace(false) {
        let t = wide(tr("Cancelar", "Cancel"));
        SetWindowTextW(item(hwnd, IDCANCEL), t.as_ptr());
        InvalidateRect(hwnd, core::ptr::null(), FALSE);
    }
}

fn with_ui<R>(f: impl FnOnce(&Ui) -> R) -> Option<R> {
    UI.with(|u| u.borrow().as_ref().map(f))
}

fn tr(es: &'static str, en: &'static str) -> &'static str {
    if ES.get() {
        es
    } else {
        en
    }
}

fn theme_values() -> Vec<String> {
    let mut v = vec!["auto".to_string(), "random".to_string()];
    v.extend(THEMES.iter().map(|t| t.id_str().to_string()));
    v
}

fn effect_values() -> Vec<String> {
    let mut v = vec!["all".to_string(), "classic".to_string(), "modern".to_string()];
    v.extend(EFFECTS.iter().map(|f| f.id_str().to_string()));
    v
}

fn read_map() -> HashMap<String, String> {
    sys::read_ini()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect()
}

fn reg_get(name: &str) -> Option<String> {
    let (sub, name) = (wide(DESKTOP), wide(name));
    let mut buf = [0u16; 520];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, core::ptr::null_mut(), buf.as_mut_ptr() as *mut _, &mut size) };
    (r == ERROR_SUCCESS).then(|| String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)]))
}

fn reg_set(name: &str, value: &str) {
    let (sub, name, v) = (wide(DESKTOP), wide(name), wide(value));
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr(), REG_SZ, v.as_ptr() as *const _, (v.len() * 2) as u32) };
}

/// El .scr que Windows debe usar: el de System32 si está instalado para todos, si no el de al lado.
fn scr_path() -> String {
    let exe = std::env::current_exe().unwrap_or_default();
    if exe.extension().is_some_and(|e| e.eq_ignore_ascii_case("scr")) {
        return exe.display().to_string();
    }
    let sys32 = PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into())).join("System32").join("ScreensaverFX.scr");
    let pf = std::env::var_os("ProgramFiles").map(PathBuf::from);
    if sys32.is_file() && pf.is_some_and(|pf| exe.starts_with(pf)) {
        return sys32.display().to_string();
    }
    exe.with_extension("scr").display().to_string()
}

unsafe fn item(hwnd: HWND, id: i32) -> HWND {
    GetDlgItem(hwnd, id)
}

unsafe fn combo_sel(hwnd: HWND, id: i32) -> usize {
    SendMessageW(item(hwnd, id), CB_GETCURSEL, 0, 0).max(0) as usize
}

fn active() -> bool {
    ACTIVE.get()
}

unsafe fn text_of(hwnd: HWND, id: i32) -> String {
    let h = item(hwnd, id);
    let n = GetWindowTextLengthW(h);
    let mut buf = vec![0u16; n as usize + 1];
    GetWindowTextW(h, buf.as_mut_ptr(), n + 1);
    String::from_utf16_lossy(&buf[..n as usize]).replace(['\r', '\n', '='], " ")
}

unsafe fn show(hwnd: HWND, id: i32, on: bool) {
    ShowWindow(item(hwnd, id), if on { SW_SHOW } else { SW_HIDE });
}

/// Texto propio: título y subtítulo. Arte ASCII: botón para editarlo.
unsafe fn sync_enabled(hwnd: HWND) {
    let b = combo_sel(hwnd, ID_BANNER);
    for id in [ID_LBL_TITLE, ID_TITLE, ID_LBL_SUBTITLE, ID_SUBTITLE] {
        show(hwnd, id, b != 2);
    }
    show(hwnd, ID_EDIT_ART, b == 2);
    EnableWindow(item(hwnd, ID_TITLE), (b == 1) as BOOL);
    EnableWindow(item(hwnd, ID_SUBTITLE), (b == 1) as BOOL);
    // El arte ASCII se dibuja tal cual: la letra solo vale para el rótulo por defecto y el texto.
    EnableWindow(item(hwnd, ID_FONT), (b != 2) as BOOL);
    EnableWindow(item(hwnd, ID_MINUTES), active() as BOOL);
    // Las etiquetas no se deshabilitan (el tema las pintaría en relieve): solo cambian de color.
    for id in [ID_LBL_TITLE, ID_LBL_SUBTITLE, ID_LBL_FONT, ID_LBL_AFTER, ID_LBL_MINUTES, ID_ACTIVATE] {
        InvalidateRect(item(hwnd, id), core::ptr::null(), FALSE);
    }
    InvalidateRect(hwnd, core::ptr::null(), FALSE);
}

/// Si la etiqueta acompaña a un campo que ahora mismo no se puede usar.
unsafe fn label_off(hwnd: HWND, id: i32) -> bool {
    match id {
        ID_LBL_TITLE | ID_LBL_SUBTITLE => IsWindowEnabled(item(hwnd, ID_TITLE)) == 0,
        ID_LBL_FONT => IsWindowEnabled(item(hwnd, ID_FONT)) == 0,
        ID_LBL_AFTER | ID_LBL_MINUTES => !active(),
        _ => false,
    }
}

unsafe fn build_ini(hwnd: HWND) -> String {
    let (themes, effects) = (theme_values(), effect_values());
    format!(
        "# Screensaver FX\nbanner={}\ntitle={}\nsubtitle={}\ntheme={}\nsize={}\neffects={}\nlang={}\nscreens={}\nfont={}\ncalm={}\n",
        BANNERS[combo_sel(hwnd, ID_BANNER).min(2)],
        text_of(hwnd, ID_TITLE),
        text_of(hwnd, ID_SUBTITLE),
        themes[combo_sel(hwnd, ID_THEME).min(themes.len() - 1)],
        SIZES[combo_sel(hwnd, ID_SIZE).min(2)],
        effects[combo_sel(hwnd, ID_EFFECTS).min(effects.len() - 1)],
        LANGS[combo_sel(hwnd, ID_LANG).min(2)],
        SCREENS[combo_sel(hwnd, ID_SCREENS).min(2)],
        FONTS[combo_sel(hwnd, ID_FONT).min(FONTS.len() - 1)].0,
        CALMS[combo_sel(hwnd, ID_CALM).min(1)],
    )
}

/// Guarda config.ini y aplica (o retira) el protector de pantalla del sistema.
unsafe fn save(hwnd: HWND) -> bool {
    let ok = std::fs::create_dir_all(sys::data_dir()).is_ok() && std::fs::write(sys::ini_path(), build_ini(hwnd)).is_ok();
    let scr = UI.with(|u| u.borrow().as_ref().map(|u| u.scr.clone())).unwrap_or_default();
    let ours = reg_get("SCRNSAVE.EXE").is_some_and(|c| sys::same_path(&c, &scr));
    if active() {
        let mins = text_of(hwnd, ID_MINUTES).trim().parse::<u32>().unwrap_or(5).clamp(1, 999);
        reg_set("SCRNSAVE.EXE", &scr);
        SystemParametersInfoW(SPI_SETSCREENSAVETIMEOUT, mins * 60, core::ptr::null_mut(), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
        SystemParametersInfoW(SPI_SETSCREENSAVEACTIVE, 1, core::ptr::null_mut(), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
    } else if ours {
        let (sub, name) = (wide(DESKTOP), wide("SCRNSAVE.EXE"));
        RegDeleteKeyValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr());
        SystemParametersInfoW(SPI_SETSCREENSAVEACTIVE, 0, core::ptr::null_mut(), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
    }
    ok
}

unsafe fn edit_art(hwnd: HWND) {
    let path = sys::art_path();
    if !path.exists() {
        let _ = std::fs::create_dir_all(sys::data_dir());
        let _ = std::fs::write(&path, default_text());
    }
    let (op, exe, arg) = (wide("open"), wide("notepad.exe"), wide(&format!("\"{}\"", path.display())));
    ShellExecuteW(hwnd, op.as_ptr(), exe.as_ptr(), arg.as_ptr(), core::ptr::null(), SW_SHOWNORMAL);
}

/// Lanza el propio ejecutable con la configuración que se está editando (sin guardarla).
unsafe fn spawn_with_draft(hwnd: HWND, args: &[String]) -> Option<Child> {
    let ini = UI.with(|u| u.borrow().as_ref().map(|u| u.preview_ini.clone()))?;
    std::fs::write(&ini, build_ini(hwnd)).ok()?;
    Command::new(std::env::current_exe().ok()?).args(args).env("SSFX_CONFIG", &ini).spawn().ok()
}

unsafe fn stop_preview() {
    UI.with(|u| {
        if let Some(mut c) = u.borrow_mut().as_mut().and_then(|u| u.preview.take()) {
            let _ = c.kill();
            let _ = c.wait();
        }
    });
}

unsafe fn restart_preview(hwnd: HWND) {
    stop_preview();
    let Some(host) = UI.with(|u| u.borrow().as_ref().map(|u| u.preview_host)) else { return };
    InvalidateRect(host, core::ptr::null(), TRUE);
    let child = spawn_with_draft(hwnd, &["/p".into(), (host as usize).to_string()]);
    UI.with(|u| {
        if let Some(u) = u.borrow_mut().as_mut() {
            u.preview = child;
        }
    });
}

// --- Dibujo ---------------------------------------------------------------------------------

fn mix(a: COLORREF, b: COLORREF, t: f32) -> COLORREF {
    let ch = |sh: u32| {
        let (x, y) = ((a >> sh & 0xFF) as f32, (b >> sh & 0xFF) as f32);
        ((x + (y - x) * t).round() as u32) << sh
    };
    ch(0) | ch(8) | ch(16)
}

unsafe fn solid(hdc: HDC, rc: &RECT, color: COLORREF) {
    SetBkColor(hdc, color);
    ExtTextOutW(hdc, 0, 0, ETO_OPAQUE, rc, core::ptr::null(), 0, core::ptr::null());
}

/// Rectángulo redondeado con borde y esquinas suavizadas. GDI no suaviza, así que cada píxel de
/// las esquinas se mezcla a mano entre el color de fuera, el del borde y el del relleno.
unsafe fn rrect(hdc: HDC, rc: &RECT, rad: i32, bw: i32, out: COLORREF, border: COLORREF, fill: COLORREF) {
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    if w <= 0 || h <= 0 {
        return;
    }
    let rad = rad.min(w / 2).min(h / 2);
    solid(hdc, rc, border);
    solid(hdc, &RECT { left: rc.left + bw, top: rc.top + bw, right: rc.right - bw, bottom: rc.bottom - bw }, fill);
    let rf = rad as f32;
    for cy in 0..rad {
        for cx in 0..rad {
            let (dx, dy) = (rf - cx as f32 - 0.5, rf - cy as f32 - 0.5);
            let d = (dx * dx + dy * dy).sqrt();
            let outer = (rf - d + 0.5).clamp(0.0, 1.0);
            let inner = (rf - bw as f32 - d + 0.5).clamp(0.0, 1.0);
            let c = mix(out, mix(border, fill, inner), outer);
            for (x, y) in [(rc.left + cx, rc.top + cy), (rc.right - 1 - cx, rc.top + cy), (rc.left + cx, rc.bottom - 1 - cy), (rc.right - 1 - cx, rc.bottom - 1 - cy)] {
                SetPixelV(hdc, x, y, c);
            }
        }
    }
}

unsafe fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, s: &[u16], rc: &RECT, flags: u32) {
    SelectObject(hdc, font);
    SetTextColor(hdc, color);
    SetBkMode(hdc, TRANSPARENT as i32);
    let mut r = *rc;
    DrawTextW(hdc, s.as_ptr(), -1, &mut r, flags | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
}

/// Pinta fuera de pantalla y vuelca de una vez, para que nada parpadee.
unsafe fn buffered(hdc: HDC, rc: &RECT, draw: impl FnOnce(HDC)) {
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    let mem = CreateCompatibleDC(hdc);
    let bmp = CreateCompatibleBitmap(hdc, w.max(1), h.max(1));
    let old = SelectObject(mem, bmp);
    SetViewportOrgEx(mem, -rc.left, -rc.top, core::ptr::null_mut());
    draw(mem);
    SetViewportOrgEx(mem, 0, 0, core::ptr::null_mut());
    BitBlt(hdc, rc.left, rc.top, w, h, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
}

unsafe fn window_text(hwnd: HWND) -> Vec<u16> {
    let mut buf = vec![0u16; GetWindowTextLengthW(hwnd) as usize + 1];
    GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    buf
}

unsafe fn paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = core::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    with_ui(|u| {
        buffered(hdc, &rc, |dc| {
            let pal = u.pal;
            let line = u.px(1).max(1);
            let r = |x: i32, y: i32, w: i32, h: i32| RECT { left: u.px(x), top: u.px(y), right: u.px(x + w), bottom: u.px(y + h) };
            solid(dc, &rc, pal.bg);
            // Cabecera: icono, título y descripción, sobre el mismo fondo que la barra de título.
            if !u.icon.is_null() {
                DrawIconEx(dc, u.px(PAD), u.px(22), u.icon, u.px(40), u.px(40), 0, core::ptr::null_mut(), DI_NORMAL);
            }
            let title = wide(tr("Personaliza tu protector", "Make it yours"));
            let head_w = W - 2 * PAD - 54 - UPD_W - GAP;
            draw_text(dc, u.font_title, pal.text, &title, &r(PAD + 54, 18, head_w, 30), DT_LEFT);
            let sub = wide(tr("Los cambios se ven al instante en la vista previa.", "Changes show up instantly in the preview."));
            draw_text(dc, u.font, pal.dim, &sub, &r(PAD + 54, 46, head_w, 22), DT_LEFT);
            // Bajo el botón de actualizaciones: versión instalada o lo que pasó al buscar.
            let status = wide(&UPD.with(|s| s.borrow().status.clone()));
            draw_text(dc, u.font_small, pal.dim, &status, &r(W - PAD - 330, UPD_Y + CTL_H + 6, 330, 18), DT_RIGHT);
            // Pie: confirmación de guardado junto a los botones.
            if SAVED.get() {
                let ok = wide(tr("✓  Cambios guardados", "✓  Changes saved"));
                let x0 = PAD + 128 + GAP;
                draw_text(dc, u.font, pal.text, &ok, &r(x0, BUTTONS_Y, W - PAD - 2 * 112 - 8 - GAP - x0, CTL_H), DT_RIGHT | DT_VCENTER);
            } else {
                let note = NOTE.with(|n| n.borrow().clone());
                if !note.is_empty() {
                    let x0 = PAD + 128 + GAP;
                    let note = wide(&note);
                    draw_text(dc, u.font_small, pal.dim, &note, &r(x0, BUTTONS_Y, W - PAD - 2 * 112 - 8 - GAP - x0, CTL_H), DT_RIGHT | DT_VCENTER);
                }
            }
            // Tarjetas: opciones, vista previa y activación.
            for card in [r(PAD, CARD_Y, LEFT_W, CARD_H), r(RIGHT_X, CARD_Y, RIGHT_W, CARD_H), r(PAD, ACT_Y, W - 2 * PAD, ACT_H)] {
                rrect(dc, &card, u.px(8), line, pal.bg, pal.card_line, pal.card);
            }
            // Bisel de la vista previa: negro como el protector, así sus esquinas parecen redondas.
            let edge = if pal.dark { rgb(0x454545) } else { rgb(0x000000) };
            rrect(dc, &u.bezel, u.px(8), line, pal.card, edge, rgb(0x000000));
            let hint = wide(tr("Windows lo iniciará cuando no uses el equipo.", "Windows will start it when you are away."));
            draw_text(dc, u.font_small, pal.dim, &hint, &r(PAD + CP, ACT_Y + 34, 380, 18), DT_LEFT);
            // Con arte ASCII, una nota ocupa el hueco que dejan el título y el subtítulo.
            if IsWindowVisible(item(hwnd, ID_EDIT_ART)) != 0 {
                let note = wide(tr(
                    "Se abre en el Bloc de notas. Dibuja o pega ahí tu arte ASCII y guarda el archivo: el protector lo usará como rótulo.",
                    "Opens in Notepad. Draw or paste your ASCII art there and save the file: the screensaver will use it as its banner.",
                ));
                let mut area = r(PAD + CP, CARD_Y + CP + STEP + 4 + CTL_H + 10, LEFT_W - 2 * CP, STEP);
                SelectObject(dc, u.font_small);
                SetTextColor(dc, pal.dim);
                DrawTextW(dc, note.as_ptr(), -1, &mut area, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);
            }
            // Marcos de los campos de texto.
            for (id, f) in &u.frames {
                let e = item(hwnd, *id);
                if IsWindowVisible(e) == 0 {
                    continue;
                }
                let (border, fill) = if IsWindowEnabled(e) == 0 {
                    (pal.field_off_line, pal.field_off)
                } else if GetFocus() == e {
                    (u.accent, pal.field)
                } else {
                    (pal.field_line, pal.field)
                };
                rrect(dc, f, u.px(4), line, pal.card, border, fill);
            }
        })
    });
    EndPaint(hwnd, &ps);
}

/// Botones e interruptor (BS_OWNERDRAW): relleno de acento para el principal, neutro el resto.
unsafe fn draw_item(d: &DRAWITEMSTRUCT) {
    with_ui(|u| {
        buffered(d.hDC, &d.rcItem, |dc| {
            let pal = u.pal;
            let line = u.px(1).max(1);
            let id = d.CtlID as i32;
            let rc = d.rcItem;
            let hot = HOT.get() == d.hwndItem;
            let pressed = d.itemState & ODS_SELECTED != 0;
            let focus = d.itemState & ODS_FOCUS != 0 && d.itemState & ODS_NOFOCUSRECT == 0;
            let out = if id == ID_EDIT_ART || id == ID_ACTIVATE { pal.card } else { pal.bg };
            solid(dc, &rc, out);
            if id == ID_ACTIVATE {
                let on = active();
                if focus {
                    rrect(dc, &rc, (rc.bottom - rc.top) / 2, line, out, pal.text, out);
                }
                let m = u.px(3);
                let pill = RECT { left: rc.left + m, top: rc.top + m, right: rc.right - m, bottom: rc.bottom - m };
                let ph = pill.bottom - pill.top;
                let (border, fill, knob) = if on {
                    let a = if hot { mix(u.accent, out, 0.12) } else { u.accent };
                    (a, a, u.on_accent)
                } else {
                    (pal.dim, if hot { pal.field_hot } else { out }, pal.dim)
                };
                rrect(dc, &pill, ph / 2, line, out, border, fill);
                let k = u.px(if hot || pressed { 14 } else { 12 });
                let pad = (ph - k) / 2;
                let x = if on { pill.right - pad - k } else { pill.left + pad };
                let kr = RECT { left: x, top: pill.top + pad, right: x + k, bottom: pill.top + pad + k };
                rrect(dc, &kr, k / 2, 0, fill, knob, knob);
                return;
            }
            let (border, fill, text) = if IsWindowEnabled(d.hwndItem) == 0 {
                (pal.field_off_line, pal.field_off, pal.off)
            } else if id == IDOK {
                let a = mix(u.accent, out, if pressed { 0.22 } else if hot { 0.1 } else { 0.0 });
                (a, a, u.on_accent)
            } else if pressed {
                (pal.field_line, pal.field_press, pal.dim)
            } else {
                (pal.field_line, if hot { pal.field_hot } else { pal.field }, pal.text)
            };
            if focus {
                rrect(dc, &rc, u.px(4), 2 * line, out, pal.text, fill);
            } else {
                rrect(dc, &rc, u.px(4), line, out, border, fill);
            }
            draw_text(dc, u.font, text, &window_text(d.hwndItem), &rc, DT_CENTER | DT_VCENTER);
        })
    });
}

/// Desplegable: el control sigue siendo el COMBOBOX nativo (lista, teclado, accesibilidad), pero
/// la caja cerrada se pinta aquí para que tenga el mismo aspecto que los campos de texto.
unsafe fn paint_combo(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = core::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut rc);
    let sel = SendMessageW(hwnd, CB_GETCURSEL, 0, 0);
    let mut text = vec![0u16; 1];
    if sel >= 0 {
        text = vec![0u16; SendMessageW(hwnd, CB_GETLBTEXTLEN, sel as usize, 0).max(0) as usize + 1];
        SendMessageW(hwnd, CB_GETLBTEXT, sel as usize, text.as_mut_ptr() as LPARAM);
    }
    let dropped = SendMessageW(hwnd, CB_GETDROPPEDSTATE, 0, 0) != 0;
    with_ui(|u| {
        buffered(hdc, &rc, |dc| {
            let pal = u.pal;
            let line = u.px(1).max(1);
            let fill = if dropped {
                pal.field_press
            } else if HOT.get() == hwnd {
                pal.field_hot
            } else {
                pal.field
            };
            let border = if GetFocus() == hwnd { u.accent } else { pal.field_line };
            rrect(dc, &rc, u.px(4), line, pal.card, border, fill);
            let area = RECT { left: rc.left + u.px(11), top: rc.top, right: rc.right - u.px(32), bottom: rc.bottom };
            draw_text(dc, u.font, pal.text, &text, &area, DT_LEFT | DT_VCENTER);
            // Flecha: dos trazos a 45°, que GDI dibuja limpios sin suavizado.
            let (cx, cy, a) = (rc.right - u.px(17), (rc.top + rc.bottom) / 2, u.px(4));
            let pen = CreatePen(PS_SOLID, 1, pal.dim);
            let old = SelectObject(dc, pen);
            for dy in 0..line {
                let pts = [POINT { x: cx - a, y: cy - a / 2 + dy }, POINT { x: cx, y: cy + a - a / 2 + dy }, POINT { x: cx + a + 1, y: cy - a / 2 - 1 + dy }];
                Polyline(dc, pts.as_ptr(), 3);
            }
            SelectObject(dc, old);
            DeleteObject(pen);
        })
    });
    EndPaint(hwnd, &ps);
}

/// Subclase común de botones y desplegables: resaltado al pasar el ratón y pintado propio.
unsafe extern "system" fn ctl_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, kind: usize) -> LRESULT {
    match msg {
        WM_MOUSEMOVE if HOT.get() != hwnd => {
            HOT.set(hwnd);
            let mut t = TRACKMOUSEEVENT { cbSize: core::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
            TrackMouseEvent(&mut t);
            InvalidateRect(hwnd, core::ptr::null(), FALSE);
        }
        WM_MOUSELEAVE if HOT.get() == hwnd => {
            HOT.set(core::ptr::null_mut());
            InvalidateRect(hwnd, core::ptr::null(), FALSE);
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(ctl_proc), 0);
        }
        // Un botón owner-draw convierte el doble clic en otra notificación y se pierde un clic.
        WM_LBUTTONDBLCLK if kind == KIND_BUTTON => return DefSubclassProc(hwnd, WM_LBUTTONDOWN, wp, lp),
        WM_PAINT if kind == KIND_COMBO => {
            paint_combo(hwnd);
            return 0;
        }
        WM_ERASEBKGND if kind == KIND_COMBO => return 1,
        // El combo nativo repinta por su cuenta en estos casos; se vuelve a pintar encima.
        WM_SETFOCUS | WM_KILLFOCUS | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_KEYDOWN | WM_KEYUP | WM_CHAR | WM_MOUSEWHEEL | CB_SETCURSEL | CB_SHOWDROPDOWN if kind == KIND_COMBO => {
            let r = DefSubclassProc(hwnd, msg, wp, lp);
            InvalidateRect(hwnd, core::ptr::null(), FALSE);
            UpdateWindow(hwnd);
            return r;
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            return 0;
        }
        WM_ERASEBKGND => return 1,
        WM_DRAWITEM => {
            draw_item(&*(lp as *const DRAWITEMSTRUCT));
            return 1;
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            let hdc = wp as HDC;
            let id = GetDlgCtrlID(lp as HWND);
            let is_edit = matches!(id, ID_MINUTES | ID_TITLE | ID_SUBTITLE);
            let r = with_ui(|u| {
                let pal = u.pal;
                // Un EDIT deshabilitado llega como WM_CTLCOLORSTATIC.
                let (back, brush) = match msg {
                    WM_CTLCOLORSTATIC if is_edit => (pal.field_off, u.field_off),
                    WM_CTLCOLORSTATIC => (pal.card, u.card),
                    WM_CTLCOLORBTN => (pal.bg, u.bg),
                    _ => (pal.field, u.field),
                };
                let text = if msg == WM_CTLCOLORSTATIC && (is_edit || label_off(hwnd, id)) {
                    pal.off
                } else if msg == WM_CTLCOLORSTATIC && !matches!(id, ID_ACTIVATE_LABEL | ID_LBL_AFTER | ID_LBL_MINUTES) {
                    pal.dim
                } else {
                    pal.text
                };
                SetTextColor(hdc, text);
                SetBkColor(hdc, back);
                brush as LRESULT
            });
            if let Some(r) = r {
                return r;
            }
        }
        // Clic en el margen de un campo: el foco va a su EDIT.
        WM_LBUTTONDOWN => {
            let pt = POINT { x: (lp & 0xFFFF) as i16 as i32, y: (lp >> 16 & 0xFFFF) as i16 as i32 };
            let hit = with_ui(|u| u.frames.iter().find(|(_, f)| PtInRect(f, pt) != 0).map(|(id, _)| *id)).flatten();
            if let Some(e) = hit.map(|id| item(hwnd, id)) {
                if IsWindowVisible(e) != 0 && IsWindowEnabled(e) != 0 {
                    SetFocus(e);
                }
            }
            return 0;
        }
        WM_TIMER if wp == TIMER_PREVIEW => {
            KillTimer(hwnd, TIMER_PREVIEW);
            restart_preview(hwnd);
            return 0;
        }
        WM_TIMER if wp == TIMER_IDENTIFY => {
            KillTimer(hwnd, TIMER_IDENTIFY);
            close_identify();
            return 0;
        }
        WM_COMMAND => {
            let (id, code) = ((wp & 0xFFFF) as i32, (wp >> 16 & 0xFFFF) as u32);
            match id {
                // El borde de acento sigue al foco.
                ID_TITLE | ID_SUBTITLE | ID_MINUTES if code == EN_SETFOCUS || code == EN_KILLFOCUS => {
                    InvalidateRect(hwnd, core::ptr::null(), FALSE);
                }
                ID_BANNER | ID_THEME | ID_SIZE | ID_EFFECTS | ID_LANG | ID_SCREENS | ID_FONT | ID_CALM => {
                    InvalidateRect(lp as HWND, core::ptr::null(), FALSE);
                }
                _ => {}
            }
            let changed = match id {
                ID_BANNER | ID_THEME | ID_SIZE | ID_EFFECTS | ID_LANG | ID_FONT | ID_CALM if code == CBN_SELCHANGE => {
                    sync_enabled(hwnd);
                    true
                }
                ID_TITLE | ID_SUBTITLE if code == EN_CHANGE => true,
                // No cambia la vista previa (es una sola pantalla): solo hay algo que guardar.
                ID_SCREENS if code == CBN_SELCHANGE => {
                    mark_dirty(hwnd);
                    false
                }
                ID_IDENTIFY => {
                    identify(hwnd);
                    false
                }
                // BN_CLICKED del interruptor o STN_CLICKED de su texto.
                ID_ACTIVATE | ID_ACTIVATE_LABEL if code == 0 => {
                    ACTIVE.set(!ACTIVE.get());
                    sync_enabled(hwnd);
                    false
                }
                ID_EDIT_ART => {
                    edit_art(hwnd);
                    false
                }
                ID_TEST => {
                    let _ = spawn_with_draft(hwnd, &["/s".into()]);
                    false
                }
                ID_UPDATE => {
                    update_clicked(hwnd);
                    false
                }
                // Guarda y deja la ventana abierta con la confirmación a la vista.
                IDOK => {
                    if save(hwnd) {
                        DIRTY.set(false);
                        NOTE.with(|n| n.borrow_mut().clear());
                        SAVED.set(true);
                        let t = wide(tr("Cerrar", "Close"));
                        SetWindowTextW(item(hwnd, IDCANCEL), t.as_ptr());
                        InvalidateRect(hwnd, core::ptr::null(), FALSE);
                    } else {
                        let (t, c) = (wide(tr("No se pudo guardar la configuración.", "Could not save the settings.")), wide("Screensaver FX"));
                        MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_ICONERROR);
                    }
                    false
                }
                IDCANCEL => {
                    DestroyWindow(hwnd);
                    false
                }
                _ => false,
            };
            if changed {
                SetTimer(hwnd, TIMER_PREVIEW, PREVIEW_DELAY_MS, None);
            }
            if changed || (matches!(id, ID_ACTIVATE | ID_ACTIVATE_LABEL) && code == 0) || (id == ID_MINUTES && code == EN_CHANGE) {
                mark_dirty(hwnd);
            }
            return 0;
        }
        WM_UPDATE_DONE => {
            update_done(hwnd, *Box::from_raw(lp as *mut Done));
            return 0;
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            return 0;
        }
        WM_DESTROY => {
            stop_preview();
            close_identify();
            PostQuitMessage(0);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

// --- Identificar pantallas ------------------------------------------------------------------

/// Una ventana con el número de cada pantalla, como en Configuración › Pantalla, pero con el
/// orden en que la recorren los efectos de escritorio (el que usa `Desk`).
struct Badge {
    hwnd: HWND,
    num: Vec<u16>,
    info: Vec<u16>,
}

thread_local! {
    static BADGES: RefCell<Vec<Badge>> = const { RefCell::new(Vec::new()) };
    static BADGE_FONT: Cell<HFONT> = const { Cell::new(core::ptr::null_mut()) };
}

unsafe extern "system" fn monitor_proc(_: HMONITOR, _: HDC, rc: *mut RECT, data: LPARAM) -> BOOL {
    (*(data as *mut Vec<RECT>)).push(*rc);
    TRUE
}

/// Monitores de izquierda a derecha por su centro y, si coinciden, de arriba abajo.
fn monitors() -> Vec<RECT> {
    let mut v: Vec<RECT> = Vec::new();
    unsafe { EnumDisplayMonitors(core::ptr::null_mut(), core::ptr::null(), Some(monitor_proc), &mut v as *mut _ as LPARAM) };
    v.sort_by_key(|r| (r.left + r.right, r.top + r.bottom));
    v
}

unsafe fn close_identify() {
    for b in BADGES.with(|v| v.take()) {
        DestroyWindow(b.hwnd);
    }
    let f = BADGE_FONT.with(|f| f.replace(core::ptr::null_mut()));
    if !f.is_null() {
        DeleteObject(f);
    }
}

unsafe fn identify(owner: HWND) {
    close_identify();
    let Some(s) = with_ui(|u| u.s) else { return };
    let hinst = GetModuleHandleW(core::ptr::null());
    let class = wide("ScreensaverFXBadge");
    let wc = WNDCLASSEXW {
        cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(badge_proc),
        hInstance: hinst,
        hCursor: LoadCursorW(core::ptr::null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..core::mem::zeroed()
    };
    RegisterClassExW(&wc);
    BADGE_FONT.with(|f| f.set(make_font(owner, (110.0 * s) as i32, FW_SEMIBOLD, "Segoe UI Variable Display")));
    let (bw, bh) = ((220.0 * s) as i32, (220.0 * s) as i32);
    for (i, r) in monitors().iter().enumerate() {
        let main = r.left == 0 && r.top == 0;
        let size = format!("{} × {}", r.right - r.left, r.bottom - r.top);
        let info = if main { format!("{}  ·  {size}", tr("Principal", "Main")) } else { size };
        // Abajo a la izquierda, como Windows, para no tapar el configurador.
        let (x, y) = (r.left + (40.0 * s) as i32, r.bottom - bh - (90.0 * s) as i32);
        let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, class.as_ptr(), class.as_ptr(), WS_POPUP, x, y, bw, bh, owner, core::ptr::null_mut(), hinst, core::ptr::null());
        if hwnd.is_null() {
            continue;
        }
        // Esquinas redondas en Windows 11 (en Windows 10 no hace nada).
        let round: u32 = 2;
        DwmSetWindowAttribute(hwnd, 33 /* DWMWA_WINDOW_CORNER_PREFERENCE */, &round as *const _ as *const _, 4);
        BADGES.with(|v| v.borrow_mut().push(Badge { hwnd, num: wide(&(i + 1).to_string()), info: wide(&info) }));
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    SetTimer(owner, TIMER_IDENTIFY, IDENTIFY_MS, None);
}

unsafe extern "system" fn badge_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = core::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc: RECT = core::mem::zeroed();
            GetClientRect(hwnd, &mut rc);
            let texts = BADGES.with(|v| v.borrow().iter().find(|b| b.hwnd == hwnd).map(|b| (b.num.clone(), b.info.clone())));
            with_ui(|u| {
                buffered(hdc, &rc, |dc| {
                    solid(dc, &rc, u.pal.card);
                    // Borde de acento, como la pantalla seleccionada en Configuración.
                    let k = u.px(3).max(2);
                    for (l, t, r, b) in [(0, 0, rc.right, k), (0, rc.bottom - k, rc.right, rc.bottom), (0, 0, k, rc.bottom), (rc.right - k, 0, rc.right, rc.bottom)] {
                        solid(dc, &RECT { left: l, top: t, right: r, bottom: b }, u.accent);
                    }
                    if let Some((num, info)) = &texts {
                        let big = RECT { left: 0, top: 0, right: rc.right, bottom: rc.bottom - u.px(44) };
                        draw_text(dc, BADGE_FONT.with(|f| f.get()), u.pal.text, num, &big, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                        let low = RECT { left: 0, top: rc.bottom - u.px(56), right: rc.right, bottom: rc.bottom - u.px(20) };
                        draw_text(dc, u.font, u.pal.dim, info, &low, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                    }
                })
            });
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_ERASEBKGND => return 1,
        // Un clic en cualquiera las cierra todas.
        WM_LBUTTONDOWN => {
            close_identify();
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

// --- Construcción ---------------------------------------------------------------------------

struct Builder {
    hwnd: HWND,
    hinst: HINSTANCE,
    font: HFONT,
    font_small: HFONT,
    s: f32,
    dark: bool,
    /// Alto de una línea de texto, en píxeles reales.
    line_h: i32,
    frames: RefCell<Vec<(i32, RECT)>>,
}

#[allow(clippy::too_many_arguments)]
impl Builder {
    fn px(&self, v: i32) -> i32 {
        (v as f32 * self.s).round() as i32
    }

    /// Crea un control con posición y tamaño en píxeles reales.
    unsafe fn add_px(&self, class: &str, text: &str, style: u32, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
        let (c, t) = (wide(class), wide(text));
        let ctl = CreateWindowExW(0, c.as_ptr(), t.as_ptr(), WS_CHILD | WS_VISIBLE | style, x, y, w, h, self.hwnd, id as usize as HMENU, self.hinst, core::ptr::null());
        SendMessageW(ctl, WM_SETFONT, self.font as WPARAM, 1);
        if self.dark {
            let theme = wide(if class == "BUTTON" { "DarkMode_Explorer" } else { "DarkMode_CFD" });
            SetWindowTheme(ctl, theme.as_ptr(), core::ptr::null());
        }
        ctl
    }

    unsafe fn add(&self, class: &str, text: &str, style: u32, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
        self.add_px(class, text, style, id, self.px(x), self.px(y), self.px(w), self.px(h))
    }

    /// Etiqueta pequeña y atenuada encima de un campo.
    unsafe fn label(&self, id: i32, text: &str, x: i32, y: i32, w: i32) {
        let l = self.add("STATIC", text, 0x80 /* SS_NOPREFIX */, id, x, y, w, 16);
        SendMessageW(l, WM_SETFONT, self.font_small as WPARAM, 1);
    }

    /// Texto normal en línea con un campo de `CTL_H` de alto que empieza en `y`.
    unsafe fn text(&self, id: i32, text: &str, style: u32, x: i32, y: i32, w: i32) {
        let top = self.px(y) + (self.px(CTL_H) - self.line_h) / 2;
        self.add_px("STATIC", text, 0x80 /* SS_NOPREFIX */ | style, id, self.px(x), top, self.px(w), self.line_h);
    }

    unsafe fn combo(&self, id: i32, x: i32, y: i32, w: i32, items: &[String], sel: usize) {
        let c = self.add("COMBOBOX", "", (CBS_DROPDOWNLIST as u32) | WS_TABSTOP | WS_VSCROLL, id, x, y, w, 360);
        for it in items {
            let t = wide(it);
            SendMessageW(c, CB_ADDSTRING, 0, t.as_ptr() as LPARAM);
        }
        SendMessageW(c, CB_SETCURSEL, sel.min(items.len().saturating_sub(1)), 0);
        // El alto de la caja es el del «ítem -1» más el borde del tema: se mide y se corrige.
        let want = self.px(CTL_H);
        SendMessageW(c, CB_SETITEMHEIGHT, usize::MAX, (want - 6) as LPARAM);
        let mut r: RECT = core::mem::zeroed();
        GetWindowRect(c, &mut r);
        let got = r.bottom - r.top;
        if got != want {
            SendMessageW(c, CB_SETITEMHEIGHT, usize::MAX, (2 * want - 6 - got) as LPARAM);
        }
        SetWindowSubclass(c, Some(ctl_proc), 0, KIND_COMBO);
    }

    /// EDIT sin borde centrado dentro de un marco redondeado que pinta la ventana.
    unsafe fn edit(&self, id: i32, x: i32, y: i32, w: i32, text: &str, max: usize, extra: u32) {
        let frame = RECT { left: self.px(x), top: self.px(y), right: self.px(x) + self.px(w), bottom: self.px(y) + self.px(CTL_H) };
        let inset = self.px(if extra & 0x1 != 0 { 6 } else { 10 });
        let top = frame.top + (frame.bottom - frame.top - self.line_h) / 2;
        let e = self.add_px("EDIT", text, (ES_AUTOHSCROLL as u32) | WS_TABSTOP | extra, id, frame.left + inset, top, frame.right - frame.left - 2 * inset, self.line_h);
        SendMessageW(e, 0x00C5 /* EM_LIMITTEXT */, max, 0);
        SendMessageW(e, 0x00D3 /* EM_SETMARGINS */, 3 /* EC_LEFTMARGIN | EC_RIGHTMARGIN */, 0);
        self.frames.borrow_mut().push((id, frame));
    }

    unsafe fn button(&self, id: i32, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
        let b = self.add("BUTTON", text, (BS_OWNERDRAW as u32) | WS_TABSTOP, id, x, y, w, h);
        SetWindowSubclass(b, Some(ctl_proc), 0, KIND_BUTTON);
        b
    }

    /// Ancho en píxeles lógicos de un texto con la fuente normal.
    unsafe fn text_w(&self, s: &str) -> i32 {
        let hdc = GetDC(self.hwnd);
        let old = SelectObject(hdc, self.font);
        let t = wide(s);
        let mut sz = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(hdc, t.as_ptr(), t.len() as i32 - 1, &mut sz);
        SelectObject(hdc, old);
        ReleaseDC(self.hwnd, hdc);
        (sz.cx as f32 / self.s).ceil() as i32
    }
}

fn system_dark() -> bool {
    // Solo para pruebas: `SSFX_THEME=light|dark` manda sobre el registro, sin tocar el tema del usuario.
    match std::env::var("SSFX_THEME").as_deref() {
        Ok("light") => return false,
        Ok("dark") => return true,
        _ => {}
    }
    let (sub, name) = (wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"), wide("AppsUseLightTheme"));
    let (mut v, mut size) = (1u32, 4u32);
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr(), RRF_RT_REG_DWORD, core::ptr::null_mut(), &mut v as *mut u32 as *mut _, &mut size) };
    r == ERROR_SUCCESS && v == 0
}

/// Acento para el botón principal y el interruptor, como en Windows 11: el tono claro de la
/// paleta en tema oscuro y el oscuro en tema claro. Sin paleta, se aproxima desde el acento base.
fn accent_color(dark: bool) -> COLORREF {
    let (sub, name) = (wide("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Accent"), wide("AccentPalette"));
    let mut p = [0u8; 32];
    let mut size = p.len() as u32;
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr(), RRF_RT_REG_BINARY, core::ptr::null_mut(), p.as_mut_ptr() as *mut _, &mut size) };
    let c = if r == ERROR_SUCCESS && size == 32 {
        // Ocho colores RGBA, del más claro al más oscuro: el 1 es «Light 2» y el 4, «Dark 1».
        let i = if dark { 4 } else { 16 };
        p[i] as u32 | (p[i + 1] as u32) << 8 | (p[i + 2] as u32) << 16
    } else if dark {
        mix(rgb(sys::accent()), rgb(0xFFFFFF), 0.45)
    } else {
        mix(rgb(sys::accent()), rgb(0x000000), 0.12)
    };
    // Con acentos casi negros o casi blancos el botón principal no se distinguiría del resto.
    if dark && luma(c) < 120 {
        mix(c, rgb(0xFFFFFF), 0.6)
    } else if !dark && luma(c) > 170 {
        mix(c, rgb(0x000000), 0.5)
    } else {
        c
    }
}

/// Luminosidad percibida, de 0 a 255.
fn luma(c: COLORREF) -> u32 {
    ((c & 0xFF) * 299 + (c >> 8 & 0xFF) * 587 + (c >> 16 & 0xFF) * 114) / 1000
}

/// Fuente de la interfaz: Segoe UI Variable (Windows 11) o, si no está, Segoe UI.
unsafe fn make_font(hwnd: HWND, px: i32, weight: u32, variable: &str) -> HFONT {
    let hdc = GetDC(hwnd);
    let mut font = core::ptr::null_mut();
    for face in [variable, "Segoe UI"] {
        let f = wide(face);
        font = CreateFontW(-px, 0, 0, 0, weight as i32, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, f.as_ptr());
        let old = SelectObject(hdc, font);
        let mut got = [0u16; 64];
        let n = GetTextFaceW(hdc, got.len() as i32, got.as_mut_ptr()).max(1) as usize - 1;
        SelectObject(hdc, old);
        if face == "Segoe UI" || String::from_utf16_lossy(&got[..n]).starts_with(face) {
            break;
        }
        DeleteObject(font);
    }
    ReleaseDC(hwnd, hdc);
    font
}

pub fn run(owner: Option<HWND>) {
    unsafe {
        ES.set(sys::system_is_spanish());
        let pal = if system_dark() { DARK } else { LIGHT };
        let hinst = GetModuleHandleW(core::ptr::null());
        let bg = CreateSolidBrush(pal.bg);
        let class = wide("ScreensaverFXSettings");
        let icon_big = LoadImageW(hinst, 1 as _, IMAGE_ICON, GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON), LR_DEFAULTCOLOR) as HICON;
        let icon_small = LoadImageW(hinst, 1 as _, IMAGE_ICON, GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_DEFAULTCOLOR) as HICON;
        let wc = WNDCLASSEXW {
            cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hCursor: LoadCursorW(core::ptr::null_mut(), IDC_ARROW),
            hbrBackground: bg,
            lpszClassName: class.as_ptr(),
            hIcon: icon_big,
            hIconSm: icon_small,
            ..core::mem::zeroed()
        };
        RegisterClassExW(&wc);

        let dpi = GetDpiForSystem().max(96);
        let s = dpi as f32 / 96.0;
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
        let mut rc = RECT { left: 0, top: 0, right: (W as f32 * s).round() as i32, bottom: (H as f32 * s).round() as i32 };
        AdjustWindowRect(&mut rc, style, FALSE);
        let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let title = wide(tr("Screensaver FX · Configuración", "Screensaver FX · Settings"));
        let owner_h = owner.unwrap_or(core::ptr::null_mut());
        let hwnd = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), style, (sw - ww) / 2, ((sh - wh) / 2).max(0), ww, wh, owner_h, core::ptr::null_mut(), hinst, core::ptr::null());
        if hwnd.is_null() {
            return;
        }
        if pal.dark {
            let on: BOOL = 1;
            DwmSetWindowAttribute(hwnd, 20 /* DWMWA_USE_IMMERSIVE_DARK_MODE */, &on as *const _ as *const _, 4);
        }
        // En Windows 11 la barra de título toma el color del fondo y la cabecera queda de una pieza.
        DwmSetWindowAttribute(hwnd, 35 /* DWMWA_CAPTION_COLOR */, &pal.bg as *const _ as *const _, 4);

        let px = |v: i32| (v as f32 * s).round() as i32;
        let font = make_font(hwnd, px(14), FW_NORMAL, "Segoe UI Variable Text");
        let font_small = make_font(hwnd, px(12), FW_NORMAL, "Segoe UI Variable Small");
        let font_title = make_font(hwnd, px(20), FW_SEMIBOLD, "Segoe UI Variable Display");
        let hdc = GetDC(hwnd);
        let old = SelectObject(hdc, font);
        let mut tm: TEXTMETRICW = core::mem::zeroed();
        GetTextMetricsW(hdc, &mut tm);
        SelectObject(hdc, old);
        ReleaseDC(hwnd, hdc);
        let icon = LoadImageW(hinst, 1 as _, IMAGE_ICON, px(40), px(40), LR_DEFAULTCOLOR) as HICON;
        let preview_ini = std::env::temp_dir().join(format!("ssfx-preview-{}.ini", std::process::id()));
        let b = Builder { hwnd, hinst, font, font_small, s, dark: pal.dark, line_h: tm.tmHeight, frames: RefCell::new(Vec::new()) };

        let m = read_map();
        let get = |k: &str| m.get(k).cloned().unwrap_or_default();
        let pos = |vals: &[&str], k: &str, def: usize| vals.iter().position(|v| v.eq_ignore_ascii_case(&get(k))).unwrap_or(def);
        let es = ES.get();

        // Tarjeta izquierda: rótulo, texto, colores y efectos.
        let (x, w) = (PAD + CP, LEFT_W - 2 * CP);
        let mut y = CARD_Y + CP;
        b.label(-1, tr("Rótulo", "Banner"), x, y, w);
        let banners = [tr("JACC · XACODE (por defecto)", "JACC · XACODE (default)"), tr("Texto propio", "Custom text"), tr("Arte ASCII (screensaver.txt)", "ASCII art (screensaver.txt)")];
        b.combo(ID_BANNER, x, y + LABEL_H, w, &banners.map(String::from), pos(&BANNERS, "banner", 0));
        y += STEP;
        b.label(ID_LBL_TITLE, tr("Título (grande)", "Title (large)"), x, y, w);
        b.edit(ID_TITLE, x, y + LABEL_H, w, &get("title"), MAX_TITLE, 0);
        b.button(ID_EDIT_ART, tr("Editar screensaver.txt…", "Edit screensaver.txt…"), x, y + 4, w, CTL_H);
        y += STEP;
        b.label(ID_LBL_SUBTITLE, tr("Subtítulo (opcional)", "Subtitle (optional)"), x, y, w);
        b.edit(ID_SUBTITLE, x, y + LABEL_H, w, &get("subtitle"), MAX_SUBTITLE, 0);
        y += STEP;
        b.label(-1, tr("Colores", "Colors"), x, y, w);
        let mut themes = vec![tr("Color de acento de Windows", "Windows accent color").to_string(), tr("Aleatorio (cambia en cada efecto)", "Random (changes every effect)").to_string()];
        themes.extend(THEMES.iter().map(|t| t.name_str().to_string()));
        let theme = get("theme");
        let theme = theme.trim();
        let tsel = theme_values().iter().position(|v| v.eq_ignore_ascii_case(theme));
        // Un tema que no existe (config.ini editado a mano) se usa como «color de acento»: se avisa.
        if tsel.is_none() && !theme.is_empty() && !theme.eq_ignore_ascii_case("system") {
            let msg = if ES.get() {
                format!("Tema «{theme}» desconocido: se usa el de acento.")
            } else {
                format!("Unknown theme \"{theme}\": using the accent color.")
            };
            NOTE.with(|n| *n.borrow_mut() = msg);
        }
        let tsel = tsel.unwrap_or(0);
        b.combo(ID_THEME, x, y + LABEL_H, w, &themes, tsel);
        y += STEP;
        b.label(-1, tr("Efectos", "Effects"), x, y, w);
        let mut fx = vec![tr("Todos", "All").to_string(), tr("Clásicos de Omarchy", "Omarchy classics").to_string(), tr("Solo los nuevos", "New ones only").to_string()];
        fx.extend(EFFECTS.iter().map(|f| {
            let n = if es { f.name_es } else { f.name_en };
            format!("{} {}", tr("Solo:", "Only:"), n.to_str().unwrap_or(""))
        }));
        let fsel = effect_values().iter().position(|v| v.eq_ignore_ascii_case(&get("effects"))).unwrap_or(0);
        b.combo(ID_EFFECTS, x, y + LABEL_H, w, &fx, fsel);

        // Tarjeta derecha: vista previa 16:9 dentro de su bisel; debajo, tamaño e idioma del reloj
        // a la altura de la última fila de la izquierda.
        let (rx, rw) = (RIGHT_X + CP, RIGHT_W - 2 * CP);
        b.label(-1, tr("Vista previa", "Preview"), rx, CARD_Y + CP, rw);
        let inset = px(BEZEL);
        let (pw, top) = (px(rw) - 2 * inset, px(CARD_Y + CP + LABEL_H));
        let ph = pw * 9 / 16;
        let bezel = RECT { left: px(rx), top, right: px(rx) + px(rw), bottom: top + ph + 2 * inset };
        let host = b.add_px("STATIC", "", 0x4 /* SS_BLACKRECT */ | WS_CLIPCHILDREN, ID_PREVIEW, bezel.left + inset, bezel.top + inset, pw, ph);
        let half = (rw - GAP) / 2;
        b.label(-1, tr("Tamaño", "Size"), rx, y, half);
        b.combo(ID_SIZE, rx, y + LABEL_H, half, &[tr("Pequeño", "Small"), tr("Mediano", "Medium"), tr("Grande", "Large")].map(String::from), pos(&SIZES, "size", 1));
        b.label(-1, tr("Idioma del reloj", "Clock language"), rx + half + GAP, y, half);
        b.combo(ID_LANG, rx + half + GAP, y + LABEL_H, half, &[tr("Del sistema", "System"), "Español", "English"].map(String::from), pos(&LANGS, "lang", 0));

        // Letra del rótulo y, a la derecha, el modo calma.
        y += STEP;
        b.label(ID_LBL_FONT, tr("Letra del rótulo", "Banner typeface"), x, y, w);
        let fonts = [tr("Bloque", "Block"), tr("Bloque con sombra", "Block with shadow"), tr("Solo contorno", "Outline only"), tr("Píxel 8×8 (con minúsculas)", "8×8 pixel (with lowercase)")];
        let fpos = FONTS.iter().position(|f| f.0.eq_ignore_ascii_case(get("font").trim())).unwrap_or(0);
        b.combo(ID_FONT, x, y + LABEL_H, w, &fonts.map(String::from), fpos);
        b.label(-1, tr("Movimiento", "Motion"), rx, y, rw);
        let calm = matches!(get("calm").trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on") as usize;
        b.combo(ID_CALM, rx, y + LABEL_H, rw, &[tr("Normal", "Normal"), tr("Calma: a la mitad de velocidad", "Calm: half speed")].map(String::from), calm);

        // Última fila: cómo se reparten las pantallas y, al lado, qué número tiene cada una.
        y += STEP;
        b.label(-1, tr("Varias pantallas", "Multiple screens"), x, y, w);
        let modes = [tr("Un escritorio: los efectos cruzan", "One desktop: effects cross over"), tr("Cada pantalla por su cuenta", "Each screen on its own"), tr("Solo la principal (resto en negro)", "Main screen only (rest black)")];
        b.combo(ID_SCREENS, x, y + LABEL_H, w, &modes.map(String::from), pos(&SCREENS, "screens", 0));
        b.label(-1, tr("Orden: el de Configuración › Pantalla", "Order: as in Settings › Display"), rx, y, rw);
        b.button(ID_IDENTIFY, tr("Identificar pantallas", "Identify screens"), rx, y + LABEL_H, half, CTL_H);

        // Tarjeta inferior: usarlo como protector del sistema. A la izquierda el texto (que también
        // conmuta); a la derecha «tras N minutos» y el interruptor.
        let scr = scr_path();
        let active = reg_get("ScreenSaveActive").as_deref() == Some("1") && reg_get("SCRNSAVE.EXE").is_some_and(|c| sys::same_path(&c, &scr));
        ACTIVE.set(active);
        let mins = reg_get("ScreenSaveTimeOut").and_then(|v| v.parse::<u32>().ok()).map(|v| (v / 60).max(1)).unwrap_or(5);
        let act = tr("Usarlo como protector de pantalla", "Use it as my screensaver");
        let (after, minutes) = (tr("Tras", "After"), tr("minutos", "minutes"));
        let yc = ACT_Y + (ACT_H - CTL_H) / 2;
        let tx = W - PAD - CP - TOGGLE_W;
        let mw = b.text_w(minutes) + 2;
        let mx = tx - 16 - mw;
        let ex = mx - 8 - 52;
        let aw = b.text_w(after) + 2;
        let ax = ex - 8 - aw;
        let lw = (b.text_w(act) + 4).min(ax - 16 - x);
        b.text(ID_ACTIVATE_LABEL, act, 0x100 /* SS_NOTIFY */, x, ACT_Y + 6, lw);
        b.button(ID_ACTIVATE, act, tx, ACT_Y + (ACT_H - TOGGLE_H) / 2, TOGGLE_W, TOGGLE_H);
        b.text(ID_LBL_AFTER, after, 0, ax, yc, aw);
        b.edit(ID_MINUTES, ex, yc, 52, &mins.to_string(), 3, ES_NUMBER as u32 | 0x1 /* ES_CENTER */);
        b.text(ID_LBL_MINUTES, minutes, 0, mx, yc, mw);

        // Pie: probar a la izquierda; cancelar y guardar (con el color de acento) a la derecha.
        b.button(ID_TEST, tr("Probar ahora", "Try it now"), PAD, BUTTONS_Y, 128, CTL_H);
        b.button(IDCANCEL, tr("Cancelar", "Cancel"), W - PAD - 2 * 112 - 8, BUTTONS_Y, 112, CTL_H);
        b.button(IDOK, tr("Guardar", "Save"), W - PAD - 112, BUTTONS_Y, 112, CTL_H);
        // Cabecera, a la derecha: actualizaciones.
        b.button(ID_UPDATE, "", W - PAD - UPD_W, UPD_Y, UPD_W, CTL_H);
        set_upd(hwnd, format!("{} {}", tr("Versión", "Version"), update::VERSION), false, None);

        let accent = accent_color(pal.dark);
        UI.with(|u| {
            *u.borrow_mut() = Some(Ui {
                pal,
                s,
                accent,
                on_accent: if luma(accent) > 150 { rgb(0x000000) } else { rgb(0xFFFFFF) },
                font,
                font_small,
                font_title,
                bg,
                card: CreateSolidBrush(pal.card),
                field: CreateSolidBrush(pal.field),
                field_off: CreateSolidBrush(pal.field_off),
                icon,
                frames: b.frames.take(),
                bezel,
                preview_host: host,
                preview: None,
                preview_ini,
                scr,
            })
        });
        sync_enabled(hwnd);
        if let Some(o) = owner {
            EnableWindow(o, FALSE);
        }
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        restart_preview(hwnd);

        let mut msg: MSG = core::mem::zeroed();
        while GetMessageW(&mut msg, core::ptr::null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if let Some(o) = owner {
            EnableWindow(o, TRUE);
            SetForegroundWindow(o);
        }
        if let Some(u) = UI.with(|u| u.borrow_mut().take()) {
            let _ = std::fs::remove_file(&u.preview_ini);
            for o in [u.font as HGDIOBJ, u.font_small as HGDIOBJ, u.font_title as HGDIOBJ, u.bg as HGDIOBJ, u.card as HGDIOBJ, u.field as HGDIOBJ, u.field_off as HGDIOBJ] {
                DeleteObject(o);
            }
            DestroyIcon(u.icon);
        }
    }
}
