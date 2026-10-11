//! Host Wine integration — the "WineXP" bridge.
//!
//! Where the simulators we port UI from keep a fake in-browser disk, this module
//! talks to the **real Wine prefix on the host**: its drives (`dosdevices`), the
//! Windows applications actually installed in `Program Files`, and launching
//! those applications as real Wine processes.
//!
//! Everything here is safe Rust: process spawning goes through
//! [`std::process::Command`], filesystem access through `std::fs`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The Wine prefix (`$WINEPREFIX`, else `~/.wine`).
pub fn prefix() -> PathBuf {
    std::env::var_os("WINEPREFIX")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".wine")))
        .unwrap_or_else(|| PathBuf::from(".wine"))
}

/// The `C:\` drive (`<prefix>/drive_c`).
pub fn drive_c() -> PathBuf {
    prefix().join("drive_c")
}

/// A logical drive, mirroring one entry in `<prefix>/dosdevices`.
#[derive(Clone, Debug)]
pub struct Drive {
    pub letter: char,
    pub target: PathBuf,
}

impl Drive {
    /// A short label like winecfg shows ("Local Disk", "CD Drive", "Floppy").
    pub fn label(&self) -> &'static str {
        let path = self.target.to_string_lossy();
        if path.ends_with("drive_c") {
            "Local Disk"
        } else if path.contains("drive_") {
            "Local Disk"
        } else if path == "/" {
            "Root"
        } else if path.starts_with("/dev/") || path.starts_with("/media/") {
            "CD Drive"
        } else if path.starts_with("/tmp") {
            "Local Disk"
        } else {
            "Network Drive"
        }
    }
}

/// The drives Wine exposes, read from `dosdevices` (e.g. `c:` -> `../drive_c`).
pub fn drives() -> Vec<Drive> {
    let dir = prefix().join("dosdevices");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut chars = name.chars();
        let (Some(letter), Some(':')) = (chars.next(), chars.next()) else {
            continue;
        };
        if !letter.is_ascii_alphabetic() || chars.next().is_some() {
            continue;
        }
        let target = std::fs::read_link(entry.path())
            .map(|link| normalize(&dir, &link))
            .unwrap_or_else(|_| entry.path());
        out.push(Drive { letter: letter.to_ascii_uppercase(), target });
    }
    out.sort_by_key(|drive| drive.letter);
    out.dedup_by_key(|drive| drive.letter);
    out
}

fn normalize(base: &Path, link: &Path) -> PathBuf {
    if link.is_absolute() {
        link.to_path_buf()
    } else {
        // dosdevices links are like `../drive_c`; collapse them for display.
        let joined = base.join(link);
        std::fs::canonicalize(&joined).unwrap_or(joined)
    }
}

/// Whether a `wine` binary is on `PATH`.
pub fn is_available() -> bool {
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path).any(|dir| {
                let candidate = dir.join("wine");
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}

/// The `wine` executable to spawn (`$WINE`, else `wine`).
fn wine_bin() -> String {
    std::env::var("WINE").unwrap_or_else(|_| "wine".to_owned())
}

/// A Windows application installed in the prefix.
#[derive(Clone, Debug)]
pub struct InstalledApp {
    pub name: String,
    pub exe: PathBuf,
}

/// Scans `Program Files` (both architectures) for installed applications —
/// the set winecfg's *Applications* tab would let you configure.
pub fn installed_apps() -> Vec<InstalledApp> {
    let mut out = Vec::new();
    for root in ["Program Files", "Program Files (x86)"] {
        let base = drive_c().join(root);
        let Ok(vendors) = std::fs::read_dir(&base) else {
            continue;
        };
        for vendor in vendors.flatten() {
            if !vendor.path().is_dir() {
                continue;
            }
            let Ok(apps) = std::fs::read_dir(vendor.path()) else {
                continue;
            };
            for app in apps.flatten() {
                if !app.path().is_dir() {
                    continue;
                }
                if let Some(exe) = main_exe(&app.path()) {
                    out.push(InstalledApp {
                        name: app.file_name().to_string_lossy().into_owned(),
                        exe,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    out
}

/// Picks a plausible "main" executable from an application folder: the largest
/// `.exe` directly inside it, ignoring uninstallers and setup programs.
fn main_exe(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<(u64, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("exe")) != Some(true) {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
        if stem.starts_with("unins") || stem.starts_with("setup") || stem == "install" {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if best.as_ref().map(|(b, _)| size > *b).unwrap_or(true) {
            best = Some((size, path));
        }
    }
    best.map(|(_, path)| path)
}

/// Launches a Windows executable with Wine, detached from this process.
pub fn launch(exe: &Path) -> std::io::Result<()> {
    Command::new(wine_bin())
        .arg(exe)
        .spawn()
        .map(|_| ())
}

/// The Windows version string reported in `<prefix>/system.reg`
/// (`"CurrentVersion"`/`"CurrentBuild"` under `HKLM\...\Windows`).
pub fn windows_version() -> Option<String> {
    let text = std::fs::read_to_string(prefix().join("system.reg")).ok()?;
    let value = |key: &str| -> Option<String> {
        text.lines()
            .find(|line| line.trim_start().starts_with(&format!("\"{key}\"")))
            .and_then(|line| line.split('"').nth(3))
            .map(str::to_owned)
    };
    let version = value("CurrentVersion")?;
    let build = value("CurrentBuildNumber").unwrap_or_default();
    if build.is_empty() {
        Some(format!("Windows {version}"))
    } else {
        Some(format!("Windows {version} (Build {build})"))
    }
}
