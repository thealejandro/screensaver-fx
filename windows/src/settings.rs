//! Ventana de configuración (/c): rótulo, tema, tamaño, efectos e idioma. Guarda config.ini.

use crate::sys::{self, wide};
use ssfx_core::art::{default_text, MAX_SUBTITLE, MAX_TITLE};
use ssfx_core::color::THEMES;
use ssfx_core::fx::EFFECTS;
use std::cell::Cell;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
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

const BANNERS: [&str; 3] = ["default", "text", "file"];
const SIZES: [&str; 3] = ["s", "m", "l"];
const LANGS: [&str; 3] = ["auto", "es", "en"];

thread_local! {
    static ES: Cell<bool> = const { Cell::new(true) };
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

unsafe fn item(hwnd: HWND, id: i32) -> HWND {
    GetDlgItem(hwnd, id)
}

unsafe fn combo_sel(hwnd: HWND, id: i32) -> usize {
    SendMessageW(item(hwnd, id), CB_GETCURSEL, 0, 0).max(0) as usize
}

unsafe fn text_of(hwnd: HWND, id: i32) -> String {
    let h = item(hwnd, id);
    let n = GetWindowTextLengthW(h);
    let mut buf = vec![0u16; n as usize + 1];
    GetWindowTextW(h, buf.as_mut_ptr(), n + 1);
    String::from_utf16_lossy(&buf[..n as usize]).replace(['\r', '\n', '='], " ")
}

unsafe fn sync_enabled(hwnd: HWND) {
    let b = combo_sel(hwnd, ID_BANNER);
    EnableWindow(item(hwnd, ID_TITLE), (b == 1) as BOOL);
    EnableWindow(item(hwnd, ID_SUBTITLE), (b == 1) as BOOL);
    EnableWindow(item(hwnd, ID_EDIT_ART), (b == 2) as BOOL);
}

unsafe fn save(hwnd: HWND) -> bool {
    let (themes, effects) = (theme_values(), effect_values());
    let ini = format!(
        "# Screensaver FX\nbanner={}\ntitle={}\nsubtitle={}\ntheme={}\nsize={}\neffects={}\nlang={}\n",
        BANNERS[combo_sel(hwnd, ID_BANNER).min(2)],
        text_of(hwnd, ID_TITLE),
        text_of(hwnd, ID_SUBTITLE),
        themes[combo_sel(hwnd, ID_THEME).min(themes.len() - 1)],
        SIZES[combo_sel(hwnd, ID_SIZE).min(2)],
        effects[combo_sel(hwnd, ID_EFFECTS).min(effects.len() - 1)],
        LANGS[combo_sel(hwnd, ID_LANG).min(2)],
    );
    std::fs::create_dir_all(sys::data_dir()).is_ok() && std::fs::write(sys::ini_path(), ini).is_ok()
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

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let (id, code) = ((wp & 0xFFFF) as i32, (wp >> 16 & 0xFFFF) as u32);
            match id {
                ID_BANNER if code == CBN_SELCHANGE => sync_enabled(hwnd),
                ID_EDIT_ART => edit_art(hwnd),
                ID_TEST => {
                    if save(hwnd) {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).arg("/s").spawn();
                        }
                    }
                }
                IDOK => {
                    if save(hwnd) {
                        DestroyWindow(hwnd);
                    } else {
                        let (t, c) = (wide(tr("No se pudo guardar la configuración.", "Could not save the settings.")), wide("Screensaver FX"));
                        MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_ICONERROR);
                    }
                }
                IDCANCEL => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            return 0;
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

struct Builder {
    hwnd: HWND,
    hinst: HINSTANCE,
    font: HFONT,
    s: f32,
}

impl Builder {
    fn px(&self, v: i32) -> i32 {
        (v as f32 * self.s).round() as i32
    }

    unsafe fn add(&self, class: &str, text: &str, style: u32, ex: u32, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
        let (c, t) = (wide(class), wide(text));
        let ctl = CreateWindowExW(
            ex,
            c.as_ptr(),
            t.as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            self.px(x),
            self.px(y),
            self.px(w),
            self.px(h),
            self.hwnd,
            id as usize as HMENU,
            self.hinst,
            core::ptr::null(),
        );
        SendMessageW(ctl, WM_SETFONT, self.font as WPARAM, 1);
        ctl
    }

    unsafe fn label(&self, text: &str, y: i32) {
        self.add("STATIC", text, 0 /* SS_LEFT */, 0, -1, 16, y + 4, 150, 20);
    }

    unsafe fn combo(&self, id: i32, y: i32, items: &[String], sel: usize) {
        let c = self.add("COMBOBOX", "", (CBS_DROPDOWNLIST as u32) | WS_TABSTOP | WS_VSCROLL, 0, id, 170, y, 290, 320);
        for it in items {
            let w = wide(it);
            SendMessageW(c, CB_ADDSTRING, 0, w.as_ptr() as LPARAM);
        }
        SendMessageW(c, CB_SETCURSEL, sel.min(items.len().saturating_sub(1)), 0);
    }

    unsafe fn edit(&self, id: i32, y: i32, text: &str, max: usize) {
        let e = self.add("EDIT", text, (ES_AUTOHSCROLL as u32) | WS_TABSTOP, WS_EX_CLIENTEDGE, id, 170, y, 290, 24);
        SendMessageW(e, 0x00C5 /* EM_LIMITTEXT */, max, 0);
    }

    unsafe fn button(&self, id: i32, text: &str, x: i32, y: i32, w: i32, default: bool) {
        let style = if default { BS_DEFPUSHBUTTON } else { BS_PUSHBUTTON } as u32;
        self.add("BUTTON", text, style | WS_TABSTOP, 0, id, x, y, w, 28);
    }
}

pub fn run(owner: Option<HWND>) {
    unsafe {
        ES.set(sys::system_is_spanish());
        let hinst = GetModuleHandleW(core::ptr::null());
        let class = wide("ScreensaverFXSettings");
        let wc = WNDCLASSEXW {
            cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hCursor: LoadCursorW(core::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_BTNFACE + 1) as usize as HBRUSH,
            lpszClassName: class.as_ptr(),
            ..core::mem::zeroed()
        };
        RegisterClassExW(&wc);

        let dpi = GetDpiForSystem().max(96);
        let s = dpi as f32 / 96.0;
        let (cw, ch) = ((476.0 * s) as i32, (372.0 * s) as i32);
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut rc = RECT { left: 0, top: 0, right: cw, bottom: ch };
        AdjustWindowRect(&mut rc, style, FALSE);
        let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let title = wide(tr("Screensaver FX · Configuración", "Screensaver FX · Settings"));
        let owner_h = owner.unwrap_or(core::ptr::null_mut());
        let hwnd = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), style, (sw - ww) / 2, (sh - wh) / 2, ww, wh, owner_h, core::ptr::null_mut(), hinst, core::ptr::null());
        if hwnd.is_null() {
            return;
        }
        let face = wide("Segoe UI");
        let font = CreateFontW(-((9 * dpi as i32) / 72), 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, face.as_ptr());
        let b = Builder { hwnd, hinst, font, s };

        let m = read_map();
        let get = |k: &str| m.get(k).cloned().unwrap_or_default();
        let pos = |vals: &[&str], k: &str, def: usize| vals.iter().position(|v| v.eq_ignore_ascii_case(&get(k))).unwrap_or(def);
        let es = ES.get();

        let mut y = 16;
        b.label(tr("Rótulo", "Banner"), y);
        let banners = [tr("JACC · XACODE (por defecto)", "JACC · XACODE (default)"), tr("Texto personalizado", "Custom text"), tr("Arte ASCII propio (screensaver.txt)", "Own ASCII art (screensaver.txt)")];
        b.combo(ID_BANNER, y, &banners.map(String::from), pos(&BANNERS, "banner", 0));
        y += 34;
        b.label(tr("Título", "Title"), y);
        b.edit(ID_TITLE, y, &get("title"), MAX_TITLE);
        y += 32;
        b.label(tr("Subtítulo", "Subtitle"), y);
        b.edit(ID_SUBTITLE, y, &get("subtitle"), MAX_SUBTITLE);
        y += 32;
        b.button(ID_EDIT_ART, tr("Editar screensaver.txt…", "Edit screensaver.txt…"), 170, y, 290, false);
        y += 44;

        b.label(tr("Colores", "Colors"), y);
        let mut themes = vec![tr("Color de acento de Windows", "Windows accent color").to_string(), tr("Aleatorio (cambia en cada efecto)", "Random (changes every effect)").to_string()];
        themes.extend(THEMES.iter().map(|t| t.name_str().to_string()));
        let tv = theme_values();
        let tsel = tv.iter().position(|v| v.eq_ignore_ascii_case(&get("theme"))).unwrap_or(0);
        b.combo(ID_THEME, y, &themes, tsel);
        y += 34;
        b.label(tr("Tamaño", "Size"), y);
        b.combo(ID_SIZE, y, &[tr("Pequeño", "Small"), tr("Mediano", "Medium"), tr("Grande", "Large")].map(String::from), pos(&SIZES, "size", 1));
        y += 34;
        b.label(tr("Efectos", "Effects"), y);
        let mut fx = vec![tr("Todos", "All").to_string(), tr("Clásicos de Omarchy", "Omarchy classics").to_string(), tr("Solo los nuevos", "New ones only").to_string()];
        fx.extend(EFFECTS.iter().map(|f| {
            let n = if es { f.name_es } else { f.name_en };
            format!("{} {}", tr("Solo:", "Only:"), n.to_str().unwrap_or(""))
        }));
        let ev = effect_values();
        let fsel = ev.iter().position(|v| v.eq_ignore_ascii_case(&get("effects"))).unwrap_or(0);
        b.combo(ID_EFFECTS, y, &fx, fsel);
        y += 34;
        b.label(tr("Idioma del reloj", "Clock language"), y);
        b.combo(ID_LANG, y, &[tr("Del sistema", "System"), "Español", "English"].map(String::from), pos(&LANGS, "lang", 0));
        y += 50;

        b.button(ID_TEST, tr("Probar", "Preview"), 16, y, 100, false);
        b.button(IDCANCEL, tr("Cancelar", "Cancel"), 254, y, 100, false);
        b.button(IDOK, tr("Guardar", "Save"), 360, y, 100, true);

        sync_enabled(hwnd);
        if let Some(o) = owner {
            EnableWindow(o, FALSE);
        }
        ShowWindow(hwnd, SW_SHOW);
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
        DeleteObject(font);
    }
}
