//! Windows XP desktop, recreated natively in Rust with egui.
//!
//! The whole crate is `forbid(unsafe_code)`: memory safety is guaranteed by the
//! compiler, not by convention.
//!
//! Besides the desktop, `main` carries the headless half of the self-update
//! engine (`--check-update`, `--stage-update`, `--apply-update`, `--rollback`,
//! `--update-status`, `--update-log`, `--version`), and applies any staged
//! update before the shell starts. See `src/update.rs` for the safety rails.

#![forbid(unsafe_code)]

mod accounts;
mod app;
mod apps;
mod assets;
mod balloon;
mod fonts;
mod icons;
mod menu;
mod modal;
mod screens;
mod settings;
mod startmenu;
mod taskbar;
mod theme;
mod update;
mod vfs;
mod wine;
mod wm;

use update::{Applied, UpdateError};

fn main() -> eframe::Result {
    if let Some(code) = update_cli() {
        std::process::exit(code);
    }
    apply_staged_update();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("WineXP"),
        ..Default::default()
    };

    eframe::run_native(
        "WineXP",
        options,
        Box::new(|cc| Ok(Box::new(app::WinXp::new(cc)))),
    )
}

/// Handles the update sub-commands. Returns the process exit code when the
/// command was one of ours, so `main` can exit before opening a window.
fn update_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
            .cloned()
    };

    // The build's version — also what the update self-check runs.
    if has("--version") || has("-V") {
        println!("{}", update::DISPLAY_VERSION);
        return Some(0);
    }
    if has("--update-status") {
        println!("{}", update_status_json());
        return Some(0);
    }
    if has("--update-log") {
        for entry in update::log_entries() {
            println!(
                "{}  {:<8} {:<8} {}",
                entry.at,
                entry.action,
                entry.version.unwrap_or_default(),
                entry.detail
            );
        }
        return Some(0);
    }

    // What the project's GitHub releases currently offer (diagnostics).
    if has("--github-latest") {
        let repo = value("--github-latest").unwrap_or_else(update::configured_repo);
        return Some(match update::latest_release(&repo) {
            Ok(Some(release)) => {
                println!(
                    "{}  {}  {} bytes  {}",
                    release.tag,
                    release.asset,
                    release.size,
                    match &release.digest {
                        Some(digest) => format!("sha256 {digest}"),
                        None => "no digest published".to_owned(),
                    }
                );
                if release.version == update::Version::current() {
                    println!("this is the installed version");
                }
                0
            }
            Ok(None) => {
                println!("{} has no installable release", repo);
                0
            }
            Err(error) => {
                eprintln!("WineXP: {error}");
                3
            }
        });
    }

    let configured = || -> String {
        value("--check-update")
            .or_else(|| value("--stage-update"))
            .or_else(|| std::env::var("WINXP_UPDATE_SOURCE").ok())
            .or_else(|| settings::Settings::load().update_source)
            .unwrap_or_else(update::default_source)
    };

    if has("--check-update") {
        let source = configured();
        if source.is_empty() {
            eprintln!("WineXP: {}", UpdateError::NoSource);
            return Some(2);
        }
        return Some(match update::check_status(&source) {
            Ok(Some(manifest)) => {
                println!("update available: {} ({} bytes)", manifest.version, manifest.size);
                if !manifest.notes.is_empty() {
                    println!("{}", manifest.notes);
                }
                0
            }
            Ok(None) => {
                println!("WineXP {} is up to date", update::DISPLAY_VERSION);
                0
            }
            Err(error) => {
                eprintln!("WineXP: {error}");
                3
            }
        });
    }

    if has("--stage-update") {
        let source = configured();
        if source.is_empty() {
            eprintln!("WineXP: {}", UpdateError::NoSource);
            return Some(2);
        }
        let manifest = match update::check_status(&source) {
            Ok(Some(manifest)) => manifest,
            Ok(None) => {
                println!("WineXP {} is up to date", update::DISPLAY_VERSION);
                return Some(0);
            }
            Err(error) => {
                eprintln!("WineXP: {error}");
                return Some(3);
            }
        };
        return Some(match update::stage(&manifest) {
            Ok(pending) => {
                println!(
                    "staged {} -> {}; it will be applied on the next launch",
                    update::DISPLAY_VERSION,
                    pending.version
                );
                0
            }
            Err(error) => {
                eprintln!("WineXP: {error}");
                4
            }
        });
    }

    if has("--apply-update") {
        return Some(match update::apply_pending() {
            Ok(Applied::None) => {
                println!("nothing staged");
                5
            }
            Ok(Applied::Updated { from, to }) => {
                println!("applied {from} -> {to}");
                0
            }
            Ok(Applied::RolledBack { to, reason, .. }) => {
                eprintln!("WineXP: reverted {to}: {reason}");
                6
            }
            Err(error) => {
                eprintln!("WineXP: {error}");
                7
            }
        });
    }

    if has("--rollback") {
        return Some(match update::rollback() {
            Ok(version) => {
                println!("rolled back to {version}");
                0
            }
            Err(error) => {
                eprintln!("WineXP: {error}");
                7
            }
        });
    }

    None
}

/// Applies a staged update before the shell starts, reporting it on stderr.
fn apply_staged_update() {
    match update::apply_pending() {
        Ok(Applied::None) => {}
        Ok(Applied::Updated { from, to }) => {
            eprintln!("WineXP: applied the staged update {from} -> {to}; restart to use it");
        }
        Ok(Applied::RolledBack { to, reason, .. }) => {
            eprintln!("WineXP: the staged update to {to} was rolled back: {reason}");
        }
        Err(error) => eprintln!("WineXP: the staged update was not applied: {error}"),
    }
}

/// The `--update-status` document.
fn update_status_json() -> String {
    let pending = update::pending();
    let settings = settings::Settings::load();
    let document = serde_json::json!({
        "version": update::DISPLAY_VERSION,
        "channel": update::CHANNEL,
        "installed": update::CURRENT_VERSION,
        "pending": pending.as_ref().map(|p| p.version.clone()),
        "pending_notes": pending.as_ref().map(|p| p.notes.clone()),
        "source": settings.update_source.unwrap_or_else(update::default_source),
        "repo": update::configured_repo(),
        "asset": update::GITHUB_ASSET,
        "warning": settings.warning,
    });
    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_owned())
}
