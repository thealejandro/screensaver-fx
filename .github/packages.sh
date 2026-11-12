#!/usr/bin/env bash
# Tras cada release: actualiza en este mismo repo el cask de Homebrew (Casks/), el manifiesto de
# Scoop (bucket/) y los de winget (packaging/winget/), y con WINGET_TOKEN abre el PR a winget-pkgs.
#   GH_TOKEN=<token con contents:write en este repo> bash .github/packages.sh 0.1.17
# Con DRY=1 solo cambia los archivos, sin commit ni push.
set -euo pipefail

V="${1:?versión}"
REPO=thealejandro/screensaver-fx
ID=thealejandro.ScreensaverFX
WDIR=manifests/t/thealejandro/ScreensaverFX
tmp=$(mktemp -d)

# Sumas publicadas en la release (las mismas que comprueba el actualizador).
if [ -n "${SUMS:-}" ]; then cp "$SUMS" "$tmp/SHA256SUMS.txt"; else
  gh release download "v$V" -R "$REPO" -p SHA256SUMS.txt -D "$tmp"
fi
sum() { awk -v f="$1" '$2 == f { print $1 }' "$tmp/SHA256SUMS.txt"; }
SETUP=$(sum "ScreensaverFX-Setup-$V.exe")
ZIP=$(sum ScreensaverFX-macOS.zip)
[ ${#SETUP} -eq 64 ] && [ ${#ZIP} -eq 64 ] || { echo "Faltan sumas en SHA256SUMS.txt"; exit 1; }

# Homebrew
sed -i -E "s/^  version \".*\"/  version \"$V\"/; s/^  sha256 \".*\"/  sha256 \"$ZIP\"/" Casks/screensaver-fx.rb

# Scoop
U="https://github.com/$REPO/releases/download/v$V/ScreensaverFX-Setup-$V.exe#/setup.exe"
jq --indent 4 --arg v "$V" --arg u "$U" --arg h "$SETUP" '.version=$v | .url=$u | .hash=$h' bucket/screensaver-fx.json > "$tmp/s.json"
mv "$tmp/s.json" bucket/screensaver-fx.json

# winget: la última versión de packaging/winget sirve de plantilla.
PREV=$(ls -d packaging/winget/$WDIR/*/ | xargs -n1 basename | sort -V | tail -1)
WOUT="packaging/winget/$WDIR/$V"
if [ "$PREV" != "$V" ]; then
  mkdir -p "$WOUT"
  for f in packaging/winget/$WDIR/$PREV/*.yaml; do
    sed -E "s/${PREV//./\.}/$V/g; s/^(  InstallerSha256: ).*/\1$SETUP/; s/^ReleaseDate: .*/ReleaseDate: $(date -u +%F)/" "$f" > "$WOUT/$(basename "$f")"
  done
fi

[ -n "${DRY:-}" ] && { echo "DRY: archivos actualizados"; git status --short; exit 0; }

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add Casks bucket packaging/winget
if git commit -qm "chore: Homebrew, Scoop y winget a $V"; then
  for _ in 1 2 3; do git pull -q --rebase && git push -q && break; sleep 5; done
  echo "Homebrew y Scoop en $V"
fi

# winget: PR a microsoft/winget-pkgs desde el fork, solo con token y cuando el paquete ya está
# en el catálogo (el primer PR lo aprueba Microsoft a mano).
if [ -z "${WINGET_TOKEN:-}" ]; then
  echo "::notice::Sin WINGET_TOKEN: el PR de winget $V se abre a mano con los archivos de $WOUT."
  exit 0
fi
export GH_TOKEN="$WINGET_TOKEN"
if ! gh api "repos/microsoft/winget-pkgs/contents/$WDIR" --silent 2>/dev/null; then
  echo "::warning::$ID aún no está en winget-pkgs; no se abre PR para $V."
  exit 0
fi
B="$ID-$V"
base=$(gh api repos/microsoft/winget-pkgs/git/ref/heads/master --jq .object.sha)
gh api -X POST repos/thealejandro/winget-pkgs/git/refs -f ref="refs/heads/$B" -f sha="$base" --silent
for f in "$WOUT"/*.yaml; do
  gh api -X PUT "repos/thealejandro/winget-pkgs/contents/$WDIR/$V/$(basename "$f")" -f branch="$B" \
    -f message="New version: $ID version $V" -f content="$(base64 -w0 "$f")" --silent
done
gh pr create -R microsoft/winget-pkgs --head "thealejandro:$B" --base master \
  --title "New version: $ID version $V" \
  --body "Automated update of $ID to $V from the [release](https://github.com/$REPO/releases/tag/v$V). Installer type and switches are unchanged from the previous version."
