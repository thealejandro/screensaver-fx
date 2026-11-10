#!/bin/bash
# Instala Screensaver FX en Omarchy: el binario screensaver-fx y los envoltorios que lo
# enganchan al salvapantallas. No modifica nada de Omarchy: todo va a ~/.local/bin y a
# ~/.config/omarchy/hooks, así que `omarchy update` no lo pisa.
# Uso (desde una copia del repo):  ./linux/install.sh
# Para actualizar: `git pull` y volver a ejecutarlo.
set -euo pipefail

HERE=$(dirname "$(realpath "$0")")
REPO=$(dirname "$HERE")
BIN=~/.local/bin
HOOKS=~/.config/omarchy/hooks/post-update.d
BRANDING=~/.config/omarchy/branding
RELEASES=https://github.com/thealejandro/screensaver-fx/releases/latest/download

# Mensaje en el idioma de Omarchy: msg "texto en español" "text in English"
msg() {
  local l=
  read -r l _ 2>/dev/null <"${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/language" || true
  l=${l:-${LC_ALL:-${LC_TIME:-${LANG:-}}}}
  l=${l,,}
  if [[ ${l:0:2} == es ]]; then printf '%s' "$1"; else printf '%s' "$2"; fi
}

for cmd in ttfx omarchy; do
  command -v "$cmd" >/dev/null || { echo "$(msg "Falta '$cmd': esto necesita Omarchy con ttfx." "Missing '$cmd': this needs Omarchy with ttfx.")" >&2; exit 1; }
done

put() { # copia $1 a $2 como ejecutable; guarda antes lo que hubiera si era un archivo ajeno
  local src=$1 dst=$2
  mkdir -p "$(dirname "$dst")"
  if [[ -L $dst ]]; then
    rm "$dst" # enlace de una instalación anterior: escribir a través de él tocaría su destino
  elif [[ -e $dst ]] && ! grep -qs 'screensaver-fx' "$dst"; then
    mv "$dst" "$dst.bak-$(date +%Y%m%d-%H%M%S)"
    echo "  $(msg "copia de seguridad" "backup"): $dst.bak-*"
  fi
  install -m 755 "$src" "$dst"
  echo "  $dst"
}

# 1. Binario: compilado aquí si hay cargo; si no, el de la última versión publicada.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
if command -v cargo >/dev/null && [[ -f $REPO/linux/Cargo.toml ]]; then
  echo "$(msg "Compilando screensaver-fx…" "Building screensaver-fx…")"
  (cd "$REPO" && cargo build --release -p ssfx-term --quiet)
  cp "$REPO/target/release/screensaver-fx" "$tmp/screensaver-fx"
else
  asset="screensaver-fx-linux-$(uname -m)"
  echo "$(msg "Descargando $asset…" "Downloading $asset…")"
  curl -fsSL "$RELEASES/$asset" -o "$tmp/screensaver-fx" ||
    { echo "$(msg "No se pudo descargar $asset. Instala Rust (rustup) y vuelve a ejecutar esto para compilarlo." "Could not download $asset. Install Rust (rustup) and run this again to build it.")" >&2; exit 1; }
fi
chmod +x "$tmp/screensaver-fx"
"$tmp/screensaver-fx" --list >/dev/null || { echo "$(msg "El binario no arranca en este equipo." "The binary does not start on this machine.")" >&2; exit 1; }

echo "$(msg "Instalando:" "Installing:")"
put "$tmp/screensaver-fx" "$BIN/screensaver-fx"
put "$HERE/omarchy/bin/ttfx" "$BIN/ttfx"
put "$HERE/omarchy/bin/omarchy-screensaver" "$BIN/omarchy-screensaver"
put "$HERE/omarchy/screensaver-check.sh" "$HOOKS/screensaver-fx-check"

# 2. Restos de omarchy-screensaver-fx (la versión anterior, en Python): sus enlaces ya no hacen falta.
for old in "$BRANDING/screensaver-fx.py" "$BRANDING/generate-screensaver.py"; do
  if [[ -L $old ]]; then
    rm "$old" && echo "  $(msg "quitado el enlace antiguo" "removed the old link") $old"
  fi
done

# 3. ~/.local/bin debe ir antes que /usr/bin en el PATH de Hyprland
hypr_pid=$(pgrep -x Hyprland | head -1 || true)
if [[ -n $hypr_pid ]]; then
  path=$(tr '\0' '\n' <"/proc/$hypr_pid/environ" | sed -n 's/^PATH=//p')
  local_i=$(tr ':' '\n' <<<"$path" | grep -nxF "$HOME/.local/bin" | head -1 | cut -d: -f1)
  usr_i=$(tr ':' '\n' <<<"$path" | grep -nxF /usr/bin | head -1 | cut -d: -f1)
  if [[ -z $local_i || ( -n $usr_i && $local_i -gt $usr_i ) ]]; then
    echo "$(msg "Aviso: en el PATH de Hyprland ~/.local/bin no va antes que /usr/bin; los envoltorios no se usarán." "Warning: in the Hyprland PATH ~/.local/bin does not come before /usr/bin; the wrappers will not be used.")" >&2
  fi
fi

echo
bash "$HOOKS/screensaver-fx-check"
echo "$(msg "Pruébalo con" "Try it with"): omarchy-launch-screensaver force"
