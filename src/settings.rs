//! Shared platform data directory for persisted state
//! (`$XDG_DATA_HOME/winxp` or `~/.local/share/winxp`).

use std::path::PathBuf;

/// The per-user application data directory.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("winxp")
}
