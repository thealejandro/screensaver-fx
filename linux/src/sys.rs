//! Lo poco que hace falta del sistema (tamaño de la terminal, señales y hora local), declarado
//! a mano contra la libc para no añadir dependencias.

use core::ffi::{c_char, c_int, c_long, c_ulong};
use ssfx_core::WallTime;
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
#[derive(Default)]
struct Winsize {
    row: u16,
    col: u16,
    xpixel: u16,
    ypixel: u16,
}

#[repr(C)]
struct Tm {
    sec: c_int,
    min: c_int,
    hour: c_int,
    mday: c_int,
    mon: c_int,
    year: c_int,
    wday: c_int,
    yday: c_int,
    isdst: c_int,
    gmtoff: c_long,
    zone: *const c_char,
}

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
    fn time(t: *mut c_long) -> c_long;
    fn localtime_r(t: *const c_long, tm: *mut Tm) -> *mut Tm;
    fn getppid() -> c_int;
    #[cfg(target_os = "linux")]
    fn prctl(option: c_int, ...) -> c_int;
}

#[cfg(target_os = "linux")]
const TIOCGWINSZ: c_ulong = 0x5413;
#[cfg(not(target_os = "linux"))]
const TIOCGWINSZ: c_ulong = 0x4008_7468;

const SIGHUP: c_int = 1;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_sig: c_int) {
    STOP.store(true, Ordering::SeqCst);
}

/// Tamaño de la terminal en (columnas, filas), si la salida es una terminal.
pub fn terminal_size() -> Option<(i32, i32)> {
    let mut ws = Winsize::default();
    // SAFETY: TIOCGWINSZ escribe un `winsize` en el puntero que recibe; `ws` vive toda la llamada.
    let ok = (1..=2).any(|fd| unsafe { ioctl(fd, TIOCGWINSZ, &mut ws as *mut Winsize) } == 0 && ws.col > 0 && ws.row > 0);
    ok.then_some((ws.col as i32, ws.row as i32))
}

/// Deja anotadas las señales de cierre para que el bucle restaure la terminal antes de salir.
pub fn catch_signals() {
    for sig in [SIGHUP, SIGINT, SIGTERM] {
        // SAFETY: el manejador solo escribe un AtomicBool, que es seguro dentro de una señal.
        unsafe { signal(sig, on_signal as extern "C" fn(c_int) as usize) };
    }
}

pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Pide morir junto con el proceso que nos lanzó (el envoltorio de ttfx). Devuelve false si
/// ese proceso ya no existe.
pub fn die_with_parent() -> bool {
    #[cfg(target_os = "linux")]
    {
        const PR_SET_PDEATHSIG: c_int = 1;
        // SAFETY: prctl(PR_SET_PDEATHSIG, señal) solo cambia un ajuste de este proceso.
        unsafe { prctl(PR_SET_PDEATHSIG, SIGTERM as c_ulong) };
    }
    // SAFETY: getppid no tiene condiciones.
    unsafe { getppid() != 1 }
}

/// Hora local de pared para el efecto del reloj.
pub fn now() -> WallTime {
    // SAFETY: time acepta NULL; localtime_r escribe en `tm`, que es nuestro, y `zeroed` es un
    // valor válido para una estructura de enteros y un puntero.
    unsafe {
        let t = time(core::ptr::null_mut());
        let mut tm: Tm = core::mem::zeroed();
        if localtime_r(&t, &mut tm).is_null() {
            return WallTime::default();
        }
        WallTime { year: tm.year + 1900, month: tm.mon + 1, day: tm.mday, weekday: (tm.wday + 6) % 7, hour: tm.hour, minute: tm.min, second: tm.sec }
    }
}
