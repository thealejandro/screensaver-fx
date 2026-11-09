//! Screensaver FX para Windows: un .scr nativo (Win32 + GDI) sobre ssfx-core.
//!
//! Argumentos estándar de protector de pantalla:
//!   /s           pantalla completa en todos los monitores
//!   /p <hwnd>    vista previa dentro de la ventana de Configuración
//!   /c[:hwnd]    configuración (también sin argumentos)
//!   /watch       vigilante en segundo plano (ver watch.rs); /unwatch lo detiene
//!   /update      busca e instala una versión nueva (ver update.rs)

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod saver;
#[cfg(windows)]
mod settings;
#[cfg(windows)]
mod sys;
#[cfg(windows)]
mod watch;
#[cfg(windows)]
mod update;

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    let args: Vec<String> = std::env::args().skip(1).collect();
    let first = args.first().map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let (flag, inline) = match first.split_once(':') {
        Some((f, v)) => (f.to_string(), Some(v.to_string())),
        None => (first.clone(), None),
    };
    let hwnd_arg = || -> Option<HWND> {
        inline.clone().or_else(|| args.get(1).cloned()).and_then(|s| s.trim().parse::<usize>().ok()).filter(|&v| v != 0).map(|v| v as HWND)
    };
    let mode = flag.trim_start_matches(['/', '-']).to_string();
    if !matches!(mode.as_str(), "watch" | "unwatch" | "update") {
        update::hold_busy();
    }
    match mode.as_str() {
        "s" => saver::run(None, false),
        "w" => saver::run(None, true),
        "p" | "l" => {
            if let Some(h) = hwnd_arg() {
                saver::run(Some(h), false)
            }
        }
        "watch" => watch::run(),
        "unwatch" => watch::stop(),
        "update" => update::run_cli(),
        "c" => settings::run(hwnd_arg()),
        _ => settings::run(None),
    }
}
