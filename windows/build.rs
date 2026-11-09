//! Incrusta el manifiesto (estilos v6, DPI por monitor) y el nombre que muestra Windows en la
//! lista de protectores de pantalla (cadena 1). Usa windres de llvm-mingw.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=res/app.rc");
    println!("cargo:rerun-if-changed=res/app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let target = std::env::var("TARGET").unwrap();
    let arch = if target.starts_with("aarch64") { "aarch64" } else { "x86_64" };
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app.res.o");
    let windres = std::env::var("WINDRES").unwrap_or_else(|_| format!("{arch}-w64-mingw32-windres"));
    let ok = Command::new(&windres)
        .args(["--input", "res/app.rc", "--output-format=coff", "--output"])
        .arg(&out)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("cargo:rustc-link-arg-bins={}", out.display());
    } else {
        println!("cargo:warning=No se pudo ejecutar {windres}: el .scr no llevará manifiesto ni nombre.");
    }
}
