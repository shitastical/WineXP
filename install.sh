#!/usr/bin/env bash
#
# Install WineXP for the current user (no root needed).
#
# WineXP ships as a single-file AppImage, so "installing" it means dropping one
# file in a known place and pointing a launcher at it. That is also exactly what
# an update does: replace that one file (`src/update.rs` uses `$APPIMAGE`).
#
# Usage:
#   ./install.sh                        # install the artifact next to this script
#                                       # (an AppImage, or the raw binary)
#   ./install.sh WineXP-0.1.0-x86_64.AppImage
#   ./install.sh --from-github          # latest GitHub release, verified by sha256
#   ./install.sh --uninstall
#
# Layout:
#   ~/.local/share/winxp/WineXP.AppImage   the application (one file)
#   ~/.local/bin/winxp                     launcher
#   ~/.local/share/applications/winxp.desktop
#   ~/.local/share/icons/hicolor/256x256/apps/winxp.png
set -euo pipefail

REPO="${WINXP_UPDATE_REPO:-shitastical/WineXP}"
ASSET="winxp-linux-x86_64"
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}/winxp"
BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
APP="$PREFIX/WineXP.AppImage"

die() { echo "install.sh: $*" >&2; exit 1; }

uninstall() {
  rm -rf "$PREFIX"
  rm -f "$BIN_DIR/winxp" "$DESKTOP_DIR/winxp.desktop" "$ICON_DIR/winxp.png"
  echo "==> WineXP removed (your ~/.local/share/winxp settings were deleted too)."
  exit 0
}

# ---------------------------------------------------------------------------
# Fetch the newest release asset, verified against the published digest.
# ---------------------------------------------------------------------------
fetch_latest() {
  local dest="$1" tmp url digest
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  command -v python3 >/dev/null 2>&1 || die "--from-github needs python3 to read the release"
  echo "==> Looking up the newest release of $REPO…"
  curl -fsSL --proto '=https' --max-time 60 \
    -H 'Accept: application/vnd.github+json' -H 'User-Agent: WineXP' \
    "https://api.github.com/repos/$REPO/releases?per_page=10" > "$tmp/releases.json"
  # Pick the first non-draft release that publishes our asset, with its digest.
  read -r url digest < <(python3 - "$tmp/releases.json" "$ASSET" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
asset_name = sys.argv[2]
for release in data:
    if release.get("draft"):
        continue
    for asset in release.get("assets", []):
        if asset["name"] == asset_name:
            digest = asset.get("digest") or ""
            if digest.startswith("sha256:"):
                print(asset["browser_download_url"], digest.split(":", 1)[1])
                raise SystemExit
PY
)
  [[ -n "${url:-}" && -n "${digest:-}" ]] || die "no release publishes $ASSET with a sha256 digest"
  echo "==> Downloading $url"
  curl -fsSL --proto '=https' --max-time 600 -o "$dest" "$url"
  echo "$digest  $dest" | sha256sum -c - >/dev/null || die "the download failed its sha256 check — not installing it"
  echo "    sha256 verified"
}

# ---------------------------------------------------------------------------
# Install one file (an AppImage, or the plain binary) as $APP.
# ---------------------------------------------------------------------------
install_file() {
  local source="$1"
  [[ -f "$source" ]] || die "no such file: $source"
  if head -c 4 "$source" | grep -q '^#!' ; then
    echo "==> $source is a launcher script, not a build; pass the AppImage or the binary" >&2
    exit 2
  fi
  mkdir -p "$PREFIX" "$BIN_DIR" "$DESKTOP_DIR" "$ICON_DIR"
  # Keep the build being replaced, so a bad install is one `mv` from undone.
  [[ -f "$APP" ]] && cp -f "$APP" "$APP.previous"
  cp -f "$source" "$APP"
  chmod 755 "$APP"
  echo "==> Installed $APP"

  cat > "$BIN_DIR/winxp" <<EOF
#!/bin/sh
# WineXP launcher (installed by install.sh). The application self-updates the
# AppImage below, so this wrapper never needs to change.
exec "$APP" "\$@"
EOF
  chmod 755 "$BIN_DIR/winxp"

  # Icon for the desktop entry: the project's XP logo, scaled to 256x256.
  icon_source="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/assets/login/logo__windows_xp.png"
  if [[ -f "$icon_source" ]]; then
    if command -v convert >/dev/null 2>&1; then
      convert "$icon_source" -background none -gravity center -resize 256x256 -extent 256x256 \
        "$ICON_DIR/winxp.png" 2>/dev/null || cp -f "$icon_source" "$ICON_DIR/winxp.png"
    else
      cp -f "$icon_source" "$ICON_DIR/winxp.png"
    fi
  fi

  cat > "$DESKTOP_DIR/winxp.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=WineXP
Comment=Windows XP desktop, natively in Rust, on top of Wine
Exec=$BIN_DIR/winxp
Icon=winxp
Categories=System;DesktopSettings;
Terminal=false
StartupWMClass=winexp
EOF

  echo "==> Launcher: $BIN_DIR/winxp"
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "    (add $BIN_DIR to PATH to run \`winxp\` from anywhere)" ;;
  esac
  echo "==> Updates: \`winxp --check-update\` checks GitHub releases;"
  echo "    \`winxp --stage-update\` downloads and verifies the next one."
}

if [[ "${1:-}" == "--uninstall" ]]; then
  uninstall
elif [[ "${1:-}" == "--from-github" ]]; then
  tmpfile="$(mktemp)"
  fetch_latest "$tmpfile"
  install_file "$tmpfile"
  rm -f "$tmpfile"
elif [[ -n "${1:-}" ]]; then
  install_file "$1"
else
  # No argument: install whatever was built or downloaded next to this script.
  here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  candidate=""
  for path in "$here"/WineXP-*-x86_64.AppImage "$here"/dist/winxp-linux-x86_64 "$here"/dist/winxp-linux-x86_64.raw "$here"/target/release/winxp; do
    [[ -f "$path" ]] && candidate="$path" && break
  done
  [[ -n "$candidate" ]] || die "nothing to install — run scripts/package.sh first, or pass an artifact"
  install_file "$candidate"
fi
