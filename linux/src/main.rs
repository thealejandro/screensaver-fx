//! Screensaver FX en una terminal. En Omarchy lo lanza el envoltorio de ttfx (linux/omarchy);
//! también funciona suelto en cualquier terminal con color de 24 bits.
//!
//! Uso:  screensaver-fx [efecto] [--seconds N] [--loop] [--no-restore-cursor]
//!       screensaver-fx --list | --palette | --banner [título] [subtítulo] | --version
//!       screensaver-fx --write-banner [título] [subtítulo]

#[cfg(unix)]
mod sys;
#[cfg(unix)]
mod term;

use ssfx_core::art::{banner_text_with, default_text_with};
use ssfx_core::color::{accent_palette, build_palette, Rgb, THEMES};
use ssfx_core::fx::{fx_index, EFFECTS};
use ssfx_core::{Banner, Config, Font, ThemeSel};
use std::path::PathBuf;

const FPS: f64 = 60.0;
/// Sin colores de tema ni acento, el núcleo usa los de Tokyo Night.
const NO_ACCENT: Rgb = Rgb(0, 0, 0);
/// Colores del tema de Omarchy que forman la paleta, por orden de preferencia.
const THEME_KEYS: [&str; 8] = ["accent", "bright_blue", "bright_cyan", "bright_magenta", "magenta", "cyan", "blue", "bright_green"];

const HELP: &str = "\
Screensaver FX: animaciones de rótulo en la terminal.

Uso:
  screensaver-fx [efecto]            un efecto (al azar si no se indica) y termina
  screensaver-fx --loop              efectos sin parar, hasta Ctrl+C
  screensaver-fx --list              efectos activos, uno por línea
  screensaver-fx --palette           colores en uso, en hexadecimal
  screensaver-fx --banner [T] [S]    imprime el rótulo de texto
  screensaver-fx --write-banner [T] [S]
                                     lo guarda como screensaver.txt de Omarchy, para que los
                                     efectos de ttfx usen el mismo rótulo

Opciones:
  --seconds N            corta a los N segundos
  --font F               letra de --banner: block, shadow, outline o pixel
  --no-restore-cursor    deja el cursor oculto al terminar (lo usa el salvapantallas)

Configuración: ~/.config/screensaver-fx/config.ini (banner, title, subtitle, font, theme,
effects, lang, calm=1 para el modo calma). Sin ella usa el rótulo y los colores del tema de Omarchy.
";

#[derive(Debug, Default, PartialEq)]
struct Args {
    effect: Option<String>,
    seconds: Option<f64>,
    looping: bool,
    keep_cursor_hidden: bool,
    font: Option<String>,
    /// --list, --palette, --banner, --help o --version.
    command: Option<String>,
    /// Título y subtítulo de --banner.
    words: Vec<String>,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = argv.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" => a.command = Some("help".to_string()),
            "--list" | "--palette" | "--banner" | "--write-banner" | "--help" | "--version" => a.command = Some(arg[2..].to_string()),
            "--loop" => a.looping = true,
            "--no-restore-cursor" => a.keep_cursor_hidden = true,
            "--seconds" => {
                let v = it.next().ok_or("--seconds necesita un número")?;
                a.seconds = Some(v.parse::<f64>().ok().filter(|s| s.is_finite() && *s >= 0.0).ok_or(format!("--seconds: «{v}» no es un número válido"))?);
            }
            "--font" => a.font = Some(it.next().ok_or("--font necesita un valor")?.clone()),
            s if s.starts_with("--") => return Err(format!("opción desconocida: {s}")),
            s if matches!(a.command.as_deref(), Some("banner" | "write-banner")) => a.words.push(s.to_string()),
            s if a.effect.is_none() => a.effect = Some(s.to_string()),
            s => return Err(format!("sobra el argumento «{s}»")),
        }
    }
    Ok(a)
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn home() -> PathBuf {
    PathBuf::from(env("HOME").unwrap_or_else(|| ".".into()))
}

fn config_home() -> PathBuf {
    env("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"))
}

fn read(path: PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok().filter(|s| !s.trim().is_empty())
}

/// Colores `clave = "#rrggbb"` del colors.toml de un tema de Omarchy.
fn theme_colors(toml: &str) -> Vec<(String, Rgb)> {
    toml.lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(k, v)| {
            let hex = v.trim().trim_matches('"').strip_prefix('#')?;
            (hex.len() == 6).then(|| u32::from_str_radix(hex, 16).ok()).flatten().map(|n| (k.trim().to_string(), Rgb::hex(n)))
        })
        .collect()
}

/// Paleta del tema activo de Omarchy, con los mismos colores y orden que usaba screensaver-fx.py.
fn omarchy_palette(toml: &str) -> Option<Vec<Rgb>> {
    let found = theme_colors(toml);
    let picked: Vec<Rgb> = THEME_KEYS.iter().filter_map(|k| found.iter().find(|f| f.0 == *k).map(|f| f.1)).collect();
    (!picked.is_empty()).then(|| build_palette(&picked))
}

/// ¿Español? Manda SCREENSAVER_FX_LANG, luego el idioma de Omarchy y por último el locale.
fn spanish(omarchy_language: Option<&str>) -> bool {
    let code = env("SCREENSAVER_FX_LANG")
        .or_else(|| omarchy_language.and_then(|s| s.split_whitespace().next().map(str::to_string)))
        .or_else(|| env("LC_ALL"))
        .or_else(|| env("LC_TIME"))
        .or_else(|| env("LANG"))
        .unwrap_or_default();
    code.to_ascii_lowercase().starts_with("es")
}

fn has_key(ini: &str, key: &str) -> bool {
    ini.lines().filter_map(|l| l.split_once('=')).any(|(k, _)| k.trim().trim_start_matches('\u{feff}').eq_ignore_ascii_case(key))
}

/// Configuración efectiva: config.ini si existe y, para lo que no diga, lo que ya usa Omarchy
/// (su screensaver.txt como rótulo y los colores del tema activo).
fn load_config(seed: u64) -> Config {
    let dir = config_home().join("screensaver-fx");
    let ini = env("SCREENSAVER_FX_CONFIG").map(PathBuf::from).or(Some(dir.join("config.ini"))).and_then(read).unwrap_or_default();
    let es = spanish(read(config_home().join("omarchy/language")).as_deref());
    let mut cfg = Config::parse(&ini, NO_ACCENT, seed, es);
    let omarchy_art = || env("SCREENSAVER_FX_ART").map(PathBuf::from).or(Some(config_home().join("omarchy/branding/screensaver.txt"))).and_then(read);
    if !has_key(&ini, "banner") {
        cfg.art = omarchy_art();
        if cfg.art.is_some() {
            cfg.banner = Banner::File;
        }
    } else if cfg.banner == Banner::File {
        cfg.art = read(dir.join("screensaver.txt")).or_else(omarchy_art);
    }
    let theme = env("SCREENSAVER_FX_THEME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/state/omarchy/current/theme/colors.toml"));
    cfg.system_palette = read(theme).as_deref().and_then(omarchy_palette);
    cfg
}

fn banner_mark() -> PathBuf {
    config_home().join("omarchy/branding/.screensaver-fx-banner")
}

/// Guarda `text` como screensaver.txt de Omarchy. La primera vez conserva el que hubiera en
/// screensaver.txt.bak. Deja una marca con el título, el subtítulo y la letra: así la
/// comprobación tras `omarchy update` sabe que el rótulo es nuestro y puede regenerarlo si
/// Omarchy lo pisa con su logo.
fn write_banner(text: &str, title: &str, subtitle: &str, font: Font) -> std::io::Result<()> {
    let mark = banner_mark();
    let dir = mark.parent().map(PathBuf::from).unwrap_or_default();
    let file = dir.join("screensaver.txt");
    std::fs::create_dir_all(&dir)?;
    if file.exists() && !mark.exists() {
        std::fs::copy(&file, dir.join("screensaver.txt.bak"))?;
    }
    std::fs::write(&file, text)?;
    let name = ssfx_core::config::FONTS.iter().find(|f| f.1 == font).map_or("block", |f| f.0);
    let one = |s: &str| s.replace(['\n', '\r'], " ");
    std::fs::write(&mark, format!("{}\n{}\n{name}\n", one(title), one(subtitle)))?;
    println!("{}", file.display());
    Ok(())
}

fn palette(cfg: &Config) -> Vec<Rgb> {
    match cfg.theme {
        ThemeSel::Named(i) => THEMES[i].palette(),
        _ => cfg.system_palette.clone().unwrap_or_else(|| accent_palette(cfg.accent)),
    }
}

fn seed() -> u64 {
    if let Some(s) = env("SCREENSAVER_FX_SEED") {
        // Para demos reproducibles: vale cualquier texto.
        return s.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01B3));
    }
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    nanos ^ (std::process::id() as u64) << 32
}

#[cfg(unix)]
fn run(mut cfg: Config, args: &Args) -> std::io::Result<()> {
    use std::time::{Duration, Instant};

    if let Some(name) = &args.effect {
        cfg.effects = vec![fx_index(name).expect("efecto ya validado")];
    }
    if !sys::die_with_parent() {
        return Ok(());
    }
    sys::catch_signals();
    let size = || sys::terminal_size().unwrap_or((120, 40));
    let (mut cols, mut rows) = size();
    let mut engine = ssfx_core::Engine::for_terminal(cols, rows, cfg.clone());
    let mut screen = term::Term::new(cols, rows);
    term::enter()?;
    let (mut start, mut started, mut frame) = (Instant::now(), false, 0u64);
    let result = loop {
        let t = start.elapsed().as_secs_f64();
        if sys::stop_requested() || args.seconds.is_some_and(|s| t >= s) {
            break Ok(());
        }
        engine.tick(t, sys::now());
        // Sin --loop se pinta un solo efecto: al acabar, el envoltorio decide el siguiente.
        match engine.current_effect() {
            Some(_) => started = true,
            None if started && !args.looping => break Ok(()),
            None => {}
        }
        if let Err(e) = screen.draw(engine.grid()) {
            break Err(e);
        }
        frame += 1;
        // Si cambia el tamaño de la ventana se empieza de nuevo con la rejilla correcta.
        if frame % 30 == 0 && size() != (cols, rows) {
            (cols, rows) = size();
            cfg.seed = cfg.seed.wrapping_add(frame);
            engine = ssfx_core::Engine::for_terminal(cols, rows, cfg.clone());
            screen = term::Term::new(cols, rows);
            term::enter()?;
            (start, started, frame) = (Instant::now(), false, 0);
            continue;
        }
        let next = Duration::from_secs_f64(frame as f64 / FPS);
        if let Some(wait) = next.checked_sub(start.elapsed()) {
            std::thread::sleep(wait);
        }
    };
    term::leave(!args.keep_cursor_hidden, rows);
    match result {
        // Terminal cerrada mientras se pintaba: no es un error.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

#[cfg(not(unix))]
fn run(_cfg: Config, _args: &Args) -> std::io::Result<()> {
    Err(std::io::Error::other("la versión de terminal solo existe para Linux y macOS; en Windows usa el .scr"))
}

fn fail(msg: &str) -> ! {
    eprintln!("screensaver-fx: {msg}");
    std::process::exit(2)
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args(&argv).unwrap_or_else(|e| fail(&e));
    let cfg = load_config(seed());
    match args.command.as_deref() {
        Some("help") => print!("{HELP}"),
        Some("version") => println!("screensaver-fx {}", env!("CARGO_PKG_VERSION")),
        Some("list") => cfg.effects.iter().for_each(|&i| println!("{}", EFFECTS[i].id_str())),
        Some("palette") => println!("{}", palette(&cfg).iter().map(|c| format!("{:06x}", c.u32())).collect::<Vec<_>>().join(" ")),
        Some(cmd @ ("banner" | "write-banner")) => {
            let font_named = |f: &str| ssfx_core::config::FONTS.iter().find(|x| x.0.eq_ignore_ascii_case(f)).map(|x| x.1);
            // Sin texto en la orden: el último rótulo guardado, el de config.ini o JACC + XACODE.
            let saved = read(banner_mark()).map(|m| m.lines().map(str::to_string).collect::<Vec<_>>()).filter(|m| m.len() == 3);
            let (title, subtitle, mut font) = if let Some(t) = args.words.first() {
                (t.clone(), args.words.get(1).cloned().unwrap_or_default(), cfg.font)
            } else if let Some(m) = saved {
                (m[0].clone(), m[1].clone(), font_named(&m[2]).unwrap_or(cfg.font))
            } else {
                (cfg.title.clone(), cfg.subtitle.clone(), cfg.font)
            };
            if let Some(f) = &args.font {
                font = font_named(f).unwrap_or_else(|| fail(&format!("letra desconocida: {f}")));
            }
            // Sin ningún texto: el rótulo por defecto, tal cual lo dibuja el protector.
            let text = if title.is_empty() && subtitle.is_empty() { default_text_with(font) } else { banner_text_with(&title, &subtitle, font) };
            if text.trim().is_empty() {
                fail("ese texto no tiene nada que se pueda dibujar");
            }
            if cmd == "banner" {
                print!("{text}");
            } else if let Err(e) = write_banner(&text, &title, &subtitle, font) {
                fail(&format!("no se pudo guardar el rótulo: {e}"));
            }
        }
        _ => {
            if let Some(name) = &args.effect {
                if fx_index(name).is_none() {
                    let all: Vec<&str> = EFFECTS.iter().map(|f| f.id_str()).collect();
                    fail(&format!("efecto desconocido: {name}. Disponibles: {}", all.join(", ")));
                }
            }
            if let Err(e) = run(cfg, &args) {
                fail(&e.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse_args(&list.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_effect_and_options() {
        let a = args(&["plasma", "--seconds", "0.5", "--no-restore-cursor"]).unwrap();
        assert_eq!((a.effect.as_deref(), a.seconds, a.keep_cursor_hidden, a.looping), (Some("plasma"), Some(0.5), true, false));
        assert!(args(&["--loop"]).unwrap().looping);
        assert_eq!(args(&[]).unwrap(), Args::default());
    }

    #[test]
    fn parses_commands() {
        for (flag, name) in [("--list", "list"), ("--palette", "palette"), ("--help", "help"), ("-h", "help"), ("--version", "version")] {
            assert_eq!(args(&[flag]).unwrap().command.as_deref(), Some(name), "{flag}");
        }
        let a = args(&["--banner", "Hola", "mundo", "--font", "shadow"]).unwrap();
        assert_eq!((a.command.as_deref(), a.words.as_slice(), a.font.as_deref(), a.effect), (Some("banner"), &["Hola".to_string(), "mundo".to_string()][..], Some("shadow"), None));
    }

    #[test]
    fn rejects_bad_arguments() {
        for bad in [&["--seconds"][..], &["--seconds", "x"], &["--seconds", "-1"], &["--seconds", "inf"], &["--nope"], &["a", "b"], &["--font"]] {
            assert!(args(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn reads_omarchy_theme_colors() {
        let toml = "# tema\nmode = \"dark\"\naccent = \"#3ddc97\"\nhyprland_active_border = \"rgba(3ddc97cc) 45deg\"\nbright_blue = \"#85c0ff\"\nbad = \"#12345\"\nblue=\"#5AA9FF\"\n";
        let found = theme_colors(toml);
        assert_eq!(found, [("accent".to_string(), Rgb::hex(0x3ddc97)), ("bright_blue".to_string(), Rgb::hex(0x85c0ff)), ("blue".to_string(), Rgb::hex(0x5aa9ff))]);
        let pal = omarchy_palette(toml).unwrap();
        assert_eq!(pal[0], Rgb::hex(0x3ddc97));
        assert!(pal.len() >= 2);
        // Sin ninguno de los colores esperados no hay paleta de tema: decide el núcleo.
        assert!(omarchy_palette("foreground = \"#ffffff\"").is_none());
        assert!(omarchy_palette("").is_none());
    }

    #[test]
    fn detects_ini_keys() {
        assert!(has_key("theme=aero\n Banner = text\n", "banner"));
        assert!(has_key("\u{feff}banner=file", "banner"));
        assert!(!has_key("title=banner\n# banner=text", "banner"));
    }
}
