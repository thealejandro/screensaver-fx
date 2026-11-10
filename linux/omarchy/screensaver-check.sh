#!/bin/bash
# Comprueba que Screensaver FX sigue encajando con la versión actual de Omarchy
# y ttfx. Se ejecuta tras cada `omarchy update` (hook post-update
# screensaver-fx-check) y también a mano: bash linux/omarchy/screensaver-check.sh
#
# Nunca rompe nada: si algo deja de encajar, el envoltorio de ttfx cede al
# comportamiento original de Omarchy y aquí se avisa de qué hay que adaptar.

BRANDING=~/.config/omarchy/branding
FX=$(command -v screensaver-fx || echo ~/.local/bin/screensaver-fx)
WRAP=~/.local/bin/ttfx
LOG=~/.local/state/omarchy/screensaver-check.log
problems=()

# Mensaje en el idioma de Omarchy: msg "texto en español" "text in English"
msg() {
  local l=
  read -r l _ 2>/dev/null <"${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/language" || true
  l=${l:-${LC_ALL:-${LC_TIME:-${LANG:-}}}}
  l=${l,,}
  if [[ ${l:0:2} == es ]]; then printf '%s' "$1"; else printf '%s' "$2"; fi
}
warn() { problems+=("$1"); }

# 1. Script original de Omarchy y cómo llama a ttfx
orig="${OMARCHY_PATH:-/usr/share/omarchy}/bin/omarchy-screensaver"
[[ -f $orig ]] || orig=$(type -ap omarchy-screensaver | grep -v "^$HOME/" | head -1)
if [[ ! -f $orig ]]; then
  warn "$(msg "No se encuentra el script original omarchy-screensaver (¿Omarchy lo movió o renombró?)." "The original omarchy-screensaver script was not found (did Omarchy move or rename it?).")"
else
  grep -q 'ttfx' "$orig" || warn "$(msg "El salvapantallas de Omarchy ya no usa ttfx: las animaciones propias no se aplican." "The Omarchy screensaver no longer uses ttfx: the custom animations are not applied.")"
  grep -q -- '--random-effect' "$orig" || warn "$(msg "Omarchy ya no pasa --random-effect a ttfx: el envoltorio deja pasar la llamada sin cambios." "Omarchy no longer passes --random-effect to ttfx: the wrapper lets the call through unchanged.")"
  grep -q 'branding/screensaver.txt' "$orig" || warn "$(msg "Omarchy cambió la ruta del texto del salvapantallas (antes branding/screensaver.txt)." "Omarchy changed the path of the screensaver text (it used to be branding/screensaver.txt).")"
  grep -qE 'pgrep[^#]*-x ttfx|pkill -x ttfx' "$orig" || warn "$(msg "Omarchy ya no vigila el proceso por el nombre ttfx: revisar cómo se cierran las animaciones propias." "Omarchy no longer watches the process by the name ttfx: check how the custom animations are closed.")"
fi

# 2. El envoltorio de ttfx tiene prioridad en el PATH
first=$(type -ap ttfx | head -1)
[[ $(realpath "$first" 2>/dev/null) == $(realpath "$WRAP" 2>/dev/null) ]] ||
  warn "$(msg "El ttfx que se usa es $first y no el envoltorio $WRAP (orden del PATH)." "The ttfx in use is $first and not the wrapper $WRAP (PATH order).")"

# 3. ttfx real: efectos y opciones de color
real=$(type -ap ttfx | while read -r c; do [[ $(realpath "$c") != $(realpath "$WRAP") ]] && { echo "$c"; break; }; done)
if [[ -z $real ]]; then
  warn "$(msg "No está instalado ttfx." "ttfx is not installed.")"
else
  n=$("$real" --help 2>/dev/null | awk '/^Commands:/{f=1;next} /^Options:/{f=0} f && $1!="help"' | wc -l)
  ((n > 0)) || warn "$(msg "No se pudo leer la lista de efectos de ttfx (cambió el formato de --help)." "Could not read the ttfx effect list (the --help format changed).")"
  "$real" wipe --help 2>/dev/null | grep -q -- '--final-gradient-stops' ||
    warn "$(msg "ttfx ya no admite --final-gradient-stops: los efectos saldrán solo con sus colores originales." "ttfx no longer supports --final-gradient-stops: effects will only use their original colors.")"
fi

# 4. Animaciones propias y colores del tema
if ! "$FX" --list >/dev/null 2>&1; then
  warn "$(msg "El binario screensaver-fx no está instalado o no arranca ($FX)." "The screensaver-fx binary is not installed or does not start ($FX).")"
else
  [[ -f ~/.local/state/omarchy/current/theme/colors.toml ]] ||
    warn "$(msg "Omarchy movió colors.toml del tema: las animaciones usan colores por defecto." "Omarchy moved the theme colors.toml: animations use default colors.")"
  for e in $("$FX" --list); do
    script -qec "stty cols 120 rows 40; '$FX' $e --seconds 0.5" /dev/null >/dev/null 2>&1 ||
      warn "$(msg "La animación propia «$e» falla." "The custom animation «$e» fails.")"
  done
fi

# 5. Texto del salvapantallas
if [[ ! -s $BRANDING/screensaver.txt ]]; then
  warn "$(msg "Falta $BRANDING/screensaver.txt o está vacío." "$BRANDING/screensaver.txt is missing or empty.")"
elif [[ -f $BRANDING/.screensaver-fx-banner ]] &&
  cmp -s "$BRANDING/screensaver.txt" "${OMARCHY_PATH:-/usr/share/omarchy}/logo.txt"; then
  # El rótulo lo generó screensaver-fx y Omarchy lo pisó con su logo: se vuelve a generar.
  "$FX" --write-banner >/dev/null 2>&1 ||
    warn "$(msg "screensaver.txt volvió al logo de Omarchy. Regenerar: screensaver-fx --write-banner" "screensaver.txt went back to the Omarchy logo. Regenerate: screensaver-fx --write-banner")"
fi

mkdir -p "$(dirname "$LOG")"
{
  echo "== $(date '+%F %T') · omarchy $(omarchy version 2>/dev/null) · $("${real:-ttfx}" --version 2>/dev/null)"
  if ((${#problems[@]})); then printf -- '- %s\n' "${problems[@]}"; else echo "OK"; fi
} >>"$LOG"

if ((${#problems[@]})); then
  echo "screensaver-fx: ${#problems[@]} $(msg "aviso(s), detalle en" "warning(s), details in") $LOG"
  printf -- '  - %s\n' "${problems[@]}"
  omarchy-notification-send -u normal "$(msg "Salvapantallas: revisar tras la actualización" "Screensaver: check after the update")" \
    "${problems[0]} ($(msg "detalle en" "details in") ~/.local/state/omarchy/screensaver-check.log)" >/dev/null 2>&1 || true
else
  echo "$(msg "screensaver-fx: todo encaja con esta versión de Omarchy." "screensaver-fx: everything fits this Omarchy version.")"
fi
exit 0
