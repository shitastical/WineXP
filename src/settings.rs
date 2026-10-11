//! Shared platform data directory for persisted state
//! (`$XDG_DATA_HOME/winxp` or `~/.local/share/winxp`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The shell's persisted settings (`settings.json` next to the account store).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Set once the OOBE wizard has finished — Reborn XP's `oobepassed`, which
    /// decides between the OOBE wizard and the logon screen at boot.
    #[serde(default)]
    pub oobe_passed: bool,
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_dir());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path(), json);
        }
    }
}

/// The per-user application data directory.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("winxp")
}

fn path() -> PathBuf {
    data_dir().join("settings.json")
}
