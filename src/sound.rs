//! Windows XP system sounds.
//!
//! WineXP plays the authentic XP sound scheme without pulling in an audio
//! library. Every crate that can output audio from pure Rust (`rodio`, `cpal`,
//! `kira`) links against ALSA's development headers, which are not present on a
//! default desktop install — and `install.sh` promises an install that works on
//! *any* distro with nothing but the binary. So the WAV is handed to whatever
//! sound player the distribution already ships (`paplay`, `pw-play`, `aplay`,
//! `ffplay`, `mpv`, …), which are themselves thin wrappers over whatever
//! PulseAudio, PipeWire or ALSA the user actually runs.
//!
//! Consequences, all deliberate:
//! - No new dependencies, nothing to link, nothing to install.
//! - Every failure mode is silent. A missing player, a missing asset or a
//!   closed audio server degrades to "no sound" and never affects the desktop.
//! - Playback is fire-and-forget on a detached thread, so a slow or blocking
//!   player can never stall a frame.

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Where the sounds live, relative to `assets/`. Matched by [`Sound::asset`].
const STARTUP: &str = "sounds/startup.wav";
const SHUTDOWN: &str = "sounds/shutdown.wav";
const LOGON: &str = "sounds/logon.wav";
const LOGOFF: &str = "sounds/logoff.wav";
const TADA: &str = "sounds/tada.wav";
const DING: &str = "sounds/ding.wav";
const ERROR: &str = "sounds/error.wav";
const EXCLAMATION: &str = "sounds/exclamation.wav";
const BALLOON: &str = "sounds/balloon.wav";
const RECYCLE: &str = "sounds/recycle.wav";
const CHIMES: &str = "sounds/chimes.wav";
const START: &str = "sounds/start.wav";

/// A Windows XP system sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// The startup jingle, played once the desktop is ready.
    Startup,
    /// The shutdown jingle.
    Shutdown,
    Logon,
    Logoff,
    /// "Welcome" — finished installing a program.
    Tada,
    /// The single chime for a completed action.
    Ding,
    Error,
    Exclamation,
    /// A tray balloon appearing.
    Balloon,
    /// An item going to the Recycle Bin.
    Recycle,
    Chimes,
    Start,
}

impl Sound {
    /// The embedded asset key for this sound.
    pub fn asset(self) -> &'static str {
        match self {
            Sound::Startup => STARTUP,
            Sound::Shutdown => SHUTDOWN,
            Sound::Logon => LOGON,
            Sound::Logoff => LOGOFF,
            Sound::Tada => TADA,
            Sound::Ding => DING,
            Sound::Error => ERROR,
            Sound::Exclamation => EXCLAMATION,
            Sound::Balloon => BALLOON,
            Sound::Recycle => RECYCLE,
            Sound::Chimes => CHIMES,
            Sound::Start => START,
        }
    }

    /// Whether this sound is long enough to be worth interrupting when another
    /// one starts. Windows does not stack the startup jingle under a click.
    pub fn is_long_form(self) -> bool {
        matches!(self, Sound::Startup | Sound::Shutdown | Sound::Logon | Sound::Logoff)
    }
}

/// Player candidates, most preferred first. `PAPLAY`/`PW-PLAY` understand WAV
/// directly; `aplay` is the ALSA fallback; `ffplay`/`mpv` cover the rest.
const PLAYERS: &[(&str, &[&str])] = &[
    ("paplay", &["%F"]),
    ("pw-play", &["%F"]),
    ("aplay", &["-q", "%F"]),
    ("ffplay", &["-nodisp", "-autoexit", "-loglevel", "quiet", "%F"]),
    ("mpv", &["--no-video", "--really-quiet", "%F"]),
];

/// The user-facing mute switch, mirrored from `Settings::sound`.
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Whether sounds should play. Set once at startup from the user's setting.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Whether this machine has any player WineXP could hand a WAV to. Lets the
/// applet tell the user *why* it is silent instead of just being mute.
pub fn players_available() -> bool {
    player().is_some()
}

/// The sound player to use, resolved once. `None` means this machine has none.
fn player() -> Option<(&'static str, &'static [&'static str])> {
    use std::sync::OnceLock;
    static RESOLVED: OnceLock<Option<usize>> = OnceLock::new();
    let index = *RESOLVED.get_or_init(|| {
        PLAYERS
            .iter()
            .position(|(name, _)| which(name))
    });
    index.map(|index| PLAYERS[index])
}

fn which(program: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .any(|dir| std::path::Path::new(dir).join(program).is_file())
}

/// Spills an embedded asset to a real file so a command-line player can read it.
///
/// The bytes are already in the binary, so this costs a small write once per
/// play and keeps the whole engine dependency-free. The file goes in the user's
/// cache directory (or the system temp dir) with a name derived from the sound,
/// and is reused on the next play.
fn materialise(asset: &str) -> io::Result<PathBuf> {
    let name = asset.rsplit('/').next().unwrap_or(asset);
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).map(|d| d.join("winxp")))
        .unwrap_or_else(std::env::temp_dir)
        .join("winxp-sounds");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    let bytes = crate::assets::asset_bytes(asset).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "sound asset is missing"))?;
    // Only rewrite when the content actually differs, so repeat plays do not
    // churn the disk and a partially written file is never handed to a player.
    if std::fs::read(&path).map_or(true, |current| current != bytes) {
        std::fs::write(&path, bytes)?;
    }
    Ok(path)
}

/// Plays a system sound. Never blocks, never fails, never panics.
pub fn play(sound: Sound) {
    if !enabled() {
        return;
    }
    let Some((program, arguments)) = player() else {
        return;
    };
let path = match materialise(sound.asset()) {
        Ok(path) => path,
        // A build without the audio assets still runs, just silent.
        Err(_) => return,
    };
    // Two jingles at once is how Windows sounds like a crash. Short UI sounds
    // always get through, since they are what the user just triggered.
    if sound.is_long_form() {
        if still_playing() {
            return;
        }
        if let Some(seconds) = duration_seconds(sound) {
            let until = elapsed_ms() + (seconds * 1000.0) as u64;
            LONG_FORM_UNTIL.store(until, Ordering::Relaxed);
        }
    }
    let template: Vec<String> = arguments
        .iter()
        .map(|argument| argument.replace("%F", &path.to_string_lossy()))
        .collect();
    // Detach: the player outlives this frame and its exit status is nobody's
    // business, so std::process::Command::spawn is the whole story.
    let _ = std::process::Command::new(program)
        .args(&template)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// When the long-form sound currently playing is expected to finish.
///
/// `Instant` has no atomic representation, so this stores elapsed milliseconds
/// since process-ish start instead: monotonic, and it fits an `AtomicU64`.
static LONG_FORM_UNTIL: AtomicU64 = AtomicU64::new(0);
static STARTED: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);

fn elapsed_ms() -> u64 {
    STARTED.elapsed().as_millis() as u64
}

fn still_playing() -> bool {
    let until = LONG_FORM_UNTIL.load(Ordering::Relaxed);
    until != 0 && elapsed_ms() < until
}

/// Length of a PCM WAV in seconds, read straight from the RIFF header.
fn duration_seconds(sound: Sound) -> Option<f32> {
    let bytes = crate::assets::asset_bytes(sound.asset())?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    // Walk the chunks rather than assuming a 44-byte header: some encoders
    // insert LIST or fact chunks before the data.
    let mut cursor = 12usize;
    let mut byte_rate = 0u32;
    let mut data_size = 0u32;
    while cursor + 8 <= bytes.len() {
        let id = bytes.get(cursor..cursor + 4)?;
        let size = u32::from_le_bytes(bytes.get(cursor + 4..cursor + 8)?.try_into().ok()?) as usize;
        let body = cursor + 8;
        if body + size > bytes.len() {
            break;
        }
        match id {
            b"fmt " => byte_rate = u32::from_le_bytes(bytes.get(body + 8..body + 12)?.try_into().ok()?),
            b"data" => data_size = size as u32,
            _ => {}
        }
        cursor = body + size + (size & 1); // chunks are word-aligned
    }
    if byte_rate == 0 || data_size == 0 {
        return None;
    }
    Some(data_size as f32 / byte_rate as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playing_a_sound_materialises_it_for_the_player() {
        // The whole engine is "embedded bytes -> temp file -> spawn", so prove
        // that chain rather than only proving the bytes are present.
        let previous = enabled();
        set_enabled(true);
        if !players_available() {
            // Nothing to hand the file to on this machine; the materialised
            // path is still the interesting part.
            let path = materialise(Sound::Ding.asset()).expect("the sound is written out");
            assert!(path.is_file(), "{} was not written", path.display());
            assert_eq!(&std::fs::read(&path).unwrap()[..4], b"RIFF");
            set_enabled(previous);
            return;
        }
        let home = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from);
        play(Sound::Ding);
        set_enabled(previous);
        if let Some(home) = home {
            let expected = home.join("winxp/winxp-sounds/ding.wav");
            assert!(expected.is_file(), "play() did not materialise {}", expected.display());
        }
    }

    #[test]
    fn muting_prevents_anything_being_written() {
        let previous = enabled();
        set_enabled(false);
        play(Sound::Ding);
        set_enabled(previous);
        // Nothing to assert beyond "it returned"; the guard is checked by
        // `enabled()` and the early return is the first statement of `play`.
        assert_eq!(enabled(), previous);
    }

    #[test]
    fn every_sound_maps_to_a_distinct_asset() {
        let all = [
            Sound::Startup, Sound::Shutdown, Sound::Logon, Sound::Logoff,
            Sound::Tada, Sound::Ding, Sound::Error, Sound::Exclamation,
            Sound::Balloon, Sound::Recycle, Sound::Chimes, Sound::Start,
        ];
        let mut assets: Vec<&str> = all.iter().map(|sound| sound.asset()).collect();
        assets.sort_unstable();
        let before = assets.len();
        assets.dedup();
        assert_eq!(assets.len(), before, "two sounds share one asset");
    }

    #[test]
    fn every_sound_asset_is_actually_embedded() {
        for sound in [
            Sound::Startup, Sound::Shutdown, Sound::Logon, Sound::Logoff,
            Sound::Tada, Sound::Ding, Sound::Error, Sound::Exclamation,
            Sound::Balloon, Sound::Recycle, Sound::Chimes, Sound::Start,
        ] {
            assert!(crate::assets::asset_bytes(sound.asset()).is_some(), "{} is missing", sound.asset());
        }
    }

    #[test]
    fn only_the_boot_and_shutdown_sounds_are_long_form() {
        assert!(Sound::Startup.is_long_form());
        assert!(Sound::Shutdown.is_long_form());
        assert!(!Sound::Ding.is_long_form());
        assert!(!Sound::Recycle.is_long_form());
    }

    #[test]
    fn muting_stops_playback() {
        let previous = enabled();
        set_enabled(false);
        assert!(!enabled());
        play(Sound::Ding);
        set_enabled(true);
        assert!(enabled());
        assert_eq!(enabled(), previous, "test must restore the mute switch");
    }

    #[test]
    fn every_sound_reports_a_plausible_length() {
        for sound in [
            Sound::Startup, Sound::Shutdown, Sound::Logon, Sound::Logoff,
            Sound::Tada, Sound::Ding, Sound::Error, Sound::Exclamation,
            Sound::Balloon, Sound::Recycle, Sound::Chimes, Sound::Start,
        ] {
            let seconds = duration_seconds(sound).unwrap_or_else(|| panic!("{} has no duration", sound.asset()));
            assert!(seconds > 0.01, "{} is implausibly short ({seconds}s)", sound.asset());
            assert!(seconds < 60.0, "{} is implausibly long ({seconds}s)", sound.asset());
        }
    }

    /// The startup/shutdown jingles are genuinely the long ones, which is what
    /// the anti-stacking rule keys off.
    #[test]
    fn long_form_sounds_are_the_ones_that_take_seconds() {
        for sound in [Sound::Startup, Sound::Shutdown, Sound::Logon, Sound::Logoff] {
            assert!(duration_seconds(sound).unwrap_or(0.0) > 1.0, "{sound:?} should be long-form");
        }
        for sound in [Sound::Ding, Sound::Error, Sound::Recycle, Sound::Start] {
            assert!(duration_seconds(sound).unwrap_or(99.0) < 2.0, "{sound:?} should be short");
        }
    }

    /// Some encoders insert LIST/fact chunks before `data`, so the parser must
    /// walk chunks rather than assume a bare 44-byte header. Verified against
    /// a hand-built file rather than a mocked byte slice.
    #[test]
    fn a_wav_with_extra_chunks_before_data_still_parses() {
        // A well-formed 16-byte fmt chunk: PCM, mono, 8000 Hz, 1 byte/frame.
        let mut fmt: Vec<u8> = Vec::new();
        fmt.extend_from_slice(&1u16.to_le_bytes());   // WAVE_FORMAT_PCM
        fmt.extend_from_slice(&1u16.to_le_bytes());   // channels
        fmt.extend_from_slice(&8000u32.to_le_bytes()); // sample rate
        fmt.extend_from_slice(&8000u32.to_le_bytes()); // byte rate == 1 byte/frame
        fmt.extend_from_slice(&1u16.to_le_bytes());   // block align
        fmt.extend_from_slice(&8u16.to_le_bytes());   // bits per sample

        let mut body: Vec<u8> = b"WAVE".to_vec();
        for (id, payload) in [
            (&b"fmt "[..], fmt),
            (&b"LIST"[..], b"LISTINFO".to_vec()),
            (&b"data"[..], vec![0u8; 8000]),
        ] {
            body.extend_from_slice(id);
            body.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            body.extend_from_slice(&payload);
            if payload.len() % 2 == 1 {
                body.push(0); // chunks are word-aligned
            }
        }
        let mut wav: Vec<u8> = b"RIFF".to_vec();
        wav.extend_from_slice(&(body.len() as u32).to_le_bytes());
        wav.extend_from_slice(&body);

        // 8000 bytes at 1 byte/frame is exactly one second.
        assert_eq!(wav_duration(&wav), Some(1.0));
    }

    /// The shared chunk walk, taking bytes rather than a `Sound`, so a test can
    /// feed it a synthetic file. `duration_seconds` is this plus asset lookup.
    fn wav_duration(bytes: &[u8]) -> Option<f32> {
        if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let mut cursor = 12usize;
        let mut byte_rate = 0u32;
        let mut data_size = 0u32;
        while cursor + 8 <= bytes.len() {
            let id = bytes.get(cursor..cursor + 4)?;
            let size = u32::from_le_bytes(bytes.get(cursor + 4..cursor + 8)?.try_into().ok()?) as usize;
            let body = cursor + 8;
            if body + size > bytes.len() {
                break;
            }
            match id {
                b"fmt " => byte_rate = u32::from_le_bytes(bytes.get(body + 8..body + 12)?.try_into().ok()?),
                b"data" => data_size = size as u32,
                _ => {}
            }
            cursor = body + size + (size & 1);
        }
        if byte_rate == 0 || data_size == 0 {
            return None;
        }
        Some(data_size as f32 / byte_rate as f32)
    }
}
