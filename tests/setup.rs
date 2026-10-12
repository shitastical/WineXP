//! Installation checks use isolated HOME/XDG/Wine paths, never real user state.
#![cfg(unix)]
use std::path::PathBuf;
use std::process::{Command, Output};

struct Sandbox { root: PathBuf }
impl Sandbox {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("winxp-setup-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(root.join("data/winxp")).unwrap();
        Self { root }
    }
    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        command.env("HOME", &self.root).env("XDG_DATA_HOME", self.root.join("data"))
            .env("WINEPREFIX", self.root.join("wine"))
            .env("WINXP_UPDATE_DIR", self.root.join("updates"))
            .env_remove("APPIMAGE").env("PATH", "/usr/bin:/bin")
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent-winxp-test-bus")
            .env("XDG_RUNTIME_DIR", self.root.join("runtime"));
        command
    }
    fn run(&self, args: &[&str]) -> Output { self.command(env!("CARGO_BIN_EXE_winxp")).args(args).output().unwrap() }
    fn app(&self) -> PathBuf { self.root.join("data/winxp/WineXP.AppImage") }
}
impl Drop for Sandbox { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); } }
fn successful(output: &Output) {
    assert!(output.status.success(), "stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}
#[test]
fn install_reinstall_and_launch_preserve_user_data() {
    let sandbox = Sandbox::new();
    let settings = sandbox.root.join("data/winxp/settings.json");
    let accounts = sandbox.root.join("data/winxp/accounts.json");
    std::fs::write(&settings, "{\"schema\":1,\"oobe_passed\":true}").unwrap();
    std::fs::write(&accounts, "existing accounts fixture").unwrap();
    successful(&sandbox.run(&["--install"]));
    assert!(sandbox.app().is_file());
    assert!(sandbox.root.join("data/applications/winxp.desktop").is_file());
    assert!(sandbox.root.join("data/icons/hicolor/256x256/apps/winxp.png").is_file());
    let installed = sandbox.command(sandbox.root.join(".local/bin/winxp")).arg("--version").output().unwrap();
    successful(&installed);
    successful(&sandbox.command(sandbox.app()).arg("--install").output().unwrap());
    assert_eq!(std::fs::read_to_string(settings).unwrap(), "{\"schema\":1,\"oobe_passed\":true}");
    assert_eq!(std::fs::read_to_string(accounts).unwrap(), "existing accounts fixture");
    assert!(String::from_utf8_lossy(&sandbox.run(&["--installation-status"]).stdout).contains("installed: true"));
    std::fs::remove_file(sandbox.app()).unwrap();
    assert!(String::from_utf8_lossy(&sandbox.run(&["--installation-status"]).stdout).contains("installed: false"));
}
#[test]
fn failed_self_check_does_not_replace_installation() {
    let sandbox = Sandbox::new();
    successful(&sandbox.run(&["--install"]));
    let before = std::fs::metadata(sandbox.app()).unwrap().len();
    let bad = sandbox.root.join("bad.AppImage");
    std::fs::write(&bad, b"not an executable").unwrap();
    let output = sandbox.command(env!("CARGO_BIN_EXE_winxp")).env("APPIMAGE", bad).arg("--install").output().unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::metadata(sandbox.app()).unwrap().len(), before);
    successful(&sandbox.command(sandbox.app()).arg("--version").output().unwrap());
}
#[test]
fn uninstall_keeps_settings_and_wine_files() {
    let sandbox = Sandbox::new();
    successful(&sandbox.run(&["--install"]));
    let settings = sandbox.root.join("data/winxp/settings.json");
    std::fs::write(&settings, "keep me").unwrap();
    std::fs::create_dir_all(sandbox.root.join("wine/drive_c")).unwrap();
    std::fs::write(sandbox.root.join("wine/drive_c/document.txt"), "keep document").unwrap();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("install.sh");
    successful(&sandbox.command("/bin/bash").arg(script).arg("--uninstall").output().unwrap());
    assert!(!sandbox.app().exists());
    assert_eq!(std::fs::read_to_string(settings).unwrap(), "keep me");
    assert_eq!(std::fs::read_to_string(sandbox.root.join("wine/drive_c/document.txt")).unwrap(), "keep document");
}
