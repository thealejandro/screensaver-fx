#!/usr/bin/env bash
# Publica la versión recién lanzada en Homebrew (thealejandro/homebrew-tap) y en winget
# (PR a microsoft/winget-pkgs desde el fork thealejandro/winget-pkgs).
#   GH_TOKEN=<PAT con public_repo> bash .github/packages.sh 0.1.16
# Con DRY=1 solo genera los archivos en $OUT (por defecto ./pkg-out) sin subir nada.
set -euo pipefail

V="${1:?versión}"
REPO=thealejandro/screensaver-fx
OUT="${OUT:-pkg-out}"
ID=thealejandro.ScreensaverFX
WDIR=manifests/t/thealejandro/ScreensaverFX
rm -rf "${OUT:?}" && mkdir -p "$OUT"

if [ -z "${GH_TOKEN:-}" ] && [ -z "${DRY:-}" ]; then
  echo "::warning::Falta el secreto PKG_TOKEN: no se actualizan Homebrew ni winget."
  exit 0
fi

# Sumas publicadas en la release (las mismas que comprueba el actualizador).
if [ -n "${SUMS:-}" ]; then cp "$SUMS" "$OUT/SHA256SUMS.txt"; else
  gh release download "v$V" -R "$REPO" -p SHA256SUMS.txt -D "$OUT"
fi
sum() { awk -v f="$1" '$2 == f { print $1 }' "$OUT/SHA256SUMS.txt"; }
SETUP=$(sum "ScreensaverFX-Setup-$V.exe")
ZIP=$(sum ScreensaverFX-macOS.zip)
[ -n "$SETUP" ] && [ -n "$ZIP" ] || { echo "Faltan sumas en SHA256SUMS.txt"; exit 1; }

# --- Homebrew --------------------------------------------------------------
mkdir -p "$OUT/tap/Casks"
CASK="$OUT/tap/Casks/screensaver-fx.rb"
if [ -n "${DRY:-}" ]; then cp "${CASK_SRC:?}" "$CASK"; else
  gh api repos/thealejandro/homebrew-tap/contents/Casks/screensaver-fx.rb -H "Accept: application/vnd.github.raw" > "$CASK"
fi
sed -i -E "s/^  version \".*\"/  version \"$V\"/; s/^  sha256 \".*\"/  sha256 \"$ZIP\"/" "$CASK"

# --- winget: la última versión de packaging/winget sirve de plantilla --------
PREV=$(ls -d packaging/winget/$WDIR/*/ | xargs -n1 basename | sort -V | tail -1)
WOUT="$OUT/winget/$WDIR/$V"
mkdir -p "$WOUT"
for f in packaging/winget/$WDIR/$PREV/*.yaml; do
  sed -E "s/${PREV//./\\.}/$V/g; s/^(  InstallerSha256: ).*/\1$SETUP/; s/^ReleaseDate: .*/ReleaseDate: $(date -u +%F)/" "$f" > "$WOUT/$(basename "$f")"
done

if [ -n "${DRY:-}" ]; then
  echo "DRY: archivos en $OUT"
  exit 0
fi

# Homebrew: commit directo al tap.
sha=$(gh api repos/thealejandro/homebrew-tap/contents/Casks/screensaver-fx.rb --jq .sha)
gh api -X PUT repos/thealejandro/homebrew-tap/contents/Casks/screensaver-fx.rb \
  -f message="screensaver-fx $V" -f sha="$sha" -f content="$(base64 -w0 "$CASK")" --jq .commit.sha
echo "Homebrew actualizado a $V"

# winget: solo cuando el paquete ya está en el catálogo (el primer PR lo aprueba Microsoft a mano).
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
