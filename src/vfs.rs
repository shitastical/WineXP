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

/// Where the Recycle Bin lives inside `C:\`.
pub const TRASH_DIR: &str = ".winxp-trash";

/// The Recycle Bin in XP path form, the way Explorer refers to it.
pub const TRASH_XP: &str = "C:/.winxp-trash";

/// An item sitting in the Recycle Bin, as the user sees it.
#[derive(Clone, Debug)]
pub struct TrashEntry {
    /// The timestamped id used on disk; pass this to `restore`/`purge`.
    pub id: String,
    /// The name the item had before it was deleted.
    pub name: String,
    /// Where it came from, in XP form.
    pub original: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Storage id for a recycled item. The timestamp keeps ids unique so the bin can
/// hold several files with the same name, and the suffix keeps the original
/// name recoverable from disk alone.
fn trash_id(xp: &str, stamp: u128) -> String {
    format!("{stamp}-{}", basename(xp))
}

/// The name to show for an item whose original path is `xp`.
fn display_name(xp: &str) -> String {
    basename(&normalize_path(xp))
}

fn origin_file(id: &str) -> String {
    format!(".{id}.origin.json")
}

fn is_origin_file(name: &str) -> bool {
    name.starts_with('.') && name.ends_with(".origin.json")
}

/// A directory entry as returned by [`Vfs::list`].
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    /// Stable identity used for selection. Normally the same as `name`; in the
    /// Recycle Bin it is the on-disk id, because two deleted files can share a
    /// name and must still be separately selectable.
    pub id: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

/// The host drive backing `C:\`.
#[derive(Clone)]
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

    /// Create a folder without overwriting anything already present.
    pub fn create_folder(&self, directory: &str, name: &str) -> io::Result<()> {
        validate_name(name)?;
        std::fs::create_dir(self.resolve(&join(directory, name)))
    }

    /// Rename within the same directory; refuse to overwrite an existing item.
    pub fn rename(&self, path: &str, name: &str) -> io::Result<()> {
        validate_name(name)?;
        let destination = self.resolve(&join(&dirname(path), name));
        if destination.exists() { return Err(io::Error::new(io::ErrorKind::AlreadyExists, "An item with that name already exists.")); }
        std::fs::rename(self.resolve(path), destination)
    }

    /// Copy or move an item into a folder without replacing existing content.
    pub fn transfer(&self, source: &str, directory: &str, moving: bool) -> io::Result<()> {
        let source = normalize_path(source);
        let destination = join(directory, &basename(&source));
        if source == "C:/" || destination == source || destination.starts_with(&format!("{source}/")) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Cannot copy a folder into itself."));
        }
        let from = self.resolve(&source);
        let to = self.resolve(&destination);
        if to.exists() { return Err(io::Error::new(io::ErrorKind::AlreadyExists, "The destination already contains an item with this name.")); }
        if moving { std::fs::rename(from, to) } else { copy_tree(&from, &to) }
    }

    /// Move into WineXP's Recycle Bin, recording the original XP path for restore.
    ///
    /// Items are stored under a timestamped id so two files of the same name can
    /// coexist in the bin; [`Vfs::list_trash`] maps those ids back to the names
    /// the user recognises.
    pub fn recycle(&self, path: &str) -> io::Result<()> {
        let normalized = normalize_path(path);
        if normalized == "C:/" || normalized.starts_with("C:/.winxp-trash") {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "This item cannot be recycled."));
        }
        let trash = self.root.join(TRASH_DIR);
        std::fs::create_dir_all(&trash)?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
        let name = trash_id(&normalized, stamp);
        let destination = trash.join(&name);
        let metadata = trash.join(origin_file(&name));
        let json = serde_json::to_vec(&serde_json::json!({ "original": normalized, "deleted_at": stamp.to_string() }))?;
        std::fs::write(&metadata, json)?;
        if let Err(error) = std::fs::rename(self.resolve(path), destination) {
            let _ = std::fs::remove_file(metadata);
            return Err(error);
        }
        Ok(())
    }

    /// The Recycle Bin, as the user sees it: original names, original locations
    /// and when each item went in. Entries whose bookkeeping file is missing
    /// (hand-edited, or a partially emptied bin) are still listed, just with an
    /// unknown origin.
    pub fn list_trash(&self) -> io::Result<Vec<TrashEntry>> {
        let trash = self.root.join(TRASH_DIR);
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&trash)? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if is_origin_file(&id) {
                continue;
            }
            let metadata = entry.metadata().ok();
            let origin = std::fs::read(trash.join(origin_file(&id)))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .and_then(|document| document.get("original").and_then(|value| value.as_str()).map(str::to_owned));
            out.push(TrashEntry {
                name: origin.as_deref().map(display_name).unwrap_or_else(|| id.clone()),
                id,
                original: origin.unwrap_or_default(),
                is_dir: metadata.as_ref().is_some_and(|md| md.is_dir()),
                size: metadata.as_ref().map(|md| md.len()).unwrap_or(0),
            });
        }
        out.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(out)
    }

    /// Restore a recycled item to its original location, never replacing a file.
    pub fn restore(&self, id: &str) -> io::Result<()> {
        validate_name(id)?;
        let trash = self.root.join(TRASH_DIR);
        let metadata = trash.join(origin_file(id));
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&metadata)?)?;
        let original = document.get("original").and_then(|v| v.as_str()).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Original location is missing."))?;
        let destination = self.resolve(original);
        if destination.exists() { return Err(io::Error::new(io::ErrorKind::AlreadyExists, "The original location already contains an item with this name.")); }
        let parent = destination.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid destination."))?;
        std::fs::create_dir_all(parent)?;
        std::fs::rename(trash.join(id), destination)?;
        std::fs::remove_file(metadata)
    }

    /// Permanently delete one item from the Recycle Bin, as Windows' own
    /// "Delete" does once an item is already in the bin.
    pub fn purge(&self, id: &str) -> io::Result<()> {
        validate_name(id)?;
        let trash = self.root.join(TRASH_DIR);
        let target = trash.join(id);
        let removed = if target.is_dir() { std::fs::remove_dir_all(&target) } else { std::fs::remove_file(&target) };
        // The bookkeeping file is disposable: clearing it must not make an
        // otherwise successful permanent delete look like a failure.
        let _ = std::fs::remove_file(trash.join(origin_file(id)));
        removed
    }

    /// Permanently delete everything in the Recycle Bin. Returns how many items
    /// were removed.
    pub fn empty_trash(&self) -> io::Result<usize> {
        let items = self.list_trash()?;
        let mut removed = 0;
        for item in &items {
            if self.purge(&item.id).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Lists the contents of a directory. The `.`/`..` entries are omitted (the
    /// caller adds whatever it shows).
    pub fn list(&self, xp: &str) -> io::Result<Vec<Entry>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(self.resolve(xp))? {
            let entry = entry?;
            let md = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            out.push(Entry {
                id: name.clone(),
                name,
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

fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(from)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "Copying symbolic links is not supported."));
    }
    if metadata.is_dir() {
        std::fs::create_dir(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        let mut source = std::fs::File::open(from)?;
        let mut destination = std::fs::OpenOptions::new().write(true).create_new(true).open(to)?;
        io::copy(&mut source, &mut destination)?;
        std::fs::set_permissions(to, metadata.permissions())?;
    }
    Ok(())
}

#[cfg(test)]
mod file_operation_tests {
    use super::*;
    #[test]
    fn create_rename_copy_recycle_restore() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("winxp-vfs-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let vfs = Vfs { root: root.clone() };
        vfs.create_folder("C:/", "Documents").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/note.txt"), "hello").unwrap();
        vfs.rename("C:/Documents/note.txt", "renamed.txt").unwrap();
        vfs.create_folder("C:/", "Copies").unwrap();
        vfs.transfer("C:/Documents/renamed.txt", "C:/Copies", false).unwrap();
        assert_eq!(std::fs::read_to_string(vfs.resolve("C:/Copies/renamed.txt")).unwrap(), "hello");
        assert!(vfs.transfer("C:/Documents/renamed.txt", "C:/Copies", false).is_err());
        assert!(vfs.transfer("C:/Documents", "C:/Documents", false).is_err());
        vfs.recycle("C:/Documents/renamed.txt").unwrap();
        assert!(!vfs.exists("C:/Documents/renamed.txt"));
        let bin = vfs.list_trash().unwrap();
        assert_eq!(bin.len(), 1);
        // The bin shows the name the user recognises, not the storage id.
        assert_eq!(bin[0].name, "renamed.txt");
        assert_eq!(bin[0].original, "C:/Documents/renamed.txt");
        vfs.restore(&bin[0].id).unwrap();
        assert_eq!(std::fs::read_to_string(vfs.resolve("C:/Documents/renamed.txt")).unwrap(), "hello");
        assert!(vfs.list_trash().unwrap().is_empty());
        assert!(vfs.create_folder("C:/", "../escape").is_err());
        assert!(vfs.create_folder("C:/", "CON.txt").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    fn scratch() -> Vfs {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("winxp-trash-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        Vfs { root }
    }

    #[test]
    fn recycling_the_same_name_twice_keeps_both_items() {
        let vfs = scratch();
        vfs.create_folder("C:/", "Documents").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/note.txt"), "first").unwrap();
        vfs.recycle("C:/Documents/note.txt").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/note.txt"), "second").unwrap();
        vfs.recycle("C:/Documents/note.txt").unwrap();

        let bin = vfs.list_trash().unwrap();
        assert_eq!(bin.len(), 2, "both copies must survive in the bin");
        assert!(bin.iter().all(|item| item.name == "note.txt"));
        assert_ne!(bin[0].id, bin[1].id);

        // Restoring one must not touch the other.
        vfs.restore(&bin[0].id).unwrap();
        assert_eq!(std::fs::read_to_string(vfs.resolve("C:/Documents/note.txt")).unwrap(), "second");
        assert_eq!(vfs.list_trash().unwrap().len(), 1);
        std::fs::remove_dir_all(&vfs.root).unwrap();
    }

    #[test]
    fn deleting_inside_the_bin_is_permanent_and_emptying_clears_it() {
        let vfs = scratch();
        vfs.create_folder("C:/", "Documents").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/a.txt"), "a").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/b.txt"), "b").unwrap();
        vfs.recycle("C:/Documents/a.txt").unwrap();
        vfs.recycle("C:/Documents/b.txt").unwrap();

        let bin = vfs.list_trash().unwrap();
        assert_eq!(bin.len(), 2);
        let purged = &bin[0].id;
        let survivor = bin[1].id.clone();
        vfs.purge(purged).unwrap();
        assert_eq!(vfs.list_trash().unwrap().len(), 1);

        // A purged item leaves nothing behind, not even its bookkeeping file.
        let mut remaining: Vec<String> = std::fs::read_dir(vfs.root.join(TRASH_DIR)).unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        remaining.sort();
        let mut expected = vec![survivor.clone(), origin_file(&survivor)];
        expected.sort();
        assert_eq!(remaining, expected, "purge left {remaining:?}");

        assert_eq!(vfs.empty_trash().unwrap(), 1);
        assert!(vfs.list_trash().unwrap().is_empty());
        assert!(std::fs::read_dir(vfs.root.join(TRASH_DIR)).unwrap().next().is_none(), "emptying must leave the bin itself, and nothing in it");
        std::fs::remove_dir_all(&vfs.root).unwrap();
    }

    #[test]
    fn restoring_never_overwrites_a_file_that_reappeared() {
        let vfs = scratch();
        vfs.create_folder("C:/", "Documents").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/note.txt"), "old").unwrap();
        vfs.recycle("C:/Documents/note.txt").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/note.txt"), "new").unwrap();

        let bin = vfs.list_trash().unwrap();
        assert!(vfs.restore(&bin[0].id).is_err(), "restore must refuse to clobber");
        assert_eq!(std::fs::read_to_string(vfs.resolve("C:/Documents/note.txt")).unwrap(), "new");
        assert_eq!(vfs.list_trash().unwrap().len(), 1, "the item stays in the bin");
        std::fs::remove_dir_all(&vfs.root).unwrap();
    }

    #[test]
    fn the_bin_itself_can_never_be_recycled() {
        let vfs = scratch();
        assert!(vfs.recycle("C:/").is_err());
        assert!(vfs.recycle("C:/.winxp-trash").is_err());
        std::fs::remove_dir_all(&vfs.root).unwrap();
    }

    #[test]
    fn trash_listing_never_exposes_bookkeeping_files() {
        let vfs = scratch();
        vfs.create_folder("C:/", "Documents").unwrap();
        std::fs::write(vfs.resolve("C:/Documents/note.txt"), "x").unwrap();
        vfs.recycle("C:/Documents/note.txt").unwrap();
        for item in vfs.list_trash().unwrap() {
            assert!(!is_origin_file(&item.id), "origin file leaked into the listing");
        }
        std::fs::remove_dir_all(&vfs.root).unwrap();
    }
}

fn validate_name(name: &str) -> io::Result<()> {
    let reserved = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty() || name == "." || name == ".." || name.ends_with([' ', '.']) || name.chars().any(|c| c.is_control() || "\\/<>:\"|?*".contains(c))
        || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (reserved.len() == 4 && (reserved.starts_with("COM") || reserved.starts_with("LPT")) && matches!(reserved.as_bytes()[3], b'1'..=b'9')) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "A file name cannot contain \\ / : * ? \" < > | or use a reserved device name."));
    }
    Ok(())
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
