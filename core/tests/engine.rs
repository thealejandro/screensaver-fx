//! Pruebas del motor completo a través de su API pública (lo que usan Windows y macOS).

use ssfx_core::art::Art;
use ssfx_core::color::{Rgb, THEMES};
use ssfx_core::fx::EFFECTS;
use ssfx_core::{Config, Engine, WallTime};

const NOW: WallTime = WallTime { year: 2026, month: 11, day: 9, weekday: 0, hour: 21, minute: 47, second: 10 };
const ACCENT: Rgb = Rgb(0x00, 0x78, 0xD4);
const FPS: f64 = 60.0;

fn cfg(ini: &str, seed: u64) -> Config {
    Config::parse(ini, ACCENT, seed, true)
}

fn engine(w: i32, h: i32, ini: &str) -> Engine {
    Engine::new(w, h, cfg(ini, 7))
}

fn nonblack(e: &Engine) -> usize {
    e.pixels().iter().filter(|&&p| p != 0).count()
}

/// Comprueba lo que la plataforma da por hecho en cada fotograma.
struct Checker {
    prev: Vec<u32>,
    /// Comparar también que fuera del rectángulo sucio no cambió nada (más lento).
    strict: bool,
}

impl Checker {
    fn new(e: &Engine, strict: bool) -> Checker {
        Checker { prev: if strict { e.pixels().to_vec() } else { Vec::new() }, strict }
    }

    fn tick(&mut self, e: &mut Engine, t: f64, now: WallTime, what: &str) -> bool {
        let changed = e.tick(t, now);
        let l = e.layout();
        let (bw, bh) = (l.buf_w as usize, l.buf_h as usize);
        let px = e.pixels();
        assert_eq!(px.len(), bw * bh, "{what} t={t}: tamaño del búfer");
        match e.dirty() {
            Some((x, y, w, h)) => {
                assert!(changed, "{what} t={t}: rectángulo sucio sin cambios");
                let (x, y, w, h) = (x as usize, y as usize, w as usize, h as usize);
                assert!(w > 0 && h > 0, "{what} t={t}: rectángulo vacío");
                assert!(x + w <= bw && y + h <= bh, "{what} t={t}: rectángulo {x},{y} {w}x{h} fuera de {bw}x{bh}");
                if self.strict {
                    for row in 0..bh {
                        let (a, b) = (&px[row * bw..][..bw], &self.prev[row * bw..][..bw]);
                        if (y..y + h).contains(&row) {
                            assert!(a[..x] == b[..x] && a[x + w..] == b[x + w..], "{what} t={t}: cambió fuera del rectángulo (fila {row})");
                            self.prev[row * bw + x..][..w].copy_from_slice(&a[x..x + w]);
                        } else {
                            assert!(a == b, "{what} t={t}: cambió fuera del rectángulo (fila {row})");
                        }
                    }
                }
            }
            None => {
                assert!(!changed, "{what} t={t}: cambió sin rectángulo sucio");
                if self.strict {
                    assert!(px == &self.prev[..], "{what} t={t}: devolvió false pero cambió");
                }
            }
        }
        changed
    }
}

fn assert_valid_pixels(e: &Engine, what: &str) {
    assert!(e.pixels().iter().all(|&p| p >> 24 == 0), "{what}: píxel con byte alto (no es 0x00RRGGBB)");
}

/// Ejecuta `frames` fotogramas a `FPS` desde `t0`.
fn run(e: &mut Engine, ck: &mut Checker, t0: f64, frames: usize, what: &str) -> f64 {
    let mut t = t0;
    for _ in 0..frames {
        t += 1.0 / FPS;
        ck.tick(e, t, NOW, what);
    }
    t
}

const SIZES: [(i32, i32); 12] = [
    (320, 240),
    (1280, 720),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
    (1080, 1920),
    (152, 112),
    (10, 10),
    (1, 1),
    (0, 0),
    (1, 1080),
    (3440, 1440),
];

#[test]
fn every_effect_at_every_size_runs_600_frames() {
    for def in EFFECTS.iter() {
        let id = def.id_str();
        for &(w, h) in &SIZES {
            let what = format!("{id} {w}x{h}");
            let mut e = engine(w, h, &format!("effects={id}"));
            let l = e.layout();
            assert_eq!(e.pixels().len(), (l.buf_w * l.buf_h) as usize, "{what}");
            assert_eq!(nonblack(&e), 0, "{what}: empieza en negro");
            let mut ck = Checker::new(&e, false);
            run(&mut e, &mut ck, 0.0, 600, &what);
            assert!(nonblack(&e) > 0, "{what}: todo negro tras 10 s");
            assert_valid_pixels(&e, &what);
        }
    }
}

#[test]
fn every_effect_full_cycle_with_strict_dirty_rect() {
    for def in EFFECTS.iter() {
        let id = def.id_str();
        for (w, h) in [(1920, 1080), (320, 240)] {
            let what = format!("{id} {w}x{h}");
            let mut e = engine(w, h, &format!("effects={id}"));
            let mut ck = Checker::new(&e, true);
            assert_eq!(e.current_effect(), None);
            // Hueco inicial (0.15 s).
            ck.tick(&mut e, 0.1, NOW, &what);
            assert_eq!(e.current_effect(), None, "{what}: antes del primer efecto");
            assert_eq!(nonblack(&e), 0, "{what}");
            ck.tick(&mut e, 0.15, NOW, &what);
            assert_eq!(e.current_effect(), Some(id), "{what}");
            // Efecto completo + hueco + comienzo del siguiente (el mismo, es el único).
            let frames = ((def.secs as f64 + 0.6 + 2.0) * FPS) as usize;
            let mut t = 0.15;
            let (mut saw_gap, mut restarted) = (false, false);
            for _ in 0..frames {
                t += 1.0 / FPS;
                ck.tick(&mut e, t, NOW, &what);
                match e.current_effect() {
                    None => {
                        saw_gap = true;
                        assert_eq!(nonblack(&e), 0, "{what} t={t}: el hueco entre efectos es negro");
                    }
                    Some(cur) => {
                        assert_eq!(cur, id);
                        restarted |= saw_gap;
                    }
                }
            }
            assert!(saw_gap && restarted, "{what}: no pasó por el hueco y vuelta a empezar");
            assert_valid_pixels(&e, &what);
        }
    }
}

#[test]
fn banner_is_fully_visible_near_the_end_of_each_effect() {
    let art_cells = Art::default_art().cells.len();
    for def in EFFECTS.iter() {
        let id = def.id_str();
        let mut e = engine(1920, 1080, &format!("effects={id}"));
        let mut ck = Checker::new(&e, false);
        let frames = ((0.15 + def.secs as f64 - 0.4) * FPS) as usize;
        run(&mut e, &mut ck, 0.0, frames, id);
        assert_eq!(e.current_effect(), Some(id));
        // Cada celda del rótulo (casi todas bloques) pinta varios píxeles de 8x16.
        assert!(nonblack(&e) >= art_cells * 32, "{id}: {} píxeles para {art_cells} celdas", nonblack(&e));
    }
}

#[test]
fn something_is_drawn_soon_after_start() {
    for def in EFFECTS.iter() {
        let id = def.id_str();
        let mut e = engine(1280, 720, &format!("effects={id}"));
        let mut ck = Checker::new(&e, false);
        let mut first = None;
        let mut t = 0.0;
        for _ in 0..(3.0 * FPS) as usize {
            t = run(&mut e, &mut ck, t, 1, id);
            if first.is_none() && nonblack(&e) > 0 {
                first = Some(t);
            }
        }
        let first = first.unwrap_or_else(|| panic!("{id}: nada en 3 s"));
        assert!(first < 2.5, "{id}: primer píxel a los {first} s");
    }
}

#[test]
fn same_seed_is_deterministic() {
    for ini in ["effects=all", "effects=all\ntheme=random", "effects=starfield,matrix,burn,scatter"] {
        let (mut a, mut b) = (Engine::new(1280, 720, cfg(ini, 99)), Engine::new(1280, 720, cfg(ini, 99)));
        let mut t = 0.0;
        // 45 s a 30 fps: varios efectos seguidos.
        for frame in 0..1350 {
            t += 1.0 / 30.0;
            assert_eq!(a.tick(t, NOW), b.tick(t, NOW), "{ini} fotograma {frame}");
            assert_eq!(a.current_effect(), b.current_effect(), "{ini} fotograma {frame}");
            assert_eq!(a.dirty(), b.dirty(), "{ini} fotograma {frame}");
            assert!(a.pixels() == b.pixels(), "{ini}: fotograma {frame} distinto con la misma semilla");
        }
    }
}

#[test]
fn different_seeds_give_different_shows() {
    let frames = |seed| {
        let mut e = Engine::new(1280, 720, cfg("effects=all", seed));
        let mut order = Vec::new();
        let mut px = Vec::new();
        let mut t = 0.0;
        for i in 0..1800 {
            t += 1.0 / 30.0;
            e.tick(t, NOW);
            if let Some(id) = e.current_effect() {
                if order.last() != Some(&id) {
                    order.push(id);
                }
            }
            if i % 90 == 0 {
                px.push(e.pixels().to_vec());
            }
        }
        (order, px)
    };
    let (a, b) = (frames(1), frames(2));
    assert!(a != b, "dos semillas dieron exactamente lo mismo");
}

#[test]
fn effects_rotate_without_immediate_repeats() {
    for ini in ["effects=all", "effects=neon,plasma", "effects=classic", "effects=modern"] {
        let c = cfg(ini, 3);
        let allowed: Vec<&str> = c.effects.iter().map(|&i| EFFECTS[i].id_str()).collect();
        let mut e = Engine::new(800, 600, c);
        let mut order: Vec<&str> = Vec::new();
        let mut was_gap = true;
        let mut t = 0.0;
        // 60 minutos a 10 fps (dt grande, como un equipo lento): con tantos efectos, en 30 alguno podía no salir.
        for _ in 0..36_000 {
            t += 0.1;
            e.tick(t, NOW);
            match e.current_effect() {
                Some(id) if was_gap => {
                    order.push(id);
                    was_gap = false;
                }
                Some(_) => {}
                None => was_gap = true,
            }
        }
        assert!(order.len() >= 90, "{ini}: solo {} efectos en 60 min", order.len());
        assert!(order.iter().all(|id| allowed.contains(id)), "{ini}: efecto no seleccionado");
        assert!(order.windows(2).all(|w| w[0] != w[1]), "{ini}: efecto repetido seguido: {order:?}");
        let mut distinct = order.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), allowed.len(), "{ini}: no salieron todos: {distinct:?}");
    }
}

#[test]
fn single_effect_repeats_forever() {
    let mut e = engine(800, 600, "effects=wipe");
    let mut starts = 0;
    let mut was_gap = true;
    let mut t = 0.0;
    for _ in 0..1000 {
        t += 0.1;
        e.tick(t, NOW);
        let running = e.current_effect().is_some();
        if running && was_gap {
            starts += 1;
            assert_eq!(e.current_effect(), Some("wipe"));
        }
        was_gap = !running;
    }
    assert!(starts >= 9, "{starts}");
}

#[test]
fn duplicated_effect_in_list_does_not_panic() {
    // `effects=neon,neon`: al acabar el primero, filtrar el último efecto dejaba la lista vacía.
    for ini in ["effects=neon,neon", "effects=plasma, PLASMA ,plasma", "effects=wipe,wipe,nope"] {
        let mut e = engine(640, 480, ini);
        let mut t = 0.0;
        let mut starts = 0;
        let mut was_gap = true;
        for _ in 0..600 {
            t += 0.1;
            e.tick(t, NOW);
            let running = e.current_effect().is_some();
            starts += (running && was_gap) as i32;
            was_gap = !running;
        }
        assert!(starts >= 3, "{ini}: {starts}");
    }
}

/// Píxeles de plasma ya asentado (3 s), que pinta todo el rótulo con la paleta.
fn plasma_frame(ini: &str, accent: Rgb) -> Vec<u32> {
    let mut e = Engine::new(1280, 720, Config::parse(&format!("effects=plasma\n{ini}"), accent, 5, true));
    let mut t = 0.0;
    while t < 3.0 {
        t += 1.0 / FPS;
        e.tick(t, NOW);
    }
    assert!(nonblack(&e) > 0, "{ini}");
    assert_valid_pixels(&e, ini);
    e.pixels().to_vec()
}

#[test]
fn every_theme_renders_its_own_colors() {
    let mut frames: Vec<Vec<u32>> = Vec::new();
    for t in THEMES.iter() {
        let px = plasma_frame(&format!("theme={}", t.id_str()), ACCENT);
        assert!(!frames.contains(&px), "tema {} igual que otro", t.id_str());
        // Los colores pintados son mezclas de la paleta: ninguno muy oscuro con plasma a pleno.
        let pal = t.palette();
        let max_c = |p: u32| ((p >> 16) & 0xFF).max((p >> 8) & 0xFF).max(p & 0xFF);
        let pal_min = pal.iter().map(|c| c.0.max(c.1).max(c.2) as u32).min().unwrap();
        let lit: Vec<u32> = px.iter().copied().filter(|&p| p != 0).collect();
        assert!(lit.iter().all(|&p| max_c(p) + 2 >= pal_min / 2), "tema {}: color demasiado oscuro", t.id_str());
        frames.push(px);
    }
}

#[test]
fn auto_theme_follows_the_system_accent() {
    let (red, green) = (Rgb(0xE8, 0x11, 0x23), Rgb(0x10, 0x89, 0x3E));
    // Con auto, cambiar el acento cambia los colores.
    assert!(plasma_frame("theme=auto", red) != plasma_frame("theme=auto", green));
    assert!(plasma_frame("", red) != plasma_frame("", green), "auto es el valor por defecto");
    // Con un tema fijo, el acento no influye.
    assert!(plasma_frame("theme=aero", red) == plasma_frame("theme=aero", green));
    // Un tema inválido equivale a auto.
    assert!(plasma_frame("theme=nope", red) == plasma_frame("theme=auto", red));
    // Acento rojo: predominan los rojos en el rótulo.
    let px = plasma_frame("theme=auto", Rgb(0xFF, 0x30, 0x30));
    let reddish = px.iter().filter(|&&p| p != 0 && (p >> 16) & 0xFF > p & 0xFF).count();
    assert!(reddish > 0);
}

#[test]
fn random_theme_changes_between_effects() {
    let mut e = engine(1280, 720, "effects=plasma\ntheme=random");
    let mut seen: Vec<Vec<u32>> = Vec::new();
    let mut t = 0.0;
    let mut local = 0.0;
    for _ in 0..(8.0 * 16.0 * 10.0) as usize {
        t += 0.1;
        e.tick(t, NOW);
        if e.current_effect().is_some() {
            local += 0.1;
            if (local - 5.0f64).abs() < 0.05 {
                let px = e.pixels().to_vec();
                if !seen.contains(&px) {
                    seen.push(px);
                }
            }
        } else {
            local = 0.0;
        }
    }
    assert!(seen.len() >= 2, "theme=random no cambió de paleta en 8 ciclos");
}

#[test]
fn sizes_change_the_scale() {
    let k = |s: &str| engine(3840, 2160, &format!("size={s}")).layout().int_scale();
    assert_eq!((k("s"), k("m"), k("l")), (Some(3), Some(4), Some(5)));
}

fn run_frames(e: &mut Engine, secs: f64) {
    let mut t = 0.0;
    while t < secs {
        t += 1.0 / 30.0;
        e.tick(t, NOW);
    }
}

fn frame_with(ini: &str, art: Option<&str>) -> (Vec<u32>, ssfx_core::Layout) {
    let mut c = cfg(&format!("effects=plasma\n{ini}"), 1);
    c.art = art.map(str::to_string);
    let mut e = Engine::new(1920, 1080, c);
    run_frames(&mut e, 3.0);
    (e.pixels().to_vec(), e.layout())
}

#[test]
fn text_banner_one_and_two_lines() {
    let default = frame_with("", None);
    let one = frame_with("banner=text\ntitle=Hola", None);
    let two = frame_with("banner=text\ntitle=Hola\nsubtitle=Mundo", None);
    let sub_only = frame_with("banner=text\nsubtitle=Hola", None);
    assert!(one != default && two != default && one != two);
    assert!(one == sub_only, "una sola línea en el subtítulo se dibuja como título");
    assert!(nonblack_px(&two.0) > nonblack_px(&one.0));
    // Sin banner=text, title se ignora.
    assert!(frame_with("title=Hola", None) == default);
    // Texto sin glifos dibujables o vacío: rótulo por defecto.
    for ini in ["banner=text", "banner=text\ntitle=   ", "banner=text\ntitle=😀😀", "banner=text\ntitle=日本"] {
        assert!(frame_with(ini, None) == default, "{ini}");
    }
    // Título larguísimo: se recorta y no rompe nada.
    let long = frame_with(&format!("banner=text\ntitle={}\nsubtitle={}", "W".repeat(300), "M".repeat(300)), None);
    let cut = frame_with(&format!("banner=text\ntitle={}\nsubtitle={}", "W".repeat(16), "M".repeat(32)), None);
    assert!(long == cut);
    // Unicode latino.
    assert!(frame_with("banner=text\ntitle=Ñandú\nsubtitle=açaí é ótimo", None) != default);
}

fn nonblack_px(px: &[u32]) -> usize {
    px.iter().filter(|&&p| p != 0).count()
}

#[test]
fn file_banner_variants() {
    let default = frame_with("", None);
    // banner=file sin arte, arte vacío o en blanco: rótulo por defecto.
    for art in [None, Some(""), Some("   \n\t\n"), Some("\u{feff}\r\n")] {
        assert!(frame_with("banner=file", art) == default, "{art:?}");
    }
    // Arte propio con varias líneas, tabs, CRLF, BOM y caracteres no ASCII.
    let art = "\u{feff}  ╔══════╗\r\n\t║ ñ é ü ║\r\n  ╚══════╝\r\n😀 日本 ░▒▓█\r\n";
    let own = frame_with("banner=file", Some(art));
    assert!(own != default && nonblack_px(&own.0) > 0);
    // El arte solo se usa con banner=file.
    assert!(frame_with("", Some(art)) == default);
    // Línea demasiado larga o demasiadas líneas: se descarta y vuelve el rótulo por defecto.
    assert!(frame_with("banner=file", Some(&"#".repeat(5000))) == default);
    assert!(frame_with("banner=file", Some(&vec!["#"; 500].join("\n"))) == default);
}

#[test]
fn max_size_art_on_small_and_large_screens() {
    // 400x200, el máximo que acepta Art::parse.
    let big = vec!["▓█".repeat(200); 200].join("\n");
    for (w, h) in [(320, 240), (1920, 1080), (3840, 2160), (1, 1)] {
        let mut c = cfg("banner=file", 2);
        c.art = Some(big.clone());
        let mut e = Engine::new(w, h, c);
        let l = e.layout();
        assert!(l.cols >= 408 && l.rows >= 210, "{w}x{h}: {l:?}");
        let mut ck = Checker::new(&e, false);
        let mut t = 0.0;
        for _ in 0..8 {
            t += 0.5;
            ck.tick(&mut e, t, NOW, "400x200");
        }
        assert!(nonblack(&e) > 0, "{w}x{h}");
    }
}

#[test]
fn large_art_with_every_effect() {
    let big = vec!["▓█░ ".repeat(30); 60].join("\n");
    for (w, h) in [(320, 240), (1920, 1080), (1, 1)] {
        for def in EFFECTS.iter() {
            let id = def.id_str();
            let mut c = cfg(&format!("banner=file\neffects={id}"), 2);
            c.art = Some(big.clone());
            let mut e = Engine::new(w, h, c);
            let l = e.layout();
            assert!(l.cols >= 120 + 8 && l.rows >= 60 + 10, "{id} {w}x{h}: {l:?}");
            let mut ck = Checker::new(&e, false);
            // Pasos de 0,25 s hasta pasar el final del efecto.
            let mut t = 0.0;
            let mut drew = false;
            while t < def.secs as f64 + 1.0 {
                t += 0.25;
                ck.tick(&mut e, t, NOW, id);
                drew |= nonblack(&e) > 0;
            }
            assert!(drew, "{id} {w}x{h}");
        }
    }
}

#[test]
fn odd_tick_times_do_not_panic() {
    for def in EFFECTS.iter() {
        let id = def.id_str();
        let mut e = engine(1280, 720, &format!("effects={id}"));
        let mut ck = Checker::new(&e, false);
        let times = [0.0, 0.0, -1.0, 0.2, 0.2, 0.1, 5.0, 4.0, 3.0, 1e-12, 30.0, 30.0001, 1e7, 1e7 + 0.016, 1e7 + 100.0, 1e12, -1e12, 3.0, 3.016];
        for &t in &times {
            ck.tick(&mut e, t, NOW, id);
        }
        for &t in &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1.0, 2.0, 1e30] {
            ck.tick(&mut e, t, NOW, id);
        }
        // Tras los valores absurdos sigue funcionando con tiempos normales.
        let mut t = 2e30;
        for _ in 0..200 {
            t += 1.0 / 60.0;
            ck.tick(&mut e, t, NOW, id);
        }
    }
}

#[test]
fn recovers_after_non_finite_time() {
    // Un NaN o infinito suelto (reloj de la plataforma roto) no debe congelar la animación.
    for bad in [f64::NAN, f64::INFINITY] {
        let mut e = engine(1280, 720, "effects=all");
        run_frames(&mut e, 1.0);
        e.tick(bad, NOW);
        let mut t = 1.0;
        let mut starts = 0;
        let mut was_gap = e.current_effect().is_none();
        for _ in 0..(60.0 * 30.0) as usize {
            t += 1.0 / 30.0;
            e.tick(t, NOW);
            let running = e.current_effect().is_some();
            starts += (running && was_gap) as i32;
            was_gap = !running;
        }
        assert!(starts >= 2, "{bad}: la animación se quedó parada ({starts} efectos en 60 s)");
    }
}

#[test]
fn repeated_time_reports_no_change_for_time_driven_effects() {
    // Efectos que solo dependen del tiempo: el mismo t dos veces no cambia nada.
    for id in ["neon", "terminal", "plasma", "clock", "decrypt", "rain", "beams", "slide", "scatter", "waves", "spotlights", "wipe", "glitch", "burn", "matrix"] {
        let mut e = engine(1280, 720, &format!("effects={id}"));
        let mut ck = Checker::new(&e, true);
        let mut t = 0.0;
        for i in 0..240 {
            t += 1.0 / 60.0;
            ck.tick(&mut e, t, NOW, id);
            if i % 40 == 39 {
                assert!(!ck.tick(&mut e, t, NOW, id), "{id} t={t}: mismo instante, cambio inesperado");
                assert_eq!(e.dirty(), None);
            }
        }
    }
}

#[test]
fn clock_follows_wall_time_and_language() {
    let frame = |ini: &str, now: WallTime| {
        let mut e = engine(1920, 1080, &format!("effects=clock\n{ini}"));
        let mut t = 0.0;
        while t < 3.0 {
            t += 1.0 / 30.0;
            e.tick(t, now);
        }
        e.pixels().to_vec()
    };
    let base = frame("", NOW);
    assert!(base != frame("", WallTime { minute: 48, ..NOW }), "el minuto no cambia el reloj");
    assert!(base != frame("", WallTime { day: 10, weekday: 1, ..NOW }), "la fecha no cambia");
    assert!(base != frame("lang=en", NOW), "el idioma no cambia la fecha");
    // Los dos puntos parpadean con los segundos.
    assert!(frame("", WallTime { second: 10, ..NOW }) != frame("", WallTime { second: 11, ..NOW }));
    // Hora basura (plataforma sin reloj): sin pánico.
    for now in [WallTime::default(), WallTime { year: -5, month: 0, day: -1, weekday: -8, hour: 99, minute: -7, second: -1 }] {
        frame("", now);
    }
}

#[test]
fn engine_from_parsed_garbage_runs() {
    let ini = "\u{feff}[x]\nTHEME = RANDOM\nsize=xxl\neffects=all,neon,,\nbanner=TEXT\ntitle=\t¡Hola, señor!\nsubtitle===\nlang=??\n";
    let mut e = Engine::new(1366, 768, cfg(ini, u64::MAX));
    let mut ck = Checker::new(&e, false);
    run(&mut e, &mut ck, 0.0, 1200, "garbage");
    assert!(nonblack(&e) > 0 || e.current_effect().is_none());
}

// ── Varios monitores ────────────────────────────────────────────────────────

use ssfx_core::desk::Screen;

fn three_screens() -> Vec<Screen> {
    vec![
        Screen { x: -2560.0, y: -180.0, w: 2560.0, h: 1440.0, px_w: 2560, px_h: 1440 },
        Screen { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, px_w: 1920, px_h: 1080 },
        Screen { x: 1920.0, y: -400.0, w: 1080.0, h: 1920.0, px_w: 1080, px_h: 1920 },
    ]
}

#[test]
fn monitors_stay_in_sync_with_different_frame_times() {
    // Cada monitor dibuja a su ritmo (60, 144 y 30 Hz con tirones): mismo efecto y tema a la vez.
    for ini in ["effects=all", "effects=all\ntheme=random", "effects=classic", "effects=relay,comet,tide"] {
        let screens = three_screens();
        let mut es: Vec<Engine> = (0..3).map(|me| Engine::with_desk(&screens, me, cfg(ini, 99))).collect();
        let steps = [1.0 / 60.0, 1.0 / 144.0, 1.0 / 30.0];
        let mut ts = [0.0f64; 3];
        let mut checked = 0;
        for probe in 1..=400 {
            let target = probe as f64 * 0.75 + 0.013;
            for k in 0..3 {
                while ts[k] < target {
                    // Tirón de 70 ms de vez en cuando.
                    ts[k] += if (ts[k] * 10.0) as i64 % 37 == 0 { 0.07 } else { steps[k] };
                    es[k].tick(ts[k].min(target), NOW);
                }
                ts[k] = target;
                es[k].tick(target, NOW);
            }
            let fx: Vec<_> = es.iter().map(|e| e.current_effect()).collect();
            assert!(fx.iter().all(|f| *f == fx[0]), "{ini} t={target}: {fx:?}");
            checked += fx[0].is_some() as i32;
        }
        assert!(checked > 300, "{ini}: casi nunca había efecto ({checked})");
    }
}

#[test]
fn a_monitor_that_joins_late_catches_up() {
    // macOS puede crear la vista de una pantalla más tarde (o recrearla al cambiar de tamaño).
    let screens = three_screens();
    let mut a = Engine::with_desk(&screens, 0, cfg("effects=all\ntheme=random", 3));
    let mut t = 0.0;
    while t < 600.0 {
        t += 1.0 / 30.0;
        a.tick(t, NOW);
    }
    let mut b = Engine::with_desk(&screens, 2, cfg("effects=all\ntheme=random", 3));
    for _ in 0..600 {
        t += 1.0 / 60.0;
        a.tick(t, NOW);
        b.tick(t, NOW);
        assert_eq!(a.current_effect(), b.current_effect(), "t={t}");
    }
}

#[test]
fn same_size_monitors_mirror_local_effects() {
    // Dos monitores iguales: los efectos de un monitor se ven idénticos en los dos a la vez.
    let screens = [Screen { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, px_w: 1920, px_h: 1080 }, Screen { x: 1920.0, y: 0.0, w: 1920.0, h: 1080.0, px_w: 1920, px_h: 1080 }];
    let mut a = Engine::with_desk(&screens, 0, cfg("effects=classic", 8));
    let mut b = Engine::with_desk(&screens, 1, cfg("effects=classic", 8));
    let mut t = 0.0;
    while t < 60.0 {
        t += 1.0 / 60.0;
        a.tick(t, NOW);
        b.tick(t, NOW);
        assert!(a.pixels() == b.pixels(), "t={t} {:?}", a.current_effect());
    }
}

#[test]
fn desk_with_bad_screens_still_runs() {
    let bad = [Screen { x: f64::NAN, y: 0.0, w: 10.0, h: 10.0, px_w: 10, px_h: 10 }, Screen { x: 0.0, y: 0.0, w: -1.0, h: 0.0, px_w: 0, px_h: 0 }];
    for me in [0, 1, 7] {
        for screens in [&bad[..], &[]] {
            let mut e = Engine::with_desk(screens, me, cfg("effects=all", 1));
            let mut ck = Checker::new(&e, false);
            run(&mut e, &mut ck, 0.0, 600, "pantallas no válidas");
        }
    }
}

#[test]
fn screens_main_leaves_the_others_black() {
    let screens = three_screens();
    let mut main = Engine::with_desk(&screens, 0, cfg("effects=all\nscreens=main", 4));
    let mut others: Vec<Engine> = (1..3).map(|me| Engine::with_desk(&screens, me, cfg("effects=all\nscreens=main", 4))).collect();
    let (mut t, mut drawn) = (0.0, 0);
    while t < 60.0 {
        t += 1.0 / 30.0;
        drawn += main.tick(t, NOW) as i32;
        for e in &mut others {
            assert!(!e.tick(t, NOW), "t={t}: una pantalla secundaria dibujó");
            assert!(e.pixels().iter().all(|&p| p == 0) && e.current_effect().is_none());
        }
    }
    assert!(drawn > 100, "la principal casi no dibujó ({drawn})");
}

#[test]
fn screens_each_runs_its_own_sequence_on_its_own_screen() {
    let screens = three_screens();
    let each: Vec<Engine> = (0..3).map(|me| Engine::with_desk(&screens, me, cfg("effects=all\nscreens=each", 4))).collect();
    let alone: Vec<Engine> = (0..3).map(|me| Engine::with_desk(&screens[me..=me], 0, cfg("effects=all", 4))).collect();
    // Cada motor usa solo su pantalla: la misma rejilla que si estuviera sola.
    for (a, b) in each.iter().zip(&alone) {
        assert_eq!(a.layout(), b.layout());
    }
    let mut es = each;
    let mut seqs = vec![Vec::new(); 3];
    let mut t = 0.0;
    while t < 400.0 {
        t += 1.0 / 20.0;
        for (e, s) in es.iter_mut().zip(&mut seqs) {
            e.tick(t, NOW);
            if let Some(f) = e.current_effect() {
                if s.last() != Some(&f) {
                    s.push(f);
                }
            }
        }
    }
    assert!(seqs[0] != seqs[1] && seqs[1] != seqs[2] && seqs[0] != seqs[2], "{seqs:?}");
}

#[test]
fn screens_option_with_a_single_screen_changes_nothing() {
    let one = [Screen::px(1920, 1080)];
    for mode in ["each", "main"] {
        let mut a = Engine::with_desk(&one, 0, cfg(&format!("effects=all\nscreens={mode}"), 6));
        let mut b = Engine::with_desk(&one, 0, cfg("effects=all", 6));
        let mut t = 0.0;
        while t < 40.0 {
            t += 1.0 / 30.0;
            assert_eq!(a.tick(t, NOW), b.tick(t, NOW));
            assert!(a.pixels() == b.pixels(), "{mode} t={t}");
        }
    }
}
