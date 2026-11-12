//! Modo protector (/s) y vista previa (/p): una ventana por monitor, cada una con su motor.
//! Dibuja con StretchDIBits solo el rectángulo que cambió, se sincroniza con DwmFlush y se
//! detiene mientras Windows tiene la pantalla apagada.

use crate::sys;
use ssfx_core::color::Rgb;
use ssfx_core::desk::Screen;
use ssfx_core::{Config, Engine};
use std::cell::{Cell, RefCell};
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, RegisterPowerSettingNotification, UnregisterPowerSettingNotification, POWERBROADCAST_SETTING, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::Sleep;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct Saver {
    hwnd: HWND,
    hdc: HDC,
    engine: Engine,
}

thread_local! {
    static SAVERS: RefCell<Vec<Saver>> = const { RefCell::new(Vec::new()) };
    static PREVIEW: Cell<bool> = const { Cell::new(false) };
    static START: Cell<POINT> = const { Cell::new(POINT { x: 0, y: 0 }) };
    /// Windows apagó la pantalla: no se dibuja nada hasta que vuelva.
    static DISPLAY_OFF: Cell<bool> = const { Cell::new(false) };
}

/// Fotogramas como mucho cada 12,5 ms: 60 fps en 60/120 Hz, ~72 en 144 Hz.
const MIN_FRAME: f64 = 0.0125;
/// Con el portátil desenchufado, como mucho ~30 fps: algo menos de dos cuadros a 60 Hz para que
/// no caiga a 20 por un milisegundo de más. Enchufado va a la velocidad de siempre.
const BATTERY_FRAME: f64 = 0.030;
/// Cada cuánto se mira si el equipo sigue con batería.
const POWER_CHECK: f64 = 5.0;

/// El equipo funciona con batería (un sobremesa, o si Windows no lo sabe, cuenta como enchufado).
fn on_battery() -> bool {
    let mut ps: SYSTEM_POWER_STATUS = unsafe { core::mem::zeroed() };
    let ok = unsafe { GetSystemPowerStatus(&mut ps) } != 0;
    ok && ps.ACLineStatus == 0
}
const MOUSE_SLOP: i32 = 6;
/// GUID_CONSOLE_DISPLAY_STATE: avisa cuando la pantalla se apaga (0), se enciende (1) o se atenúa (2).
const DISPLAY_STATE: windows_sys::core::GUID = windows_sys::core::GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47);

/// Perfilado opcional (`SSFX_PROFILE=<archivo>`): al salir escribe p50/p99/máximo del motor, del
/// dibujado y del intervalo entre cuadros, y cuántos cuadros llegaron tarde (> 1,5 veces el
/// intervalo habitual), que son los tirones que se notan. Sin la variable no hace nada.
struct Profile {
    path: std::path::PathBuf,
    tick: Vec<f32>,
    blit: Vec<f32>,
    gap: Vec<f32>,
}

impl Profile {
    fn from_env() -> Option<Profile> {
        let path = std::env::var_os("SSFX_PROFILE").filter(|p| !p.is_empty())?;
        Some(Profile { path: path.into(), tick: Vec::with_capacity(8192), blit: Vec::with_capacity(8192), gap: Vec::with_capacity(8192) })
    }

    fn write(mut self) {
        use std::fmt::Write;
        let mut out = String::new();
        let mut line = |name: &str, v: &mut Vec<f32>| {
            if v.is_empty() {
                return;
            }
            v.sort_by(|a, b| a.total_cmp(b));
            let p = |q: f32| v[((v.len() - 1) as f32 * q).round() as usize];
            let _ = writeln!(out, "{name:<8} n={:<6} p50={:.3} ms p99={:.3} ms máx={:.3} ms", v.len(), p(0.5), p(0.99), v[v.len() - 1]);
        };
        line("motor", &mut self.tick);
        line("dibujo", &mut self.blit);
        let typical = {
            let mut g = self.gap.clone();
            g.sort_by(|a, b| a.total_cmp(b));
            g.get(g.len() / 2).copied().unwrap_or(0.0)
        };
        let late = self.gap.iter().filter(|&&g| g > typical * 1.5).count();
        line("cuadro", &mut self.gap);
        let _ = writeln!(out, "tarde    {late} de {} cuadros (> {:.1} ms)", self.gap.len(), typical * 1.5);
        let _ = std::fs::write(&self.path, out);
    }
}

unsafe extern "system" fn monitor_proc(_: HMONITOR, _: HDC, rc: *mut RECT, data: LPARAM) -> BOOL {
    (*(data as *mut Vec<RECT>)).push(*rc);
    TRUE
}

fn quit() {
    unsafe { PostQuitMessage(0) };
}

/// Ganchos de bajo nivel: salen con cualquier tecla, clic o movimiento aunque la ventana no
/// tenga el foco (Windows no deja tomarlo a un proceso lanzado por el vigilante). La tecla o el
/// clic que despierta se consume, como en el protector del sistema, para que no llegue a la app.
unsafe extern "system" fn key_hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // Solo al pulsar: soltar la tecla que lanzó el protector (Enter en «Probar») no lo cierra.
    if code == HC_ACTION as i32 && matches!(wp as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
        quit();
        return 1;
    }
    CallNextHookEx(core::ptr::null_mut(), code, wp, lp)
}

unsafe extern "system" fn mouse_hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        match wp as u32 {
            WM_MOUSEMOVE => {
                let p = (*(lp as *const MSLLHOOKSTRUCT)).pt;
                let s = START.get();
                if (p.x - s.x).abs() > MOUSE_SLOP || (p.y - s.y).abs() > MOUSE_SLOP {
                    quit();
                }
            }
            _ => {
                quit();
                return 1;
            }
        }
    }
    CallNextHookEx(core::ptr::null_mut(), code, wp, lp)
}

/// Copia el búfer a la ventana. `full`: todo el búfer y bordes en negro (WM_PAINT).
unsafe fn blit(s: &Saver, hdc: HDC, full: bool) {
    let l = s.engine.layout();
    if full {
        let mut rc: RECT = core::mem::zeroed();
        GetClientRect(s.hwnd, &mut rc);
        FillRect(hdc, &rc, GetStockObject(BLACK_BRUSH));
    }
    if l.scaled_down {
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, core::ptr::null_mut());
    } else {
        SetStretchBltMode(hdc, COLORONCOLOR);
    }
    match (full, l.int_scale()) {
        // Con escala entera se copian solo las franjas que cambiaron.
        (false, Some(k)) => {
            for &(x, y, w, h) in s.engine.dirty_rects() {
                let (x, y, w, h) = (x as i32, y as i32, w as i32, h as i32);
                copy(s, hdc, (x, y, w, h), (l.dst_x + x * k, l.dst_y + y * k, w * k, h * k));
            }
        }
        _ => copy(s, hdc, (0, 0, l.buf_w, l.buf_h), (l.dst_x, l.dst_y, l.dst_w, l.dst_h)),
    }
}

/// Copia el trozo `src` del búfer (x, y, w, h) al trozo `dst` de la ventana.
unsafe fn copy(s: &Saver, hdc: HDC, (sx, sy, sw, sh): (i32, i32, i32, i32), (dx, dy, dw, dh): (i32, i32, i32, i32)) {
    let l = s.engine.layout();
    // DIB descendente que empieza en la fila `sy`: así el origen de la fuente es siempre (sx, 0).
    let mut bmi: BITMAPINFO = core::mem::zeroed();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: l.buf_w,
        biHeight: -sh,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..core::mem::zeroed()
    };
    let bits = s.engine.pixels().as_ptr().add((sy * l.buf_w) as usize);
    StretchDIBits(hdc, dx, dy, dw, dh, sx, 0, sw, sh, bits as *const _, &bmi, DIB_RGB_COLORS, SRCCOPY);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let preview = PREVIEW.get();
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = core::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            SAVERS.with(|v| {
                if let Ok(v) = v.try_borrow() {
                    if let Some(s) = v.iter().find(|s| s.hwnd == hwnd) {
                        blit(s, hdc, true);
                    }
                }
            });
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_ERASEBKGND => return 1,
        WM_SETCURSOR if !preview => {
            SetCursor(core::ptr::null_mut());
            return 1;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN | WM_MOUSEWHEEL if !preview => {
            quit();
            return 0;
        }
        WM_MOUSEMOVE if !preview => {
            let mut p: POINT = core::mem::zeroed();
            GetCursorPos(&mut p);
            let s = START.get();
            if (p.x - s.x).abs() > MOUSE_SLOP || (p.y - s.y).abs() > MOUSE_SLOP {
                quit();
            }
            return 0;
        }
        WM_POWERBROADCAST if wp as u32 == PBT_POWERSETTINGCHANGE && lp != 0 => {
            let s = &*(lp as *const POWERBROADCAST_SETTING);
            let (g, d) = (&s.PowerSetting, &DISPLAY_STATE);
            if (g.data1, g.data2, g.data3, g.data4) == (d.data1, d.data2, d.data3, d.data4) && s.DataLength >= 1 {
                DISPLAY_OFF.set(s.Data[0] == 0);
            }
            return 1;
        }
        WM_ACTIVATEAPP if !preview && wp == 0 => quit(),
        // Se conectó o quitó un monitor o cambió la resolución: las ventanas ya no cubren bien el
        // escritorio. Se cierra y, pasado el tiempo de espera, Windows lo relanza con la nueva
        // disposición. Con la pantalla apagada no: algunos monitores DisplayPort se «desconectan» al dormir.
        WM_DISPLAYCHANGE if !preview && !DISPLAY_OFF.get() => quit(),
        WM_SYSCOMMAND if !preview && matches!((wp & 0xFFF0) as u32, SC_SCREENSAVE | SC_CLOSE) => return 0,
        WM_CLOSE => {
            quit();
            return 0;
        }
        WM_DESTROY => quit(),
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

/// `windowed`: modo de desarrollo (/w), ventana normal de 1280x720 que no se cierra con el ratón.
pub fn run(parent: Option<HWND>, windowed: bool) {
    unsafe {
        let preview = parent.is_some() || windowed;
        PREVIEW.set(preview);
        let hinst = GetModuleHandleW(core::ptr::null());
        let class = sys::wide("ScreensaverFXWindow");
        let title = sys::wide("Screensaver FX");
        let wc = WNDCLASSEXW {
            cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_OWNDC,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hCursor: if preview { LoadCursorW(core::ptr::null_mut(), IDC_ARROW) } else { core::ptr::null_mut() },
            hbrBackground: GetStockObject(BLACK_BRUSH),
            lpszClassName: class.as_ptr(),
            ..core::mem::zeroed()
        };
        RegisterClassExW(&wc);

        let mut rects: Vec<RECT> = Vec::new();
        match parent {
            Some(p) => {
                let mut rc: RECT = core::mem::zeroed();
                GetClientRect(p, &mut rc);
                rects.push(rc);
            }
            None if windowed => {
                // SSFX_WINDOW=1920x1080: otro tamaño de ventana, para medir sin pantalla completa.
                let (w, h) = std::env::var("SSFX_WINDOW")
                    .ok()
                    .and_then(|s| s.split_once('x').and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?))))
                    .filter(|&(w, h): &(i32, i32)| (64..=8192).contains(&w) && (64..=8192).contains(&h))
                    .unwrap_or((1280, 720));
                rects.push(RECT { left: 0, top: 0, right: w, bottom: h });
            }
            None => {
                EnumDisplayMonitors(core::ptr::null_mut(), core::ptr::null(), Some(monitor_proc), &mut rects as *mut _ as LPARAM);
                // La principal (la que empieza en 0,0) va primero: es la que anima con screens=main.
                if let Some(i) = rects.iter().position(|r| r.left == 0 && r.top == 0) {
                    rects[..=i].rotate_right(1);
                }
            }
        }

        let (ini, art, accent, es, seed) = (sys::read_ini(), sys::read_art(), Rgb::hex(sys::accent()), sys::system_is_spanish(), sys::seed());
        // Todos los monitores forman un solo escritorio con la misma semilla y el mismo reloj: van
        // sincronizados y los efectos de escritorio pasan de una pantalla a la siguiente.
        let screens: Vec<Screen> = rects
            .iter()
            .map(|r| {
                let (w, h) = (r.right - r.left, r.bottom - r.top);
                Screen { x: r.left as f64, y: r.top as f64, w: w as f64, h: h as f64, px_w: w, px_h: h }
            })
            .collect();
        for (i, rc) in rects.iter().enumerate() {
            let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
            if w <= 0 || h <= 0 {
                continue;
            }
            let hwnd = match parent {
                Some(p) => CreateWindowExW(0, class.as_ptr(), title.as_ptr(), WS_CHILD | WS_VISIBLE, 0, 0, w, h, p, core::ptr::null_mut(), hinst, core::ptr::null()),
                None if windowed => {
                    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE;
                    let mut r = *rc;
                    AdjustWindowRect(&mut r, style, FALSE);
                    CreateWindowExW(0, class.as_ptr(), title.as_ptr(), style, 80, 80, r.right - r.left, r.bottom - r.top, core::ptr::null_mut(), core::ptr::null_mut(), hinst, core::ptr::null())
                }
                None => CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    class.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP | WS_VISIBLE,
                    rc.left,
                    rc.top,
                    w,
                    h,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    hinst,
                    core::ptr::null(),
                ),
            };
            if hwnd.is_null() {
                continue;
            }
            let mut cfg = Config::parse(&ini, accent, seed, es);
            cfg.art = art.clone();
            let engine = Engine::with_desk(&screens, i, cfg);
            SAVERS.with(|v| v.borrow_mut().push(Saver { hwnd, hdc: GetDC(hwnd), engine }));
        }
        let first = SAVERS.with(|v| v.borrow().first().map(|s| s.hwnd));
        let Some(first) = first else { return };
        let mut hooks: [HHOOK; 2] = [core::ptr::null_mut(); 2];
        if !preview {
            let mut p: POINT = core::mem::zeroed();
            GetCursorPos(&mut p);
            START.set(p);
            ShowCursor(FALSE);
            SetForegroundWindow(first);
            SetFocus(first);
            hooks = [SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_hook), hinst, 0), SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), hinst, 0)];
        }

        // Con la pantalla apagada no tiene sentido dibujar: el bucle espera mensajes sin gastar CPU.
        let power = RegisterPowerSettingNotification(first as HANDLE, &DISPLAY_STATE, DEVICE_NOTIFY_WINDOW_HANDLE);

        let mut prof = Profile::from_env();
        let t0 = Instant::now();
        let mut last = f64::NEG_INFINITY;
        let (mut battery, mut power_checked) = (false, f64::NEG_INFINITY);
        let mut msg: MSG = core::mem::zeroed();
        'main: loop {
            while PeekMessageW(&mut msg, core::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'main;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if let Some(p) = parent {
                if IsWindow(p) == 0 {
                    break;
                }
            }
            if DISPLAY_OFF.get() {
                // Al volver la pantalla se repinta todo y el motor se pone al día con el reloj.
                WaitMessage();
                if !DISPLAY_OFF.get() {
                    SAVERS.with(|v| v.borrow().iter().for_each(|s| _ = InvalidateRect(s.hwnd, core::ptr::null(), FALSE)));
                    last = f64::NEG_INFINITY;
                }
                continue;
            }
            let t = t0.elapsed().as_secs_f64();
            if t - power_checked >= POWER_CHECK {
                power_checked = t;
                battery = on_battery();
            }
            if t - last >= if battery { BATTERY_FRAME } else { MIN_FRAME } {
                if let Some(p) = prof.as_mut().filter(|_| last.is_finite()) {
                    p.gap.push(((t - last) * 1000.0) as f32);
                }
                last = t;
                let now = sys::now();
                SAVERS.with(|v| {
                    for s in v.borrow_mut().iter_mut() {
                        let a = Instant::now();
                        let changed = s.engine.tick(t, now);
                        let b = Instant::now();
                        if changed {
                            blit(s, s.hdc, false);
                        }
                        if let Some(p) = prof.as_mut() {
                            p.tick.push((b - a).as_secs_f32() * 1000.0);
                            if changed {
                                p.blit.push(b.elapsed().as_secs_f32() * 1000.0);
                            }
                        }
                    }
                });
            }
            if DwmFlush() < 0 {
                Sleep(8);
            }
        }
        if power != 0 {
            UnregisterPowerSettingNotification(power);
        }
        for h in hooks.into_iter().filter(|h| !h.is_null()) {
            UnhookWindowsHookEx(h);
        }
        if !preview {
            ShowCursor(TRUE);
        }
        if let Some(p) = prof {
            p.write();
        }
    }
}
