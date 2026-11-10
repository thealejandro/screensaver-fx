#!/usr/bin/env bash
# Notas del release: tabla de descargas (qué archivo bajar según el equipo) y cambios desde el
# release anterior, sacados de los commits feat/fix/perf.
# Uso: .github/release-notes.sh v0.1.8 > notes.md
set -euo pipefail

TAG=${1:?uso: release-notes.sh <tag>}
VER=${TAG#v}
REPO=${GITHUB_REPOSITORY:-thealejandro/screensaver-fx}
DL="https://github.com/$REPO/releases/download/$TAG"
README="https://github.com/$REPO#readme"
PREV=$(git describe --tags --abbrev=0 --match 'v*' "$TAG^" 2>/dev/null || true)

cat <<EOF
## ¿Qué descargo?

| Tu equipo | Descarga | Cómo se instala |
|---|---|---|
| **Windows 10 / 11** · recomendado | [**ScreensaverFX-Setup.exe**]($DL/ScreensaverFX-Setup.exe) | Asistente con vista previa en vivo. Elige solo la versión Intel/AMD o ARM y se actualiza solo. |
| Windows, sin instalador · Intel / AMD | [ScreensaverFX-x64.scr]($DL/ScreensaverFX-x64.scr) | Clic derecho › **Instalar**. No se actualiza solo. |
| Windows, sin instalador · ARM (Snapdragon) | [ScreensaverFX-arm64.scr]($DL/ScreensaverFX-arm64.scr) | Clic derecho › **Instalar**. No se actualiza solo. |
| **macOS 11 o posterior** · recomendado | [**ScreensaverFX.dmg**]($DL/ScreensaverFX.dmg) | Ábrelo y arrastra el protector sobre «Screen Savers». |
| macOS, con asistente | [ScreensaverFX.pkg]($DL/ScreensaverFX.pkg) | Doble clic y sigue los pasos. |
| macOS, a mano | [ScreensaverFX-macOS.zip]($DL/ScreensaverFX-macOS.zip) | Descomprime y haz doble clic en \`ScreensaverFX.saver\`. |

¿Windows o macOS avisan de que la app no está firmada? Mira cómo abrirla en el [README]($README).

<details><summary>Otros archivos</summary>

- \`ScreensaverFX-Setup-$VER.exe\`: el mismo instalador de Windows, con la versión en el nombre (lo usa el actualizador).
- \`install.ps1\` / \`install.sh\`: instalación en una línea desde la terminal (Windows / macOS), ver el [README]($README).
- \`SHA256SUMS.txt\`: sumas SHA-256 para comprobar las descargas.

</details>
EOF

[ -n "$PREV" ] || exit 0
section() {
  local title=$1 type=$2 lines
  lines=$(git log --reverse --pretty=%s "$PREV..$TAG" | sed -nE "s/^$type(\([^)]*\))?!?: (.*)/\2/p")
  [ -n "$lines" ] || return 0
  printf '\n### %s\n\n' "$title"
  while IFS= read -r l; do printf -- '- %s\n' "${l^}"; done <<<"$lines"
}
printf '\n## Cambios en %s\n' "$VER"
section "Novedades" feat
section "Correcciones" fix
section "Rendimiento" perf
printf '\n**Todos los cambios:** https://github.com/%s/compare/%s...%s\n' "$REPO" "$PREV" "$TAG"
