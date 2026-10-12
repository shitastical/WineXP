//! Real per-user WineXP installation. Never formats disks or removes a Wine prefix.
//!
//! Install targets (all per-user, no root):
//! - `<data>/winxp/WineXP.AppImage` — the application (one file)
//! - `~/.local/bin/winxp` — launcher on PATH
//! - `<xdg-data>/applications/winxp.desktop` — Start-menu entry named "WineXP"
//! - `<xdg-data>/icons/hicolor/<size>/apps/winxp.png` + pixmaps fallback
//! - `<data>/winxp/installation.json` — receipt (`installed()`)
//!
//! Menu/icon cache refreshes are best-effort and never fail the install;
//! only an unverifiable app copy fails it.
use std::path::{Path, PathBuf};
use std::io;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Receipt { pub application: PathBuf, pub version: String, pub installed_at: String }

pub fn application_path() -> PathBuf { crate::settings::data_dir().join("WineXP.AppImage") }
fn receipt_path() -> PathBuf { crate::settings::data_dir().join("installation.json") }

pub fn installed() -> bool {
    std::fs::read(receipt_path()).ok().and_then(|bytes| serde_json::from_slice::<Receipt>(&bytes).ok()).is_some_and(|receipt| receipt.application.is_file())
}

fn source_path() -> io::Result<PathBuf> {
    std::env::var_os("APPIMAGE").map(PathBuf::from).filter(|path| path.is_file()).map(Ok).unwrap_or_else(std::env::current_exe)
}
fn shell_quote(path: &Path) -> String { format!("'{}'", path.to_string_lossy().replace('\'', "'\\''")) }

/// XDG data dir: absolute `$XDG_DATA_HOME`, else `~/.local/share`.
fn xdg_data(home: &Path) -> PathBuf {
    match std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir,
        _ => home.join(".local/share"),
    }
}

/// Desktop-entry `Exec=` value: absolute path, quoted per the freedesktop
/// spec (double quotes), so `$HOME` with spaces works on every distro/menu.
fn exec_line(launcher: &Path) -> String {
    let quoted = format!("\"{}\"", launcher.display().to_string().replace('"', "\\\""));
    format!("{quoted} %U")
}

fn desktop_entry(launcher: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=WineXP\nGenericName=Windows XP desktop\nComment=Windows XP desktop for Linux with Wine integration\nExec={}\nTryExec={}\nIcon=winxp\nTerminal=false\nCategories=System;\nKeywords=windows;xp;wine;desktop;\nStartupNotify=true\nStartupWMClass=WineXP\n",
        exec_line(launcher),
        launcher.display(),
    )
}

/// Best-effort menu/icon cache refresh. Never fails the install.
fn refresh_menu_caches(data: &Path, progress: &mut impl FnMut(&str)) {
    let applications = data.join("applications");
    let run = |program: &str, args: &[&str]| {
        std::process::Command::new(program).args(args).output().map(|_| ()).unwrap_or(())
    };
    if let Ok(output) = std::process::Command::new("desktop-file-validate")
        .arg(applications.join("winxp.desktop")).output()
    {
        if !output.status.success() {
            progress(&format!(
                "Desktop entry note: {}",
                String::from_utf8_lossy(&output.stdout).trim()
            ));
        }
    }
    run("update-desktop-database", &[&applications.to_string_lossy()]);
    run("gtk-update-icon-cache", &["-f", "-t", &data.join("icons/hicolor").to_string_lossy()]);
    let _ = std::process::Command::new("xdg-desktop-menu")
        .args(["install", "--novendor", &applications.join("winxp.desktop").to_string_lossy()])
        .output();
}

/// The WineXP application icon (the project's own XP logo) at every size a
/// menu, dock or file manager may ask for, plus the unscaled original.
const ICON_SIZES: &[u16] = &[16, 32, 48, 64, 128, 256, 512];

fn icon_bytes(size: u16) -> &'static [u8] {
    match size {
        16 => include_bytes!("../assets/winxp-icons/16x16.png"),
        32 => include_bytes!("../assets/winxp-icons/32x32.png"),
        48 => include_bytes!("../assets/winxp-icons/48x48.png"),
        64 => include_bytes!("../assets/winxp-icons/64x64.png"),
        128 => include_bytes!("../assets/winxp-icons/128x128.png"),
        512 => include_bytes!("../assets/winxp-icons/512x512.png"),
        _ => include_bytes!("../assets/winxp-icons/256x256.png"),
    }
}

/// Copies the WineXP icon to every location menus actually read, each at its
/// own resolution so no desktop has to downscale a 512px PNG itself.
fn install_icons(data: &Path, progress: &mut impl FnMut(&str)) {
    let write = |target: PathBuf, bytes: &[u8], progress: &mut dyn FnMut(&str)| {
        if std::fs::create_dir_all(target.parent().unwrap_or(data)).is_err() { return; }
        if std::fs::write(&target, bytes).is_err() {
            progress(&format!("Icon note: could not write {}", target.display()));
        }
    };
    for &size in ICON_SIZES {
        let rel = format!("icons/hicolor/{size}x{size}/apps/winxp.png");
        write(data.join(&rel), icon_bytes(size), progress);
    }
    // Menus that only read the flat locations still get the 256px icon.
    let flat = icon_bytes(256);
    write(data.join("pixmaps/winxp.png"), flat, progress);
    write(data.join("icons/winxp.png"), flat, progress);
    write(data.join("share/pixmaps/winxp.png"), flat, progress);
}

pub fn install(mut progress: impl FnMut(&str)) -> io::Result<Receipt> {
    let home = std::env::var_os("HOME").map(PathBuf::from).filter(|home| home.is_absolute())
        .ok_or_else(|| io::Error::other("HOME is not set. Cannot create a per-user launcher."))?;
    let data = crate::settings::data_dir();
    let base = data.parent().map(Path::to_path_buf).unwrap_or_else(|| xdg_data(&home));
    let source = source_path()?;
    let application = application_path();
    progress("Preparing per-user application directories…");
    std::fs::create_dir_all(&data)?;
    std::fs::create_dir_all(home.join(".local/bin"))?;
    std::fs::create_dir_all(base.join("applications"))?;
    std::fs::create_dir_all(base.join("icons/hicolor/256x256/apps"))?;
    progress("Copying the WineXP application…");
    let same = std::fs::canonicalize(&source).ok() == std::fs::canonicalize(&application).ok() && application.exists();
    if !same {
        let staging = data.join("WineXP.installing");
        std::fs::copy(&source, &staging)?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))?;
        }
        progress("Verifying the installed application…");
        let result = std::process::Command::new(&staging).arg("--version").env("APPIMAGE_EXTRACT_AND_RUN", "1").output()?;
        if !result.status.success() || String::from_utf8_lossy(&result.stdout).trim() != crate::update::display_version() {
            return Err(io::Error::other("The copied application did not pass its version self-check. Existing installation was not replaced."));
        }
        if application.exists() { std::fs::copy(&application, data.join("WineXP.AppImage.previous"))?; }
        std::fs::rename(&staging, &application)?;
    }
    progress("Creating the WineXP launcher and Start-menu entry…");
    let launcher = home.join(".local/bin/winxp");
    let script = format!("#!/bin/sh\n# WineXP per-user launcher; no privileged installation.\nexport APPIMAGE_EXTRACT_AND_RUN=1\nexec {} \"$@\"\n", shell_quote(&application));
    std::fs::write(&launcher, script)?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))?;
    }
    install_icons(&base, &mut progress);
    let desktop_path = base.join("applications/winxp.desktop");
    std::fs::write(&desktop_path, desktop_entry(&launcher))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&desktop_path, std::fs::Permissions::from_mode(0o644));
    }
    refresh_menu_caches(&base, &mut progress);
    // Confirm the entry the menu will actually read.
    let entry = std::fs::read_to_string(&desktop_path).unwrap_or_default();
    if !(entry.contains("Name=WineXP") && entry.contains("Icon=winxp")) {
        return Err(io::Error::other("The Start-menu entry was written but failed verification."));
    }
    progress("Recording installation and update configuration…");
    // User-level timer checks/stages verified releases; actual replacement still
    // uses the existing updater's backup/self-check/rollback path at launch.
    let unit_dir = home.join(".config/systemd/user");
    if std::process::Command::new("systemctl").args(["--user", "show-environment"]).output().is_ok_and(|o| o.status.success()) {
        std::fs::create_dir_all(&unit_dir)?;
        let executable = application.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"").replace('%', "%%");
        std::fs::write(unit_dir.join("winxp-update.service"), format!("[Unit]\nDescription=WineXP verified update check\n[Service]\nType=oneshot\nEnvironment=APPIMAGE_EXTRACT_AND_RUN=1\nExecStart=\"{executable}\" --stage-update\n"))?;
        std::fs::write(unit_dir.join("winxp-update.timer"), "[Unit]\nDescription=WineXP automatic updates\n[Timer]\nOnBootSec=5min\nOnUnitActiveSec=1d\nPersistent=true\n[Install]\nWantedBy=timers.target\n")?;
        let reload = std::process::Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        let enabled = std::process::Command::new("systemctl").args(["--user", "enable", "--now", "winxp-update.timer"]).status();
        if reload.is_ok_and(|s| s.success()) && enabled.is_ok_and(|s| s.success()) {
            progress("Configured the per-user daily verified update timer.");
        } else {
            progress("Automatic update timer could not start; use Windows Update to check for updates.");
        }
    } else {
        progress("User service manager unavailable; use Windows Update to check for updates.");
    }
    let receipt = Receipt { application, version: crate::update::display_version(), installed_at: chrono::Local::now().to_rfc3339() };
    let temporary = data.join("installation.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&receipt)?)?;
    std::fs::rename(temporary, receipt_path())?;
    progress("Installation complete. WineXP is in your applications menu. Your Linux files and Wine prefix were preserved.");
    Ok(receipt)
}

/// Removes only WineXP-owned integration: the app copy, launcher, menu entry,
/// icons and update timer. Never touches accounts, settings, documents,
/// backups or the Wine prefix.
pub fn uninstall(mut progress: impl FnMut(&str)) -> io::Result<()> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let data = crate::settings::data_dir();
    let base = data.parent().map(Path::to_path_buf).unwrap_or_else(|| xdg_data(&home));
    for path in [
        application_path(),
        data.join("WineXP.AppImage.previous"),
        data.join("installation.json"),
        home.join(".local/bin/winxp"),
        base.join("applications/winxp.desktop"),
        base.join("icons/winxp.png"),
        base.join("pixmaps/winxp.png"),
        base.join("share/pixmaps/winxp.png"),
    ] {
        let _ = std::fs::remove_file(&path);
    }
    for size in ICON_SIZES {
        let _ = std::fs::remove_file(base.join(format!("icons/hicolor/{size}x{size}/apps/winxp.png")));
    }
    for unit in ["winxp-update.timer", "winxp-update.service"] {
        let _ = std::process::Command::new("systemctl").args(["--user", "disable", "--now", unit]).output();
        let _ = std::fs::remove_file(home.join(format!(".config/systemd/user/{unit}")));
    }
    let _ = std::process::Command::new("systemctl").args(["--user", "daemon-reload"]).output();
    refresh_menu_caches(&base, &mut progress);
    progress("WineXP removed. Settings, accounts, documents, backups and Wine programs were preserved.");
    Ok(())
}

pub fn capabilities() -> Vec<String> {
    let mut result = vec![format!("Platform: {} / {}", std::env::consts::OS, std::env::consts::ARCH), format!("Install location: {}", application_path().display()), format!("Wine prefix: {}", crate::wine::prefix().display())];
    result.push(if crate::wine::is_available() { "Wine detected" } else { "Wine not found — Windows applications require Wine to be installed" }.to_owned());
    if std::env::var_os("DISPLAY").is_some() { result.push("X11/XWayland display detected".to_owned()); }
    else if std::env::var_os("WAYLAND_DISPLAY").is_some() { result.push("Wayland detected — browser currently requires XWayland".to_owned()); }
    result.push("Browser requires WebKitGTK 4.1 and Python GI/Cairo bindings".to_owned());
    result
}
