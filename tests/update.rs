//! End-to-end tests for the self-update rails.
//!
//! Every test works on a *copy* of the built binary inside a private temp
//! directory, with `WINXP_UPDATE_DIR` pointed at that directory, so the real
//! `target/…/winxp` is never staged over or replaced.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// `ETXTBSY` — exec'ing a file that is still open for writing.
fn libc_etxtbsy() -> i32 {
    26
}

/// A private sandbox: `app/winxp` is a copy of the real build.
struct Sandbox {
    root: PathBuf,
    app: PathBuf,
    updates: PathBuf,
}

/// Where the sandboxes live. `TMPDIR` can point at a volume with no space left
/// (as it does on the development host), so `/tmp` is preferred when usable.
///
/// The probe name is unique per caller: a shared probe file let concurrent
/// tests delete each other's file mid-flight, which made a test fall back to
/// the full volume and then fail to copy or spawn the binary — a flake that
/// moved between tests from run to run.
fn temp_root(tag: &str) -> PathBuf {
    let preferred = PathBuf::from("/tmp");
    let probe = preferred.join(format!(".winexp-update-it-probe-{tag}-{}", std::process::id()));
    if std::fs::write(&probe, b"probe").is_ok() {
        let _ = std::fs::remove_file(&probe);
        return preferred;
    }
    std::env::temp_dir()
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root = temp_root(name).join(format!("winexp-update-it-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let app_dir = root.join("app");
        let updates = root.join("updates");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::create_dir_all(&updates).unwrap();
        let app = app_dir.join("winxp");
        std::fs::copy(binary(), &app).unwrap();
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { root, app, updates }
    }

    /// Runs the sandboxed build with its own update directory.
    ///
    /// Several of these tests run concurrently and each stages its own copy of
    /// a ~35 MB binary. On Linux, exec'ing a file that another thread still has
    /// open for writing fails with `ETXTBSY` ("Text file busy"), which is what
    /// made this suite fail a different test on almost every run. Retrying for
    /// a moment lets the writer finish instead of failing the test.
    fn run(&self, args: &[&str]) -> Output {
        let mut last = None;
        for attempt in 0..50 {
            match Command::new(&self.app)
                .args(args)
                .env("WINXP_UPDATE_DIR", &self.updates)
                .env("XDG_DATA_HOME", self.root.join("data"))
                .output()
            {
                Ok(output) => return output,
                Err(error) => {
                    let busy = error.raw_os_error() == Some(libc_etxtbsy());
                    assert!(
                        busy || attempt == 49,
                        "the sandboxed build runs: {error}"
                    );
                    last = Some(error);
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        }
        panic!("the sandboxed build runs: {}", last.expect("a spawn error").to_string());
    }

    /// Writes an artifact plus a manifest for it and returns the manifest path.
    fn manifest(&self, version: &str, artifact: &Path, sha_override: Option<&str>) -> PathBuf {
        let bytes = std::fs::read(artifact).unwrap();
        let digest = match sha_override {
            Some(value) => value.to_owned(),
            None => sha256(artifact),
        };
        let manifest = serde_json::json!({
            "version": version,
            "url": format!("file://{}", artifact.display()),
            "sha256": digest,
            "size": bytes.len(),
            "notes": "test release",
        });
        let path = self.root.join(format!("manifest-{version}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        path
    }

    fn pending(&self) -> Option<serde_json::Value> {
        let text = std::fs::read_to_string(self.updates.join("pending.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn version_of(&self, path: &Path) -> String {
        let output = Command::new(path).arg("--version").output().unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    #[allow(dead_code)]
    fn root(&self) -> &Path {
        &self.root
    }

    /// The update tree this sandbox owns.
    fn updates(&self) -> &Path {
        &self.updates
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_winxp"))
}

/// The semver behind a version string the build prints, so `v0.01.0 pre-alpha`
/// and Cargo's `0.1.0` compare as the same version.
fn semver(text: &str) -> String {
    let text = text.trim();
    let text = text.strip_prefix('v').unwrap_or(text);
    let core = text.split([' ', '-', '+']).next().unwrap();
    let mut parts = core.split('.').map(|part| part.parse::<u32>().unwrap());
    format!(
        "{}.{}.{}",
        parts.next().unwrap(),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0)
    )
}

fn sha256(path: &Path) -> String {
    let output = Command::new("sha256sum").arg(path).output().expect("sha256sum runs");
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .expect("sha256sum prints a digest")
        .to_owned()
}

/// A stand-in for a downloaded build: it answers `--version` like the real one.
fn fake_build(dir: &Path, name: &str, version: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo {version}; fi\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_newer_build_is_staged_verified_and_applied() {
    let sandbox = Sandbox::new("apply");
    let artifact = fake_build(&sandbox.root, "winxp-9.9.9", "9.9.9");
    let manifest = sandbox.manifest("9.9.9", &artifact, None);

    // Staging must not touch the installed build yet.
    let before = std::fs::read(&sandbox.app).unwrap();
    let staged = sandbox.run(&["--stage-update", &format!("file://{}", manifest.display())]);
    assert!(staged.status.success(), "staging failed: {}", stderr(&staged));
    assert_eq!(std::fs::read(&sandbox.app).unwrap(), before, "the installed build must be untouched");
    assert_eq!(sandbox.pending().map(|p| p["version"].clone()), Some(serde_json::json!("9.9.9")));

    // Applying performs the swap, keeping a backup of the old build.
    let applied = sandbox.run(&["--apply-update"]);
    assert!(applied.status.success(), "apply failed: {}", stderr(&applied));
    assert_eq!(sandbox.version_of(&sandbox.app), "9.9.9");
    assert!(sandbox.pending().is_none(), "the pending marker is cleared");
    let backups = std::fs::read_dir(sandbox.updates.join("backups")).unwrap().count();
    assert_eq!(backups, 1, "the replaced build is backed up");
}

#[test]
fn an_older_or_equal_version_is_refused() {
    let sandbox = Sandbox::new("downgrade");
    let installed = env!("CARGO_PKG_VERSION");
    let artifact = fake_build(&sandbox.root, "winxp-old", "0.0.1");
    let older = sandbox.manifest("0.0.1", &artifact, None);
    let equal = sandbox.manifest(installed, &artifact, None);

    for manifest in [&older, &equal] {
        let output = sandbox.run(&["--check-update", &format!("file://{}", manifest.display())]);
        assert!(!output.status.success(), "{} must be refused", manifest.display());
        assert!(stderr(&output).contains("not newer"), "unhelpful refusal: {}", stderr(&output));
    }

    // …and staging it is refused too, leaving no pending marker behind.
    let staged = sandbox.run(&["--stage-update", &format!("file://{}", older.display())]);
    assert!(!staged.status.success());
    assert!(sandbox.pending().is_none());

    // The installed build is still the one that runs.
    assert_eq!(semver(&sandbox.version_of(&sandbox.app)), semver(installed));
    assert!(sandbox.updates().exists(), "{} must be the update dir", sandbox.root().display());
}

#[test]
fn an_artifact_that_does_not_match_its_checksum_is_refused() {
    let sandbox = Sandbox::new("checksum");
    let artifact = fake_build(&sandbox.root, "winxp-bad", "9.9.9");
    let manifest = sandbox.manifest("9.9.9", &artifact, Some(&"0".repeat(64)));

    let output = sandbox.run(&["--stage-update", &format!("file://{}", manifest.display())]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("integrity"), "expected an integrity refusal: {}", stderr(&output));
    assert!(sandbox.pending().is_none(), "nothing may be staged from a bad artifact");
    assert_eq!(semver(&sandbox.version_of(&sandbox.app)), semver(env!("CARGO_PKG_VERSION")));
    // The printed version is the human one, and it still normalises to the semver
    // the updater compares with.
    assert!(sandbox.version_of(&sandbox.app).contains("pre-alpha"));
}

#[test]
fn an_insecure_source_is_refused() {
    let sandbox = Sandbox::new("insecure");
    let output = sandbox.run(&["--check-update", "http://example.com/manifest.json"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("insecure"), "expected an insecure-source refusal: {}", stderr(&output));
}

#[test]
fn a_build_that_fails_its_self_check_is_never_installed() {
    let sandbox = Sandbox::new("selfcheck");
    // A "build" whose --version answers with something that is not a version.
    let artifact = sandbox.root.join("winxp-broken");
    std::fs::write(&artifact, "#!/bin/sh\necho not-a-version\n").unwrap();
    std::fs::set_permissions(&artifact, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manifest = sandbox.manifest("9.9.9", &artifact, None);

    let staged = sandbox.run(&["--stage-update", &format!("file://{}", manifest.display())]);
    assert!(staged.status.success(), "it stages, because the bytes do match: {}", stderr(&staged));

    let before = std::fs::read(&sandbox.app).unwrap();
    let applied = sandbox.run(&["--apply-update"]);
    assert!(!applied.status.success(), "a build that cannot report its version must not install");
    assert!(stderr(&applied).contains("self-check"), "expected a self-check failure: {}", stderr(&applied));
    assert_eq!(std::fs::read(&sandbox.app).unwrap(), before, "the installed build must survive");
    assert!(sandbox.pending().is_none());
}

#[test]
fn rollback_restores_the_previous_build() {
    let sandbox = Sandbox::new("rollback");
    // Pretend a previous version was backed up before an update.
    let backups = sandbox.updates.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let previous = fake_build(&backups, "winxp-0.0.9", "0.0.9");
    let previous_bytes = std::fs::read(&previous).unwrap();

    let output = sandbox.run(&["--rollback"]);
    assert!(output.status.success(), "rollback failed: {}", stderr(&output));
    assert_eq!(std::fs::read(&sandbox.app).unwrap(), previous_bytes, "the backup was restored");
    // The build it replaced is itself kept, so a rollback stays undoable.
    let kept = backups.join(format!("winxp-{}", env!("CARGO_PKG_VERSION")));
    assert!(kept.exists(), "the replaced build is kept for a second rollback");
}

#[test]
fn update_status_reports_the_installed_version() {
    let sandbox = Sandbox::new("status");
    let output = sandbox.run(&["--update-status"]);
    assert!(output.status.success());
    let status: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(status["installed"], serde_json::json!(env!("CARGO_PKG_VERSION")));
    assert_eq!(status["pending"], serde_json::Value::Null);
}
