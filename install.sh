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
#   ~/.local/share/icons/hicolor/<size>/apps/winxp.png
set -euo pipefail

REPO="${WINXP_UPDATE_REPO:-shitastical/WineXP}"
ASSET="winxp-linux-x86_64"
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}/winxp"
BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
APP="$PREFIX/WineXP.AppImage"

die() { echo "install.sh: $*" >&2; exit 1; }

uninstall() {
  # Remove only the installed application/integration, never accounts, settings,
  # user documents, backups or the Wine prefix.
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user disable --now winxp-update.timer >/dev/null 2>&1 || true
  fi
  rm -f "$HOME/.config/systemd/user/winxp-update.timer" "$HOME/.config/systemd/user/winxp-update.service"
  rm -f "$APP" "$APP.previous" "$PREFIX/installation.json"
  rm -f "$BIN_DIR/winxp" "$DESKTOP_DIR/winxp.desktop"
  for size in 16x16 32x32 48x48 64x64 128x128 256x256 512x512; do
    rm -f "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/$size/apps/winxp.png"
  done
  rm -f "${XDG_DATA_HOME:-$HOME/.local/share}/pixmaps/winxp.png" \
        "${XDG_DATA_HOME:-$HOME/.local/share}/share/pixmaps/winxp.png" \
        "${XDG_DATA_HOME:-$HOME/.local/share}/icons/winxp.png"
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" >/dev/null 2>&1 || true
  fi
  if command -v xdg-desktop-menu >/dev/null 2>&1; then
    xdg-desktop-menu uninstall --novendor "$DESKTOP_DIR/winxp.desktop" >/dev/null 2>&1 || true
  fi
  if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload >/dev/null 2>&1 || true
  fi
  echo "==> WineXP removed. Settings, accounts, backups and Wine programs were preserved."
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
  mkdir -p "$PREFIX" "$BIN_DIR" "$DESKTOP_DIR"
  # Keep the build being replaced, so a bad install is one `mv` from undone.
  if [[ ! -e "$APP" || ! "$source" -ef "$APP" ]]; then
    [[ ! -f "$APP" ]] || cp -f "$APP" "$APP.previous"
    cp -f "$source" "$APP.installing"
    chmod 755 "$APP.installing"
    APPIMAGE_EXTRACT_AND_RUN=1 "$APP.installing" --version >/dev/null || die "copied application self-check failed; existing application preserved"
    mv -f "$APP.installing" "$APP"
  fi
  chmod 755 "$APP"
  echo "==> Installed $APP"

  cat > "$BIN_DIR/winxp" <<EOF
#!/bin/sh
# WineXP launcher (installed by install.sh). The application self-updates the
# AppImage below, so this wrapper never needs to change.
export APPIMAGE_EXTRACT_AND_RUN=1
exec "$APP" "\$@"
EOF
  chmod 755 "$BIN_DIR/winxp"

  # Let the binary own the menu entry/icons/caches (single implementation),
  # so install.sh and `winxp --install` can never drift apart.
  APPIMAGE_EXTRACT_AND_RUN=1 "$APP" --install || die "WineXP installation self-check failed"

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
