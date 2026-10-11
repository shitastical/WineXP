#!/usr/bin/env bash
#
# Package WineXP as a single-file AppImage (and a raw binary) for a release.
#
# The AppImage is the primary artifact because it makes the self-updater almost
# trivial: an update replaces one file, and `$APPIMAGE` tells the running build
# exactly which file to swap (see `src/update.rs`).
#
# The release asset that the updater installs is named exactly
# `winxp-linux-x86_64` (see `update::GITHUB_ASSET`), and it *is* the AppImage.
#
# Usage:
#   scripts/package.sh                 # build, then package AppImage + binary
#   scripts/package.sh --no-appimage   # raw binary only (no runtime download)
#   scripts/package.sh --check         # build and verify the artifacts only
#
# Output (all under dist/):
#   WineXP-<version>-x86_64.AppImage     the AppImage users run
#   winxp-linux-x86_64                   the same file under the updater's name
#   winxp-linux-x86_64.sha256            its digest, published with the release
#   winxp-linux-x86_64.raw               the plain binary (no FUSE needed)
#   WineXP-<version>-linux-x86_64.tar.gz binary + installer + docs
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APPIMAGE=1
CHECK_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --no-appimage) APPIMAGE=0 ;;
    --check)       CHECK_ONLY=1 ;;
    -h|--help)     sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "package.sh: unknown option '$arg'" >&2; exit 2 ;;
  esac
done

# ---------------------------------------------------------------------------
# Versions: Cargo's semver is what the updater compares; DISPLAY_VERSION is what
# users see ("v0.01.0 pre-alpha"). Both come from the source, never typed here.
# ---------------------------------------------------------------------------
SEMVER="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
DISPLAY="$(sed -n 's/^pub const DISPLAY_VERSION: &str = "\(.*\)";/\1/p' src/update.rs | head -1)"
if [[ -z "$SEMVER" || -z "$DISPLAY" ]]; then
  echo "package.sh: could not read the version from Cargo.toml / src/update.rs" >&2
  exit 1
fi
# The tag is the display version's number, e.g. `v0.01.0` for "v0.01.0 pre-alpha".
TAG="$(echo "$DISPLAY" | awk '{print $1}')"
[[ "$TAG" == v* ]] || TAG="v$TAG"
echo "==> WineXP $DISPLAY (semver $SEMVER, release tag $TAG)"

# ---------------------------------------------------------------------------
# 1. Build.
# ---------------------------------------------------------------------------
echo "==> Building (release)…"
cargo build --release --locked 2>/dev/null || cargo build --release
BIN="target/release/winxp"
[[ -x "$BIN" ]] || { echo "package.sh: $BIN was not built" >&2; exit 1; }

DIST="$ROOT/dist"
rm -rf "$DIST"
mkdir -p "$DIST"

# ---------------------------------------------------------------------------
# 2. The raw binary is always produced: it is the fallback artifact, and the
#    thing the tarball ships.
# ---------------------------------------------------------------------------
cp "$BIN" "$DIST/winxp-linux-x86_64.raw"
chmod 755 "$DIST/winxp-linux-x86_64.raw"

if [[ "$APPIMAGE" == 0 ]]; then
  echo "==> Skipping the AppImage (--no-appimage)"
  cp "$DIST/winxp-linux-x86_64.raw" "$DIST/winxp-linux-x86_64"
  IMAGE="" 
else
  # -------------------------------------------------------------------------
  # 3. AppDir: the filesystem the AppImage mounts.
  # -------------------------------------------------------------------------
  APP_DIR="$DIST/AppDir"
  mkdir -p "$APP_DIR/usr/bin" "$APP_DIR/usr/share/icons/hicolor/256x256/apps"
  cp "$BIN" "$APP_DIR/usr/bin/winxp"
  chmod 755 "$APP_DIR/usr/bin/winxp"

  cat > "$APP_DIR/AppRun" <<'APPRUN'
#!/bin/sh
# The AppImage entry point: run the embedded WineXP.
exec "$APPDIR/usr/bin/winxp" "$@"
APPRUN
  chmod 755 "$APP_DIR/AppRun"

  # Icon: the XP logo scaled to the 256x256 AppImage convention.
  ICON_SRC="assets/login/logo__windows_xp.png"
  if command -v convert >/dev/null 2>&1; then
    convert "$ICON_SRC" -background none -gravity center -resize 256x256 -extent 256x256 \
      "$APP_DIR/winxp.png"
  else
    cp "$ICON_SRC" "$APP_DIR/winxp.png"
  fi
  cp "$APP_DIR/winxp.png" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/winxp.png"

  cat > "$APP_DIR/winxp.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=WineXP
Comment=Windows XP desktop, natively in Rust, on top of Wine
Exec=winxp
Icon=winxp
Categories=System;DesktopSettings;
Terminal=false
StartupWMClass=winexp
DESKTOP

  # -------------------------------------------------------------------------
  # 4. Squashfs + the AppImage runtime (the runtime is fetched once and cached).
  # -------------------------------------------------------------------------
  command -v mksquashfs >/dev/null 2>&1 || {
    echo "package.sh: mksquashfs is needed to build an AppImage (install squashfs-tools)" >&2
    exit 1
  }
  echo "==> Building the squashfs…"
  mksquashfs "$APP_DIR" "$DIST/winxp.squashfs" -root-owned -noappend -no-progress -comp gzip >/dev/null

  CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/winxp-appimage"
  RUNTIME="$CACHE/runtime-x86_64"
  if [[ ! -s "$RUNTIME" ]]; then
    mkdir -p "$CACHE"
    echo "==> Fetching the AppImage runtime…"
    curl -fsSL --proto '=https' --max-time 120 -o "$RUNTIME" \
      "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64" \
      || curl -fsSL --proto '=https' --max-time 120 -o "$RUNTIME" \
        "https://github.com/AppImage/AppImageKit/releases/download/continuous/runtime-x86_64"
  fi
  [[ -s "$RUNTIME" ]] || { echo "package.sh: could not fetch the AppImage runtime" >&2; exit 1; }

  IMAGE="$DIST/WineXP-$SEMVER-x86_64.AppImage"
  cat "$RUNTIME" "$DIST/winxp.squashfs" > "$IMAGE"
  chmod 755 "$IMAGE"
  rm -f "$DIST/winxp.squashfs"
  rm -rf "$APP_DIR"

  # The artifact the updater installs: the AppImage under its release name.
  cp "$IMAGE" "$DIST/winxp-linux-x86_64"
  chmod 755 "$DIST/winxp-linux-x86_64"

  # -------------------------------------------------------------------------
  # 5. Verify the packaged AppImage actually runs and reports the right version.
  # -------------------------------------------------------------------------
  echo "==> Verifying $IMAGE…"
  GOT="$(APPIMAGE_EXTRACT_AND_RUN=1 "$IMAGE" --version)"
  if [[ "$GOT" != "$DISPLAY" ]]; then
    echo "package.sh: the AppImage reports '$GOT', expected '$DISPLAY'" >&2
    exit 1
  fi
  echo "    reports: $GOT"
fi

# ---------------------------------------------------------------------------
# 6. Digest + tarball for release publishing.
# ---------------------------------------------------------------------------
( cd "$DIST" && sha256sum "winxp-linux-x86_64" > "winxp-linux-x86_64.sha256" )

TARBALL="$DIST/WineXP-$SEMVER-linux-x86_64.tar.gz"
tar -czf "$TARBALL" -C "$ROOT" \
  --transform "s,^,WineXP-$SEMVER/," \
  dist/winxp-linux-x86_64.raw install.sh LICENSE README.md docs/UPDATE.md

echo
echo "==> Artifacts in dist/"
ls -la "$DIST" | grep -vE '^total|^d'
echo
echo "==> Publish with:"
echo "    gh release create $TAG \\"
echo "      \"$DIST/winxp-linux-x86_64\" \\"
echo "      \"$DIST/winxp-linux-x86_64.sha256\" \\"
echo "      \"$TARBALL\" \\"
echo "      --title \"WineXP $DISPLAY\" --notes-file docs/RELEASE-NOTES.md"
echo
echo "    Asset name 'winxp-linux-x86_64' is what WineXP's updater installs."
if [[ "$CHECK_ONLY" == 1 ]]; then
  echo "==> --check: artifacts verified, nothing published."
fi
