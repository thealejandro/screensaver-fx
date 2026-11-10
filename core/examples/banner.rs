//! Imprime el rótulo de texto en la terminal para revisar la letra.
//! cargo run -p ssfx-core --example banner -- "Título" "subtítulo" [block|shadow|outline|pixel]

use ssfx_core::art::banner_text_with;
use ssfx_core::config::FONTS;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i: usize, d: &str| args.get(i).cloned().unwrap_or_else(|| d.to_string());
    let font = FONTS.iter().find(|f| f.0 == arg(3, "block")).map_or(FONTS[0].1, |f| f.1);
    print!("{}", banner_text_with(&arg(1, "JACC"), &arg(2, "XACODE"), font));
}
