//! ABI de C para el envoltorio de macOS (Swift). Ver include/ssfx.h.

use crate::color::{Rgb, THEMES};
use crate::desk::Screen;
use crate::fx::EFFECTS;
use crate::{Config, Engine, WallTime};
use core::ffi::{c_char, CStr};
use core::ptr;

unsafe fn opt_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

/// Crea el motor. `ini` y `art` pueden ser NULL. `accent` es 0x00RRGGBB.
///
/// # Safety
/// `ini` y `art` deben ser NULL o cadenas C válidas terminadas en NUL. El resultado se libera
/// con `ssfx_free`.
#[no_mangle]
pub unsafe extern "C" fn ssfx_new(px_w: i32, px_h: i32, ini: *const c_char, art: *const c_char, accent: u32, seed: u64, sys_lang_es: i32) -> *mut Engine {
    let mut cfg = Config::parse(&opt_str(ini).unwrap_or_default(), Rgb::hex(accent), seed, sys_lang_es != 0);
    cfg.art = opt_str(art).filter(|s| !s.trim().is_empty());
    Box::into_raw(Box::new(Engine::new(px_w, px_h, cfg)))
}

/// Como `ssfx_new`, pero para la pantalla `me` de un escritorio con `n` pantallas. Todas las
/// pantallas deben usar la misma semilla y el mismo `t` para ir sincronizadas.
///
/// # Safety
/// Lo de `ssfx_new`; además `screens` debe ser NULL o apuntar a `n` `SsfxScreen` válidos.
#[no_mangle]
pub unsafe extern "C" fn ssfx_new_desk(screens: *const Screen, n: i32, me: i32, ini: *const c_char, art: *const c_char, accent: u32, seed: u64, sys_lang_es: i32) -> *mut Engine {
    let mut cfg = Config::parse(&opt_str(ini).unwrap_or_default(), Rgb::hex(accent), seed, sys_lang_es != 0);
    cfg.art = opt_str(art).filter(|s| !s.trim().is_empty());
    let list: &[Screen] = if screens.is_null() || n <= 0 { &[] } else { core::slice::from_raw_parts(screens, n.min(64) as usize) };
    Box::into_raw(Box::new(Engine::with_desk(list, me.max(0) as usize, cfg)))
}

/// Libera el motor.
///
/// # Safety
/// `e` debe ser NULL o un puntero de `ssfx_new` que aún no se haya liberado.
#[no_mangle]
pub unsafe extern "C" fn ssfx_free(e: *mut Engine) {
    if !e.is_null() {
        drop(Box::from_raw(e));
    }
}

/// Devuelve 1 si cambió algún píxel.
///
/// # Safety
/// `e` debe ser NULL o un motor vivo de `ssfx_new`; `now`, NULL o un `SsfxWallTime` válido.
#[no_mangle]
pub unsafe extern "C" fn ssfx_tick(e: *mut Engine, t: f64, now: *const WallTime) -> i32 {
    match e.as_mut() {
        Some(e) => e.tick(t, if now.is_null() { WallTime::default() } else { *now }) as i32,
        None => 0,
    }
}

/// Búfer BGRX de 32 bits por píxel, sin relleno entre filas.
///
/// # Safety
/// `e` debe ser NULL o un motor vivo; `w` y `h`, NULL o punteros a `int32_t` escribibles. El
/// búfer devuelto vale hasta el siguiente `ssfx_tick` o `ssfx_free`.
#[no_mangle]
pub unsafe extern "C" fn ssfx_pixels(e: *const Engine, w: *mut i32, h: *mut i32) -> *const u32 {
    let Some(e) = e.as_ref() else { return ptr::null() };
    let l = e.layout();
    if !w.is_null() {
        *w = l.buf_w;
    }
    if !h.is_null() {
        *h = l.buf_h;
    }
    e.pixels().as_ptr()
}

/// Rectángulo destino {x, y, w, h} en píxeles de pantalla (origen arriba a la izquierda).
/// Devuelve 1 si el búfer se reduce (conviene filtrado suave) y 0 si se amplía (vecino más cercano).
///
/// # Safety
/// `e` debe ser NULL o un motor vivo; `out`, NULL o un array de 4 `int32_t` escribibles.
#[no_mangle]
pub unsafe extern "C" fn ssfx_dest(e: *const Engine, out: *mut i32) -> i32 {
    let Some(e) = e.as_ref() else { return 0 };
    let l = e.layout();
    if !out.is_null() {
        let o = core::slice::from_raw_parts_mut(out, 4);
        o.copy_from_slice(&[l.dst_x, l.dst_y, l.dst_w, l.dst_h]);
    }
    l.scaled_down as i32
}

#[no_mangle]
pub extern "C" fn ssfx_theme_count() -> i32 {
    THEMES.len() as i32
}

#[no_mangle]
pub extern "C" fn ssfx_theme_id(i: i32) -> *const c_char {
    THEMES.get(i as usize).map_or(ptr::null(), |t| t.id.as_ptr())
}

#[no_mangle]
pub extern "C" fn ssfx_theme_name(i: i32) -> *const c_char {
    THEMES.get(i as usize).map_or(ptr::null(), |t| t.name.as_ptr())
}

#[no_mangle]
pub extern "C" fn ssfx_fx_count() -> i32 {
    EFFECTS.len() as i32
}

#[no_mangle]
pub extern "C" fn ssfx_fx_id(i: i32) -> *const c_char {
    EFFECTS.get(i as usize).map_or(ptr::null(), |f| f.id.as_ptr())
}

#[no_mangle]
pub extern "C" fn ssfx_fx_name(i: i32, es: i32) -> *const c_char {
    EFFECTS.get(i as usize).map_or(ptr::null(), |f| if es != 0 { f.name_es.as_ptr() } else { f.name_en.as_ptr() })
}
