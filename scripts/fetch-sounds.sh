#!/usr/bin/env bash
#
# Fetch the Windows XP system sounds WineXP plays.
#
# The engine itself (src/sound.rs) has no dependencies — it hands each WAV to
# whatever sound player the distribution already has. These files are the only
# part that is not in the repository, so this script is how a fresh clone gets
# them back.
#
#   ./scripts/fetch-sounds.sh
#
# NOTE ON LICENSING: these are Microsoft's Windows XP sound scheme, obtained
# from the public Reborn XP distribution (https://xp.quenq.com). They are
# included here for personal, non-commercial use only, and are not covered by
# this project's licence. See README.md before redistributing a build.
set -euo pipefail

BASE="https://xp.quenq.com/res/sounds"
SOUNDS=(startup shutdown logon logoff tada ding error exclamation balloon recycle chimes start)

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$here/assets/sounds"
mkdir -p "$out"

echo "==> Fetching ${#SOUNDS[@]} Windows XP system sounds into assets/sounds"
for sound in "${SOUNDS[@]}"; do
  target="$out/$sound.wav"
  if [[ -s "$target" ]] && [[ "$(head -c 4 "$target")" == "RIFF" ]]; then
    printf '    [have] %s\n' "$sound.wav"
    continue
  fi
  printf '    [get ] %s ' "$sound.wav"
  if curl -fsSL --max-time 120 -o "$target.part" "$BASE/$sound.wav" \
     && [[ "$(head -c 4 "$target.part" 2>/dev/null || true)" == "RIFF" ]]; then
    mv -f "$target.part" "$target"
    echo "ok"
  else
    rm -f "$target.part"
    echo "FAILED (not a WAV — the site may have moved it)"
  fi
done

# The 404 page this site serves for a missing sound is exactly 16009 bytes, so
# a file of that size is a saved error page rather than audio.
bad=0
for file in "$out"/*.wav; do
  [[ -e "$file" ]] || continue
  if [[ "$(head -c 4 "$file")" != "RIFF" ]]; then
    echo "==> removing $(basename "$file"): not a RIFF/WAVE file"
    rm -f "$file"
    bad=$((bad + 1))
  fi
done

echo "==> $(find "$out" -name '*.wav' | wc -l) sounds ready in assets/sounds"
if [[ $bad -gt 0 ]]; then
  echo "    ($bad file(s) were error pages and have been removed)"
fi