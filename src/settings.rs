//! Shared platform data directory for persisted state
//! (`$XDG_DATA_HOME/winxp` or `~/.local/share/winxp`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The schema version this build writes. A file from a *newer* WineXP is
/// refused rather than silently mis-read, and is never overwritten — that is
/// what stops a downgrade from eating a newer install's settings.
pub const SCHEMA: u32 = 1;

/// The shell's persisted settings (`settings.json` next to the account store).
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Written by [`SCHEMA`].
    #[serde(default = "default_schema")]
    pub schema: u32,
    /// Set once the OOBE wizard has finished — Reborn XP's `oobepassed`, which
    /// decides between the OOBE wizard and the logon screen at boot.
    #[serde(default)]
    pub oobe_passed: bool,
    /// Where "Windows Update" looks for releases. `https://…` or `file://…`.
    #[serde(default)]
    pub update_source: Option<String>,
    /// The version the shell last started with, so a changed build can show the
    /// "Installing Windows Updates…" scene (Reborn XP's `lastKnownAppVersion`).
    #[serde(default)]
    pub last_version: Option<String>,
    /// Set when the file on disk is newer than this build understands.
    #[serde(skip)]
    pub warning: Option<String>,
    /// When set, [`Settings::save`] refuses to write, so a newer schema is safe.
    #[serde(skip)]
    pub read_only: bool,
}

fn default_schema() -> u32 {
    SCHEMA
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            oobe_passed: false,
            update_source: None,
            last_version: None,
            warning: None,
            read_only: false,
        }
    }
}

impl Settings {
    /// Loads the settings, guarding the schema version.
    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string(path()) else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Self {
                warning: Some("settings.json is not valid JSON; defaults are in use".to_owned()),
                ..Self::default()
            };
        };
        let found = value.get("schema").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        if found > SCHEMA {
            return Self {
                warning: Some(format!(
                    "settings.json was written by a newer WineXP (schema {found}, this build knows {SCHEMA}); \
                     the file is left untouched and defaults are in use"
                )),
                read_only: true,
                ..Self::default()
            };
        }
        let mut settings: Self = serde_json::from_value(value).unwrap_or_default();
        settings.schema = SCHEMA;
        settings
    }

    /// Writes the settings, unless the file belongs to a newer build.
    pub fn save(&self) {
        if self.read_only {
            return;
        }
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
