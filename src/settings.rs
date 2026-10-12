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
    #[serde(default)]
    pub folders: FolderOptions,
    #[serde(default)]
    pub browser: BrowserSettings,
    /// Whether WineXP plays the Windows XP system sounds.
    #[serde(default = "default_sound")]
    pub sound: bool,
    /// Set when the file on disk is newer than this build understands.
    #[serde(skip)]
    pub warning: Option<String>,
    /// When set, [`Settings::save`] refuses to write, so a newer schema is safe.
    #[serde(skip)]
    pub read_only: bool,
}

/// Persisted browser preferences (WebKit is the engine; IE is the shell).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserSettings {
    pub homepage: String,
    pub favorites: Vec<String>,
    pub history: Vec<String>,
    pub javascript: bool,
    pub zoom: f64,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self { homepage: "https://www.google.com/".to_owned(), favorites: Vec::new(), history: Vec::new(), javascript: true, zoom: 1.0 }
    }
}

/// Explorer preferences shared with the Folder Options applet.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FolderOptions {
    pub show_hidden: bool,
    pub hide_extensions: bool,
    pub single_click: bool,
    pub show_tasks: bool,
    pub full_path_title: bool,
}

impl Default for FolderOptions {
    fn default() -> Self {
        Self { show_hidden: false, hide_extensions: true, single_click: false, show_tasks: true, full_path_title: false }
    }
}

fn default_schema() -> u32 {
    SCHEMA
}

fn default_sound() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            oobe_passed: false,
            update_source: None,
            last_version: None,
            folders: FolderOptions::default(),
            browser: BrowserSettings::default(),
            sound: true,
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
        if let Err(error) = self.try_save() {
            eprintln!("WineXP: could not save settings: {error}");
        }
    }

    /// Persist atomically and report failure to dialogs instead of pretending
    /// Apply succeeded on a read-only/full filesystem.
    pub fn try_save(&self) -> std::io::Result<()> {
        if self.read_only {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Settings belong to a newer WineXP and are read-only."));
        }
        std::fs::create_dir_all(data_dir())?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let temporary = data_dir().join(format!("settings-{}-{stamp}.tmp", std::process::id()));
        let result = (|| {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path())
        })();
        if result.is_err() { let _ = std::fs::remove_file(&temporary); }
        result
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
