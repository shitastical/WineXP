#!/usr/bin/env bash
#
# Launch the native Windows XP desktop clone.
#
# Ensures only ONE instance is ever running: any previously started copy is
# terminated before the new one is started. Running this script again therefore
# always replaces the old window instead of opening a second one.
#
# Usage:
#   ./run.sh            # build (release) and run
#   ./run.sh --debug    # build (debug) and run
#   ./run.sh --no-build # run the last built binary without rebuilding
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BIN="winxp"
PIDFILE="$ROOT/.winxp.pid"

PROFILE="release"
BUILD=1
for arg in "$@"; do
  case "$arg" in
    --debug)    PROFILE="debug" ;;
    --release)  PROFILE="release" ;;
    --no-build) BUILD=0 ;;
    -h|--help)
      sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "run.sh: unknown option '$arg' (try --help)" >&2
      exit 2
      ;;
  esac
done

# ---------------------------------------------------------------------------
# 1. Make sure no previous instance is left running.
# ---------------------------------------------------------------------------
stop_existing() {
  # By recorded PID first.
  if [[ -f "$PIDFILE" ]]; then
    local pid
    pid="$(cat "$PIDFILE" 2>/dev/null || true)"
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
    rm -f "$PIDFILE"
  fi

  # Then by name / path, to also catch instances started outside this script.
  if command -v pkill >/dev/null 2>&1; then
    pkill -x "$BIN" 2>/dev/null || true
    pkill -f "target/(debug|release)/$BIN" 2>/dev/null || true
  fi

  # Wait (briefly) for them to exit.
  for _ in $(seq 1 50); do
    if ! pgrep -x "$BIN" >/dev/null 2>&1; then
      return
    fi
    sleep 0.1
  done

  # Still alive after 5s: force kill.
  if command -v pkill >/dev/null 2>&1; then
    pkill -9 -x "$BIN" 2>/dev/null || true
  fi
}

echo "==> Stopping any running $BIN instance…"
stop_existing

# ---------------------------------------------------------------------------
# 2. Build.
# ---------------------------------------------------------------------------
if [[ "$BUILD" == 1 ]]; then
  echo "==> Building ($PROFILE)…"
  if [[ "$PROFILE" == "release" ]]; then
    cargo build --release
  else
    cargo build
  fi
fi

BINPATH="$ROOT/target/$PROFILE/$BIN"
if [[ ! -x "$BINPATH" ]]; then
  echo "run.sh: binary not found at $BINPATH (drop --no-build to build it)" >&2
  exit 1
fi

if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
  echo "run.sh: no DISPLAY/WAYLAND_DISPLAY set — cannot open a window." >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# 3. Run (one instance).
# ---------------------------------------------------------------------------
cleanup() { rm -f "$PIDFILE"; }
trap cleanup EXIT

echo "==> Starting $BIN…"
"$BINPATH" &
APP_PID=$!
echo "$APP_PID" > "$PIDFILE"
wait "$APP_PID" || true
