#!/usr/bin/env bash
# WineXP development launcher and control center. No root or package installation.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
PROFILE=release
BUILD=1
ACTION=run
MENU=0
APP_ARGS=()
STATE_DIR="$ROOT/target/launcher"
PIDFILE="$STATE_DIR/winxp.pid"
LOG="$STATE_DIR/latest.log"
COLOR=0
[[ -t 1 && -z "${NO_COLOR:-}" && "${TERM:-dumb}" != dumb ]] && COLOR=1
if (( COLOR )); then
  BLUE=$'\033[1;34m'; CYAN=$'\033[1;36m'; GREEN=$'\033[1;32m'; YELLOW=$'\033[1;33m'; RED=$'\033[1;31m'; RESET=$'\033[0m'
else
  BLUE=''; CYAN=''; GREEN=''; YELLOW=''; RED=''; RESET=''
fi
say() { printf '%s\n' "$*"; }
info() { printf '%s[WineXP]%s %s\n' "$CYAN" "$RESET" "$*"; }
warn() { printf '%s[Warning]%s %s\n' "$YELLOW" "$RESET" "$*" >&2; }
fail() { printf '%s[Error]%s %s\n' "$RED" "$RESET" "$*" >&2; }
usage() {
  cat <<'HELP'
WineXP — Windows XP desktop for Linux

Usage: ./run.sh [options] [-- application arguments]

  --menu          Interactive WineXP control center (default in a terminal)
  --run           Build and launch without the menu
  --debug         Use the debug build
  --release       Use the release build (default)
  --no-build      Use the existing binary
  --build         Build only
  --install       Install WineXP for this user; preserves settings/Wine files
  --status        Show installation and update status
  --check-update  Check for a newer verified release
  --stage-update  Download/verify an update for the next launch
  --doctor        Show Linux, Wine, browser and build diagnostics
  --logs          Print the last launch/build log
  --stop          Stop only the instance tracked by this checkout
  -h, --help      Show this help

Examples:
  ./run.sh --debug --run -- --skip-boot --open 'Internet Explorer'
  ./run.sh --no-build --doctor
  ./run.sh --release --install

No dependencies are installed automatically. NO_COLOR disables colors.
Logs: target/launcher/latest.log
HELP
}
if [[ $# == 0 && -t 0 && -t 1 ]]; then MENU=1; fi
while (( $# )); do
  case "$1" in
    --menu) MENU=1 ;;
    --run) ACTION=run; MENU=0 ;;
    --debug) PROFILE=debug ;;
    --release) PROFILE=release ;;
    --no-build) BUILD=0 ;;
    --build) ACTION=build; MENU=0 ;;
    --install) ACTION=install; MENU=0 ;;
    --status) ACTION=status; MENU=0 ;;
    --check-update) ACTION=check-update; MENU=0 ;;
    --stage-update) ACTION=stage-update; MENU=0 ;;
    --doctor) ACTION=doctor; MENU=0 ;;
    --logs) ACTION=logs; MENU=0 ;;
    --stop) ACTION=stop; MENU=0 ;;
    --) shift; APP_ARGS=("$@"); break ;;
    -h|--help) usage; exit 0 ;;
    *) fail "Unknown option: $1"; usage >&2; exit 2 ;;
  esac
  shift
done
binary() { printf '%s/target/%s/winxp' "$ROOT" "$PROFILE"; }
header() {
  printf '\n%s  ┌──────────────────────────────────────────┐\n' "$BLUE"
  printf '  │  WineXP · Desktop Control Center         │\n'
  printf '  └──────────────────────────────────────────┘%s\n\n' "$RESET"
  printf '  Build: %s   Wine prefix: %s\n' "$PROFILE" "${WINEPREFIX:-${HOME:-}/.wine}"
}
require_binary() {
  if [[ ! -x "$(binary)" ]]; then fail "No $PROFILE build found. Choose Build or remove --no-build."; return 1; fi
}
build() {
  command -v cargo >/dev/null 2>&1 || { fail 'Rust/Cargo is missing. Install a Rust toolchain or use a prebuilt WineXP AppImage.'; return 1; }
  mkdir -p "$STATE_DIR"
  info "Building WineXP ($PROFILE)…"
  local status=0
  if [[ "$PROFILE" == release ]]; then
    cargo build --release 2>&1 | tee "$LOG" || status=$?
  else
    cargo build 2>&1 | tee "$LOG" || status=$?
  fi
  if (( status )); then fail "Build failed (exit $status). Full output: $LOG"; return "$status"; fi
  printf '%s[Ready]%s %s\n' "$GREEN" "$RESET" "$(binary)"
}
# Never kill processes by executable name: another WineXP checkout or another
# user's application must not be terminated just because it has the same name.
tracked_pid() {
  [[ -f "$PIDFILE" ]] || return 1
  local pid executable expected
  read -r pid expected < "$PIDFILE" || return 1
  [[ "$pid" =~ ^[0-9]+$ && "$pid" -gt 1 ]] || return 1
  kill -0 "$pid" 2>/dev/null || return 1
  if [[ -e "/proc/$pid/exe" ]]; then
    executable="$(readlink "/proc/$pid/exe" 2>/dev/null)" || return 1
    [[ "$executable" == "$expected" || "$executable" == "$expected (deleted)" ]] || return 1
    [[ "$expected" == "$ROOT/target/debug/winxp" || "$expected" == "$ROOT/target/release/winxp" ]] || return 1
  else
    return 1
  fi
  printf '%s' "$pid"
}
stop_existing() {
  local pid
  if pid="$(tracked_pid)"; then
    info "Stopping tracked WineXP instance ($pid)…"
    kill -TERM "$pid" 2>/dev/null || { fail 'Unable to signal the tracked process.'; return 1; }
    for (( attempt=0; attempt<50; attempt++ )); do
      if ! kill -0 "$pid" 2>/dev/null; then rm -f "$PIDFILE"; return 0; fi
      sleep 0.1
    done
    fail 'WineXP did not stop within five seconds. Not force-killing it; close the application and retry.'
    return 1
  fi
  [[ ! -e "$PIDFILE" ]] || rm -f "$PIDFILE"
  info 'No tracked WineXP instance is running.'
}
doctor() {
  header
  say "  System: $(uname -srmo 2>/dev/null || uname -a)"
  if [[ -r /etc/os-release ]]; then
    local distro
    distro="$(sed -n 's/^PRETTY_NAME=//p' /etc/os-release | head -n 1)"
    say "  Distribution: ${distro//\"/}"
  fi
  say "  Session: ${XDG_SESSION_TYPE:-unknown}"
  say "  DISPLAY: ${DISPLAY:-not set}"
  say "  WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-not set}"
  say "  Data directory: ${XDG_DATA_HOME:-${HOME:-}/.local/share}/winxp"
  say ''
  for tool in cargo rustc wine python3 systemctl; do
    if command -v "$tool" >/dev/null 2>&1; then say "  [found] $tool: $(command -v "$tool")"; else say "  [missing] $tool"; fi
  done
  local python="${WINXP_BROWSER_PYTHON:-/usr/bin/python3}"
  if [[ -x "$python" ]]; then
    if "$python" - <<'PY'
import gi
import cairo
gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import Gtk, WebKit2
print('  [found] WebKitGTK 4.1 + GTK 3 + Python GI/Cairo')
PY
    then :; else
      warn 'Live browsing dependencies unavailable.'
      say '  Debian/Ubuntu packages: python3-gi python3-gi-cairo gir1.2-gtk-3.0 gir1.2-webkit2-4.1'
      say '  Other distributions: equivalent GTK3, WebKitGTK 4.1 typelibs and Python GI/Cairo packages.'
    fi
  else warn "Browser Python not found: $python"; fi
  [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]] || warn 'No graphical display. CLI diagnostics/update commands still work.'
  if [[ -n "${WAYLAND_DISPLAY:-}" && -z "${DISPLAY:-}" ]]; then warn 'Pure Wayland browser rendering is not validated; XWayland is currently required.'; fi
  if [[ -x "$(binary)" ]]; then
    say ''; "$(binary)" --installation-status
  fi
  say ''; say '  Wine is not a security sandbox. Run only installers you trust.'
}
APP_PID=''
cleanup() {
  if [[ -n "$APP_PID" && -f "$PIDFILE" ]]; then
    local recorded=''
    read -r recorded _ < "$PIDFILE" || true
    [[ "$recorded" != "$APP_PID" ]] || rm -f "$PIDFILE"
  fi
}
interrupt() {
  [[ -z "$APP_PID" ]] || kill -TERM "$APP_PID" 2>/dev/null || true
}
trap cleanup EXIT
trap interrupt INT TERM
launch() {
  require_binary || return
  if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
    fail 'No graphical display available. Run --doctor for details.'; return 1
  fi
  stop_existing || return
  mkdir -p "$STATE_DIR"
  info 'Starting WineXP…'
  info "Log: $LOG"
  printf 'WineXP launch %s\nBinary: %s\n' "$(date -Is)" "$(binary)" > "$LOG"
  "$(binary)" "${APP_ARGS[@]}" >> "$LOG" 2>&1 &
  APP_PID=$!
  printf '%s %s\n' "$APP_PID" "$(binary)" > "$PIDFILE"
  local status=0
  wait "$APP_PID" || status=$?
  cleanup
  APP_PID=''
  if (( status )); then
    fail "WineXP exited with status $status. Recent output:"
    tail -n 30 "$LOG" >&2
    return "$status"
  fi
  info 'WineXP closed normally.'
}
perform() {
  case "$ACTION" in
    doctor) doctor ;;
    logs) [[ -f "$LOG" ]] && cat "$LOG" || { warn 'No launcher log yet.'; return 1; } ;;
    stop) stop_existing ;;
    build) build ;;
    run) if (( BUILD )); then build || return; fi; launch ;;
    install)
      if (( BUILD )); then build || return; fi
      require_binary || return
      info 'Installing for this user only; preserving Wine files and account settings.'
      "$(binary)" --install
      ;;
    status|check-update|stage-update)
      require_binary || return
      case "$ACTION" in
        status) "$(binary)" --installation-status; "$(binary)" --update-status ;;
        check-update) "$(binary)" --check-update ;;
        stage-update) "$(binary)" --stage-update ;;
      esac
      ;;
  esac
}
menu() {
  [[ -t 0 && -t 1 ]] || { fail '--menu needs an interactive terminal.'; return 2; }
  local choice
  while true; do
    header
    cat <<'MENU'
  1  Build & start WineXP
  2  Start existing build
  3  Build only
  4  Install WineXP for this user
  5  Installation & update status
  6  Check for updates
  7  Download verified update
  8  System & dependency diagnostics
  9  View latest log
  s  Stop tracked WineXP instance
  p  Toggle debug/release build
  q  Quit
MENU
    printf '\n  Choice: '
    if ! read -r choice; then return 0; fi
    case "$choice" in
      1) ACTION=run; BUILD=1 ;;
      2) ACTION=run; BUILD=0 ;;
      3) ACTION=build ;;
      4) ACTION=install; BUILD=1 ;;
      5) ACTION=status ;;
      6) ACTION=check-update ;;
      7) ACTION=stage-update ;;
      8) ACTION=doctor ;;
      9) ACTION=logs ;;
      s|S) ACTION=stop ;;
      p|P) if [[ "$PROFILE" == release ]]; then PROFILE=debug; else PROFILE=release; fi; continue ;;
      q|Q) return 0 ;;
      *) warn 'Choose a listed action.'; continue ;;
    esac
    if perform; then :; else warn 'The action did not complete. See details above.'; fi
    printf '\n  Press Enter to return to the menu…'
    read -r _ || return 0
  done
}
if (( MENU )); then menu; else perform; fi
