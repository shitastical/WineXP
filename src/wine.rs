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
        .env("WINEPREFIX", prefix())
        .spawn()
        .map(|_| ())
}

/// Open a document/folder through the real Wine prefix's file associations.
pub fn open(path: &Path) -> std::io::Result<()> {
    Command::new(wine_bin()).args(["start.exe", "/unix"]).arg(path)
        .env("WINEPREFIX", prefix()).spawn().map(|_| ())
}

/// Run a Windows command line without passing it through a host shell.
/// Quoted executable paths and arguments retain Windows backslashes.
pub fn run_command(command: &str) -> std::io::Result<()> {
    let args = split_command(command)?;
    let Some(program) = args.first() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Enter a program name."));
    };
    let mut process = Command::new(wine_bin());
    if program.eq_ignore_ascii_case("winecfg") {
        process.arg("winecfg.exe").args(&args[1..]);
    } else if args.len() == 1 && (program.starts_with("http://") || program.starts_with("https://") || program.ends_with('\\') || Path::new(program).is_dir()) {
        process.args(["start.exe", program]);
    } else {
        process.args(&args);
    }
    process.env("WINEPREFIX", prefix()).spawn().map(|_| ())
}

fn split_command(command: &str) -> std::io::Result<Vec<String>> {
    let mut args = Vec::new();
    let mut arg = String::new();
    let mut quoted = false;
    let mut started = false;
    for character in command.chars() {
        match character {
            '"' => { quoted = !quoted; started = true; }
            c if c.is_whitespace() && !quoted => {
                if started { args.push(std::mem::take(&mut arg)); started = false; }
            }
            c => { arg.push(c); started = true; }
        }
    }
    if quoted { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "The command contains an unmatched quote.")); }
    if started { args.push(arg); }
    Ok(args)
}

/// Execute a registry operation through Wine's own registry server.
/// The caller runs this off the UI thread; failure never reports a successful edit.
pub fn registry_command(args: &[String]) -> std::io::Result<String> {
    let mut command = vec!["reg.exe"];
    command.extend(args.iter().map(String::as_str));
    crate::sysinfo::run_with_timeout(&wine_bin(), &command, std::time::Duration::from_secs(10))
        .ok_or_else(|| std::io::Error::other("Wine registry operation failed or timed out. Check that Wine and this registry key are available."))
}

/// Open Wine's actual configuration tool, not the read-only prefix browser.
pub fn configure() -> std::io::Result<()> { run_command("winecfg") }

#[cfg(test)]
mod command_tests {
    use super::*;
    #[test]
    fn quoted_windows_paths_preserve_backslashes() {
        assert_eq!(split_command(r#""C:\Program Files\App\app.exe" /flag "two words""#).unwrap(), [r"C:\Program Files\App\app.exe", "/flag", "two words"]);
        assert!(split_command("\"unterminated").is_err());
        assert!(split_command("   ").unwrap().is_empty());
    }
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
