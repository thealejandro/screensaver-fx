//! Modo protector (/s) y vista previa (/p): una ventana por monitor, cada una con su motor.
//! Dibuja con StretchDIBits solo el rectángulo que cambió y se sincroniza con DwmFlush.

use crate::sys;
use ssfx_core::color::Rgb;
use ssfx_core::{Config, Engine};
use std::cell::{Cell, RefCell};
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::*;
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
}

/// Fotogramas como mucho cada 12,5 ms: 60 fps en 60/120 Hz, ~72 en 144 Hz.
const MIN_FRAME: f64 = 0.0125;
const MOUSE_SLOP: i32 = 6;

unsafe extern "system" fn monitor_proc(_: HMONITOR, _: HDC, rc: *mut RECT, data: LPARAM) -> BOOL {
    (*(data as *mut Vec<RECT>)).push(*rc);
    TRUE
}

fn quit() {
    unsafe { PostQuitMessage(0) };
}

/// Copia el búfer a la ventana. `full`: todo el búfer y bordes en negro (WM_PAINT).
unsafe fn blit(s: &Saver, hdc: HDC, full: bool) {
    let l = s.engine.layout();
    let px = s.engine.pixels();
    if full {
        let mut rc: RECT = core::mem::zeroed();
        GetClientRect(s.hwnd, &mut rc);
        FillRect(hdc, &rc, GetStockObject(BLACK_BRUSH));
    }
    let (sx, sy, sw, sh, dx, dy, dw, dh) = match (full, l.int_scale(), s.engine.dirty()) {
        (false, Some(k), Some((x, y, w, h))) => {
            let (x, y, w, h) = (x as i32, y as i32, w as i32, h as i32);
            (x, y, w, h, l.dst_x + x * k, l.dst_y + y * k, w * k, h * k)
        }
        _ => (0, 0, l.buf_w, l.buf_h, l.dst_x, l.dst_y, l.dst_w, l.dst_h),
    };
    if l.scaled_down {
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, core::ptr::null_mut());
    } else {
        SetStretchBltMode(hdc, COLORONCOLOR);
    }
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
    let bits = px.as_ptr().add((sy * l.buf_w) as usize);
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
        WM_ACTIVATEAPP if !preview && wp == 0 => quit(),
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
            None if windowed => rects.push(RECT { left: 0, top: 0, right: 1280, bottom: 720 }),
            None => {
                EnumDisplayMonitors(core::ptr::null_mut(), core::ptr::null(), Some(monitor_proc), &mut rects as *mut _ as LPARAM);
            }
        }

        let (ini, art, accent, es, seed) = (sys::read_ini(), sys::read_art(), Rgb::hex(sys::accent()), sys::system_is_spanish(), sys::seed());
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
            // Cada monitor con su propia semilla: no repiten el mismo efecto a la vez.
            let mut cfg = Config::parse(&ini, accent, seed.wrapping_add(i as u64 * 0x9E37_79B9_7F4A_7C15), es);
            cfg.art = art.clone();
            let engine = Engine::new(w, h, cfg);
            SAVERS.with(|v| v.borrow_mut().push(Saver { hwnd, hdc: GetDC(hwnd), engine }));
        }
        let first = SAVERS.with(|v| v.borrow().first().map(|s| s.hwnd));
        let Some(first) = first else { return };
        if !preview {
            let mut p: POINT = core::mem::zeroed();
            GetCursorPos(&mut p);
            START.set(p);
            ShowCursor(FALSE);
            SetForegroundWindow(first);
            SetFocus(first);
        }

        let t0 = Instant::now();
        let mut last = f64::NEG_INFINITY;
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
            let t = t0.elapsed().as_secs_f64();
            if t - last >= MIN_FRAME {
                last = t;
                let now = sys::now();
                SAVERS.with(|v| {
                    for s in v.borrow_mut().iter_mut() {
                        if s.engine.tick(t, now) {
                            blit(s, s.hdc, false);
                        }
                    }
                });
            }
            if DwmFlush() < 0 {
                Sleep(8);
            }
        }
        if !preview {
            ShowCursor(TRUE);
        }
    }
}
