#!/bin/bash
# Quita Screensaver FX de Omarchy. El salvapantallas vuelve a ser el de Omarchy.
set -euo pipefail

# Mensaje en el idioma de Omarchy: msg "texto en español" "text in English"
msg() {
  local l=
  read -r l _ 2>/dev/null <"${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/language" || true
  l=${l:-${LC_ALL:-${LC_TIME:-${LANG:-}}}}
  l=${l,,}
  if [[ ${l:0:2} == es ]]; then printf '%s' "$1"; else printf '%s' "$2"; fi
}

# Solo se borra lo que instaló install.sh: los envoltorios llevan nuestro nombre dentro.
for dst in ~/.local/bin/ttfx ~/.local/bin/omarchy-screensaver ~/.config/omarchy/hooks/post-update.d/screensaver-fx-check; do
  if [[ -f $dst && ! -L $dst ]] && grep -qs 'thealejandro/screensaver-fx\|screensaver-fx-check' "$dst"; then
    rm "$dst" && echo "  $(msg "quitado" "removed") $dst"
  fi
done
if [[ -x ~/.local/bin/screensaver-fx ]] && ~/.local/bin/screensaver-fx --version 2>/dev/null | grep -q '^screensaver-fx '; then
  rm ~/.local/bin/screensaver-fx && echo "  $(msg "quitado" "removed") ~/.local/bin/screensaver-fx"
fi
rm -f ~/.config/omarchy/branding/.screensaver-fx-banner
echo "$(msg "Listo. Tu screensaver.txt y ~/.config/screensaver-fx no se tocan." "Done. Your screensaver.txt and ~/.config/screensaver-fx are left untouched.")"
