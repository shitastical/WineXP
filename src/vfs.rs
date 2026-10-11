//! Filesystem bridge to the host Wine prefix.
//!
//! `C:\` is mapped onto the user's real Wine `drive_c` (`$WINEPREFIX/drive_c`,
//! defaulting to `~/.wine/drive_c`), so the Explorer and Command Prompt read and
//! write the actual host Wine drive with full access. Paths are handled in XP
//! form (`C:\WINDOWS`, `C:/Documents and Settings/User`); resolution is clamped
//! to the drive root so `..` can never escape it.
//!
//! The `join`/`dirname`/`basename`/`normalize_path` helpers mirror the
//! DiskManager (`dm.*`) that Reborn XP's `cmd.js` and `explorer.js` call.

use std::io;
use std::path::PathBuf;

/// A directory entry as returned by [`Vfs::list`].
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

/// The host drive backing `C:\`.
pub struct Vfs {
    root: PathBuf,
}

impl Vfs {
    /// Opens the Wine `drive_c`. `WINXP_DRIVE_C` overrides the location,
    /// otherwise `$WINEPREFIX/drive_c` (default `~/.wine/drive_c`) is used.
    pub fn new() -> Self {
        Self { root: drive_c() }
    }

    /// Maps an XP path to a host path inside the drive root.
    pub fn resolve(&self, xp: &str) -> PathBuf {
        let mut out = self.root.clone();
        let s = xp.replace('\\', "/");
        let rest = if s.len() >= 2 && s.as_bytes()[1] == b':' {
            &s[2..]
        } else {
            s.as_str()
        };
        for part in rest.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    if out != self.root {
                        out.pop();
                    }
                }
                name => out.push(name),
            }
        }
        out
    }

    pub fn exists(&self, xp: &str) -> bool {
        self.resolve(xp).exists()
    }

    pub fn is_dir(&self, xp: &str) -> bool {
        self.resolve(xp).is_dir()
    }

    pub fn is_file(&self, xp: &str) -> bool {
        self.resolve(xp).is_file()
    }

    /// Lists the contents of a directory. The `.`/`..` entries are omitted (the
    /// caller adds whatever it shows).
    pub fn list(&self, xp: &str) -> io::Result<Vec<Entry>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(self.resolve(xp))? {
            let entry = entry?;
            let md = entry.metadata()?;
            out.push(Entry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir: md.is_dir(),
                size: md.len(),
                modified: md.modified().ok(),
            });
        }
        out.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(out)
    }
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

fn drive_c() -> PathBuf {
    if let Some(path) = std::env::var_os("WINXP_DRIVE_C") {
        return PathBuf::from(path);
    }
    let prefix = std::env::var_os("WINEPREFIX")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".wine")));
    match prefix {
        Some(prefix) => prefix.join("drive_c"),
        None => PathBuf::from("/"),
    }
}

// ---- XP path helpers (mirroring `dm.*`) --------------------------------------

/// Whether `path` starts with a drive designator (`C:` / `C:\`).
pub fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

/// Collapses `.`/`..` and normalises separators, keeping a `C:` drive prefix.
pub fn normalize_path(path: &str) -> String {
    let s = path.replace('\\', "/");
    let (drive, rest) = if is_absolute(&s) {
        (s[..2].to_uppercase(), s[2..].to_string())
    } else {
        ("C:".to_owned(), s)
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            name => parts.push(name),
        }
    }
    format!("{drive}/{}", parts.join("/"))
}

/// Joins `base` and `rel` in XP path space.
pub fn join(base: &str, rel: &str) -> String {
    if is_absolute(rel) {
        return normalize_path(rel);
    }
    normalize_path(&format!("{}/{}", base.trim_end_matches('/'), rel.trim_start_matches('/')))
}

/// The parent of an XP path (`C:/a/b` -> `C:/a`, `C:/a` -> `C:/`).
pub fn dirname(path: &str) -> String {
    let n = normalize_path(path);
    match n.rfind('/') {
        Some(2) => n[..3].to_owned(),
        Some(i) => n[..i].to_owned(),
        None => "C:/".to_owned(),
    }
}

/// The final component of an XP path.
pub fn basename(path: &str) -> String {
    let n = normalize_path(path);
    match n.rfind('/') {
        Some(i) if i + 1 <= n.len() => n[i + 1..].to_owned(),
        _ => String::new(),
    }
}
