//! Envoltorio WebAssembly (wasm32-unknown-unknown, sin wasm-bindgen) del motor de Screensaver FX.
//!
//! Las cadenas se pasan como puntero + longitud en bytes UTF-8 (se reservan con `ssfxw_alloc`
//! desde JS y se liberan con `ssfxw_dealloc`); las cadenas devueltas son C (terminadas en NUL,
//! válidas mientras viva el módulo). El búfer de píxeles BGRX vale hasta el siguiente
//! `ssfxw_tick` o `ssfxw_free`.

use core::slice;
use ssfx_core::color::Rgb;
use ssfx_core::{Config, Engine, WallTime};

pub use ssfx_core::ffi::{ssfx_fx_count, ssfx_fx_id, ssfx_fx_name, ssfx_theme_count, ssfx_theme_id, ssfx_theme_name};

/// Reserva `len` bytes en la memoria del módulo para que JS escriba una cadena. Se libera con
/// `ssfxw_dealloc(ptr, len)`; el motor copia el contenido en `ssfxw_new`, así que la reserva se
/// puede liberar en cuanto esa llamada vuelve.
#[no_mangle]
pub extern "C" fn ssfxw_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    core::mem::forget(v);
    p
}

/// Libera una reserva de `ssfxw_alloc`. `ptr` puede ser NULL.
///
/// # Safety
/// `ptr` debe venir de `ssfxw_alloc(len)` y no haberse liberado ya.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Crea el motor. `ini` puede ser NULL: es un ini de `clave=valor` como el de `config.ini`
/// (p. ej. `effects=plasma,neon` y `theme=aero`); `lang_es` fija el idioma de la fecha del reloj.
///
/// # Safety
/// `ini` debe ser NULL o apuntar a `ini_len` bytes UTF-8 válidos. El resultado se libera con
/// `ssfxw_free`.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_new(px_w: i32, px_h: i32, ini: *const u8, ini_len: usize, seed: u64, lang_es: i32) -> *mut Engine {
    let ini = if ini.is_null() {
        String::new()
    } else {
        String::from_utf8_lossy(slice::from_raw_parts(ini, ini_len)).into_owned()
    };
    let cfg = Config::parse(&ini, Rgb::hex(0x0078_D4), seed, lang_es != 0);
    Box::into_raw(Box::new(Engine::new(px_w, px_h, cfg)))
}

/// Libera el motor.
///
/// # Safety
/// `e` debe ser NULL o un puntero de `ssfxw_new` que aún no se haya liberado.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_free(e: *mut Engine) {
    if !e.is_null() {
        drop(Box::from_raw(e));
    }
}

/// Avanza la animación a `t` segundos (monótono) y devuelve 1 si cambió algún píxel.
/// `now` apunta a 7 `int32_t`: año, mes, día, día de la semana (0 = lunes), hora, minuto y
/// segundo; puede ser NULL para usar una hora por defecto.
///
/// # Safety
/// `e` debe ser NULL o un motor vivo; `now`, NULL o un array de 7 enteros escribibles.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_tick(e: *mut Engine, t: f64, now: *const i32) -> i32 {
    let Some(e) = e.as_mut() else { return 0 };
    let now = if now.is_null() {
        WallTime::default()
    } else {
        let a = slice::from_raw_parts(now, 7);
        WallTime { year: a[0], month: a[1], day: a[2], weekday: a[3], hour: a[4], minute: a[5], second: a[6] }
    };
    e.tick(t, now) as i32
}

/// Búfer BGRX de 32 bits por píxel, sin relleno entre filas. Devuelve el puntero y, si no son
/// NULL, el ancho y alto del búfer. Válido hasta el siguiente `ssfxw_tick` o `ssfxw_free`.
///
/// # Safety
/// `e` debe ser NULL o un motor vivo; `w` y `h`, NULL o punteros a `int32_t` escribibles.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_pixels(e: *const Engine, w: *mut i32, h: *mut i32) -> *const u32 {
    let Some(e) = e.as_ref() else { return core::ptr::null() };
    let l = e.layout();
    if !w.is_null() {
        *w = l.buf_w;
    }
    if !h.is_null() {
        *h = l.buf_h;
    }
    e.pixels().as_ptr()
}

/// Rectángulo destino {x, y, w, h} en píxeles de la ventana (origen arriba a la izquierda).
/// Devuelve 1 si el búfer se reduce (conviene filtrado suave) y 0 si se amplía (vecino más cercano).
///
/// # Safety
/// `e` debe ser NULL o un motor vivo; `out`, NULL o un array de 4 `int32_t` escribibles.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_dest(e: *const Engine, out: *mut i32) -> i32 {
    let Some(e) = e.as_ref() else { return 0 };
    let l = e.layout();
    if !out.is_null() {
        let o = slice::from_raw_parts_mut(out, 4);
        o.copy_from_slice(&[l.dst_x, l.dst_y, l.dst_w, l.dst_h]);
    }
    l.scaled_down as i32
}

/// Copia el búfer como RGBA opaco (el orden de `ImageData`) en `dst`, que debe tener
/// `buf_w * buf_h * 4` bytes (reservados con `ssfxw_alloc`). Hacerlo aquí y no en JS ahorra un
/// bucle por píxel en cada fotograma. Devuelve 0 si no hay motor.
///
/// # Safety
/// `e` debe ser NULL o un motor vivo; `dst`, un puntero a `buf_w * buf_h * 4` bytes escribibles.
#[no_mangle]
pub unsafe extern "C" fn ssfxw_rgba(e: *const Engine, dst: *mut u8) -> i32 {
    let Some(e) = e.as_ref() else { return 0 };
    let src = e.pixels();
    let out = slice::from_raw_parts_mut(dst, src.len() * 4);
    for (o, &v) in out.chunks_exact_mut(4).zip(src) {
        o.copy_from_slice(&[(v >> 16) as u8, (v >> 8) as u8, v as u8, 0xFF]);
    }
    1
}
