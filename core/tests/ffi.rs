//! ABI de C (macOS): se llama igual que desde Swift, con punteros crudos.

use ssfx_core::color::THEMES;
use ssfx_core::ffi::*;
use ssfx_core::fx::EFFECTS;
use ssfx_core::WallTime;
use std::ffi::{CStr, CString};
use std::ptr;

const NOW: WallTime = WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 };

fn cstr<'a>(p: *const std::ffi::c_char) -> &'a str {
    assert!(!p.is_null());
    unsafe { CStr::from_ptr(p) }.to_str().expect("UTF-8")
}

/// Crea, anima `secs` segundos y devuelve (w, h, píxeles no negros, suma de control, dest, reduce).
fn run(px_w: i32, px_h: i32, ini: Option<&str>, art: Option<&[u8]>, secs: f64) -> (i32, i32, usize, u64, [i32; 4], i32) {
    let ini = ini.map(|s| CString::new(s).unwrap());
    let art = art.map(|s| CString::new(s).unwrap());
    unsafe {
        let e = ssfx_new(px_w, px_h, ini.as_ref().map_or(ptr::null(), |s| s.as_ptr()), art.as_ref().map_or(ptr::null(), |s| s.as_ptr()), 0x0078D4, 7, 1);
        assert!(!e.is_null());
        let (mut changed, mut t) = (0, 0.0);
        while t < secs {
            t += 1.0 / 60.0;
            changed += ssfx_tick(e, t, &NOW);
        }
        assert!(secs < 0.2 || changed > 0, "ssfx_tick nunca devolvió 1");
        let (mut w, mut h) = (0, 0);
        let px = ssfx_pixels(e, &mut w, &mut h);
        assert!(!px.is_null() && w > 0 && h > 0);
        let buf = std::slice::from_raw_parts(px, (w * h) as usize);
        let lit = buf.iter().filter(|&&p| p != 0).count();
        let sum = buf.iter().enumerate().fold(0u64, |a, (i, &p)| a.wrapping_mul(31).wrapping_add(p as u64 ^ i as u64));
        assert!(buf.iter().all(|&p| p >> 24 == 0));
        let mut dest = [-1i32; 4];
        let down = ssfx_dest(e, dest.as_mut_ptr());
        ssfx_free(e);
        (w, h, lit, sum, dest, down)
    }
}

#[test]
fn create_tick_destroy() {
    let (w, h, lit, _, dest, down) = run(1920, 1080, None, None, 3.0);
    assert_eq!((w, h), (960, 528));
    assert!(lit > 0);
    assert_eq!(dest, [0, 12, 1920, 1056]);
    assert_eq!(down, 0);
}

#[test]
fn small_preview_is_scaled_down() {
    let (w, h, _, _, dest, down) = run(320, 240, Some("effects=plasma"), None, 1.0);
    assert_eq!(down, 1);
    let [x, y, dw, dh] = dest;
    assert!(x >= 0 && y >= 0 && x + dw <= 320 && y + dh <= 240);
    assert!(dw <= w && dh <= h);
}

#[test]
fn ini_and_art_are_used() {
    let default = run(1280, 720, Some("effects=plasma"), None, 3.0);
    let themed = run(1280, 720, Some("effects=plasma\ntheme=ventura"), None, 3.0);
    assert_ne!(default.2, 0);
    assert_ne!(default, themed);
    let art = run(1280, 720, Some("effects=plasma\nbanner=file"), Some("ABC\nDEF".as_bytes()), 3.0);
    assert_ne!(default, art);
    // Arte en blanco: como si fuera NULL.
    let blank = run(1280, 720, Some("effects=plasma\nbanner=file"), Some(b"  \n\t \n"), 3.0);
    assert_eq!(default, blank);
    // Bytes que no son UTF-8: se sustituyen, sin pánico.
    let bad = run(1280, 720, Some("effects=plasma\nbanner=file"), Some(b"\xff\xfe AB \xc3\x28\n\x80CD"), 3.0);
    assert_ne!(bad.2, 0);
}

#[test]
fn null_arguments_are_safe() {
    unsafe {
        assert_eq!(ssfx_tick(ptr::null_mut(), 1.0, &NOW), 0);
        assert!(ssfx_pixels(ptr::null(), ptr::null_mut(), ptr::null_mut()).is_null());
        let mut dest = [7i32; 4];
        assert_eq!(ssfx_dest(ptr::null(), dest.as_mut_ptr()), 0);
        assert_eq!(dest, [7; 4], "sin motor no se escribe nada");
        ssfx_free(ptr::null_mut());

        // Motor válido con punteros de salida NULL y hora NULL.
        let e = ssfx_new(800, 600, ptr::null(), ptr::null(), 0, 0, 0);
        for i in 0..30 {
            ssfx_tick(e, i as f64 / 10.0, ptr::null());
        }
        assert!(!ssfx_pixels(e, ptr::null_mut(), ptr::null_mut()).is_null());
        let mut w = 0;
        assert!(!ssfx_pixels(e, &mut w, ptr::null_mut()).is_null());
        assert!(w > 0);
        ssfx_dest(e, ptr::null_mut());
        ssfx_free(e);
    }
}

#[test]
fn degenerate_sizes() {
    for (w, h) in [(0, 0), (1, 1), (-5, -5), (10, 10), (1, 1080)] {
        let (bw, bh, _, _, dest, down) = run(w, h, None, None, 1.5);
        assert!(bw > 0 && bh > 0);
        assert_eq!(down, 1, "{w}x{h}");
        assert!(dest.iter().all(|&v| v >= 0));
    }
}

#[test]
fn many_engines_at_once() {
    // Varios monitores: motores independientes con distinta semilla.
    unsafe {
        let es: Vec<_> = (0..4).map(|i| ssfx_new(1280, 720, ptr::null(), ptr::null(), 0xFF8A3D, i, 0)).collect();
        for k in 0..120 {
            for &e in &es {
                ssfx_tick(e, k as f64 / 30.0, &NOW);
            }
        }
        for e in es {
            ssfx_free(e);
        }
    }
}

#[test]
fn theme_catalog() {
    assert_eq!(ssfx_theme_count() as usize, THEMES.len());
    for i in 0..ssfx_theme_count() {
        assert_eq!(cstr(ssfx_theme_id(i)), THEMES[i as usize].id_str());
        assert_eq!(cstr(ssfx_theme_name(i)), THEMES[i as usize].name_str());
    }
    for i in [-1, ssfx_theme_count(), i32::MAX, i32::MIN] {
        assert!(ssfx_theme_id(i).is_null(), "{i}");
        assert!(ssfx_theme_name(i).is_null(), "{i}");
    }
}

#[test]
fn effect_catalog() {
    assert_eq!(ssfx_fx_count() as usize, EFFECTS.len());
    for i in 0..ssfx_fx_count() {
        assert_eq!(cstr(ssfx_fx_id(i)), EFFECTS[i as usize].id_str());
        let (es, en) = (cstr(ssfx_fx_name(i, 1)), cstr(ssfx_fx_name(i, 0)));
        assert!(!es.is_empty() && !en.is_empty());
        assert_eq!(cstr(ssfx_fx_name(i, 42)), es, "cualquier valor distinto de 0 es español");
    }
    assert_eq!(cstr(ssfx_fx_name(0, 1)), "Neón");
    assert_eq!(cstr(ssfx_fx_name(0, 0)), "Neon");
    for i in [-1, ssfx_fx_count(), i32::MAX, i32::MIN] {
        assert!(ssfx_fx_id(i).is_null(), "{i}");
        assert!(ssfx_fx_name(i, 0).is_null(), "{i}");
    }
}

#[test]
fn header_declares_every_exported_function() {
    let h = include_str!("../../include/ssfx.h");
    let src = include_str!("../src/ffi.rs");
    let exported: Vec<&str> = src.lines().filter_map(|l| l.split("extern \"C\" fn ").nth(1)).map(|r| r.split('(').next().unwrap()).collect();
    assert_eq!(exported.len(), 12, "{exported:?}");
    for name in exported {
        assert!(h.contains(&format!("{name}(")), "{name} no está en include/ssfx.h");
    }
    // WallTime: mismo número de campos i32 que SsfxWallTime; Screen: 4 double y 2 int32_t.
    assert_eq!(std::mem::size_of::<ssfx_core::desk::Screen>(), 4 * 8 + 2 * 4);
    assert_eq!(std::mem::size_of::<WallTime>(), 7 * 4);
}

#[test]
fn desk_engines_stay_in_sync() {
    use ssfx_core::desk::Screen;
    // Dos pantallas iguales lado a lado: los efectos locales salen idénticos en las dos.
    let s = [Screen { x: 0.0, y: 0.0, w: 1280.0, h: 720.0, px_w: 1280, px_h: 720 }, Screen { x: 1280.0, y: 0.0, w: 1280.0, h: 720.0, px_w: 1280, px_h: 720 }];
    let ini = CString::new("effects=plasma").unwrap();
    unsafe {
        let es: Vec<_> = (0..2).map(|i| ssfx_new_desk(s.as_ptr(), 2, i, ini.as_ptr(), ptr::null(), 0, 9, 1)).collect();
        for k in 0..120 {
            for &e in &es {
                ssfx_tick(e, k as f64 / 30.0, &NOW);
            }
        }
        let px = |e| {
            let (mut w, mut h) = (0, 0);
            let p = ssfx_pixels(e, &mut w, &mut h);
            std::slice::from_raw_parts(p, (w * h) as usize).to_vec()
        };
        assert_eq!(px(es[0]), px(es[1]));
        for e in es {
            ssfx_free(e);
        }
        // Lista NULL, n negativo o `me` fuera de rango: un motor de 1x1 que sigue funcionando.
        for (p, n, me) in [(ptr::null(), 2, 0), (s.as_ptr(), -1, 0), (s.as_ptr(), 2, 9), (s.as_ptr(), 2, -3)] {
            let e = ssfx_new_desk(p, n, me, ptr::null(), ptr::null(), 0, 1, 0);
            assert!(!e.is_null());
            for k in 0..20 {
                ssfx_tick(e, k as f64 / 10.0, ptr::null());
            }
            assert!(!ssfx_pixels(e, ptr::null_mut(), ptr::null_mut()).is_null());
            ssfx_free(e);
        }
    }
}
