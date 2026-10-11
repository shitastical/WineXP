//! Self-update ("Windows Update") for WineXP.
//!
//! The engine is deliberately paranoid. Every one of these rails is enforced in
//! code, not by convention, and each has a test in this file or in
//! `tests/update.rs`:
//!
//! 1. **Never downgrade.** A candidate whose version is not strictly greater
//!    than the installed one is refused — including an *equal* version, which is
//!    the classic way a rollback silently eats an upgrade.
//! 2. **HTTPS only.** `http://` (and anything unrecognised) is refused; locally
//!    served `file://` manifests and absolute paths are accepted for offline
//!    testing and air-gapped installs.
//! 3. **Integrity.** The artifact's byte length *and* SHA-256 must match the
//!    manifest, re-checked immediately before the swap, and a mismatch deletes
//!    the download.
//! 4. **Staged apply.** Nothing is replaced while the app is running: an update
//!    is downloaded, verified and recorded, then applied on the next launch.
//! 5. **Backup first.** The binary being replaced is copied into
//!    `updates/backups/` before it is touched, so `--rollback` can always undo.
//! 6. **Self-check + automatic rollback.** The staged binary must answer
//!    `--version` with the version the manifest promised; if the swap somehow
//!    produces a binary that cannot, the backup is restored immediately.
//! 7. **One at a time.** A pending marker refuses a second download.
//! 8. **Writability and free-space checks** before anything is written.
//! 9. **State schema version** in `settings.json`: a *newer* schema is refused
//!    rather than silently mis-read.
//! 10. **Audit log.** Every check, stage, apply, refusal and rollback is
//!     appended to `updates/update-log.json`.
//!
//! The default source is the project's **GitHub releases** (`github:owner/repo`):
//! the newest non-draft release is selected, and only a release whose tag is a
//! *higher* version than the installed build is ever offered. A release asset
//! without a SHA-256 digest (GitHub's `digest` field, or a published
//! `<asset>.sha256` sibling) is refused rather than installed unverified.
//!
//! Networking goes through the system `curl` (and the filesystem for `file://`),
//! so the crate keeps its zero-HTTP-dependency build.

use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::settings::data_dir;

/// The version this build compares with (`Cargo.toml`'s semver).
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The release channel this build belongs to.
pub const CHANNEL: &str = "pre-alpha";

/// The version users see — `Cargo.toml`'s version, zero-padded, with the channel.
/// [`the_display_version_matches_cargo`] keeps this in step with the manifest.
pub const DISPLAY_VERSION: &str = "v0.01.0 pre-alpha";

/// The repository WineXP checks for releases (`WINXP_UPDATE_REPO` overrides).
pub const GITHUB_REPO: &str = "shitastical/WineXP";

/// The release asset WineXP installs, by convention.
pub const GITHUB_ASSET: &str = "winxp-linux-x86_64";

/// Free space that must remain on top of the artifact size, in bytes.
const SLACK: u64 = 32 * 1024 * 1024;
/// How long `curl` may take, and how long a self-check may take.
const FETCH_TIMEOUT: Duration = Duration::from_secs(120);
const SELF_CHECK_TIMEOUT: Duration = Duration::from_secs(20);
/// Entries kept in the audit log.
const LOG_LIMIT: usize = 200;

// ---- versions ----------------------------------------------------------------

/// A three-part semantic version.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parses `1.2.3`, `v1.2.3` and `1.2.3-rc1`/`1.2.3+meta` (the pre-release and
    /// build suffixes are ignored, as the manifest carries exact artifacts).
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let core = text.split(['-', '+', ' ']).next()?;
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        Some(Self { major, minor, patch })
    }

    /// This build's version.
    pub fn current() -> Self {
        Self::parse(CURRENT_VERSION).expect("Cargo's version is a valid semver")
    }

    /// Whether `self` is strictly newer than `other`.
    pub fn is_newer_than(&self, other: &Self) -> bool {
        *self > *other
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ---- errors ------------------------------------------------------------------

/// Why an update was refused or failed. Every variant is a *safe* outcome: the
/// installed build is untouched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// No update source is configured.
    NoSource,
    /// The candidate is not strictly newer than what is installed.
    NotNewer { installed: Version, candidate: Version },
    /// The candidate requires a newer base build than this one.
    TooOld { installed: Version, required: Version },
    /// The source is not an HTTPS URL (or an explicit `file://` path).
    InsecureSource(String),
    /// Fetching the manifest or artifact failed.
    Fetch(String),
    /// The manifest is malformed or its fields are implausible.
    Manifest(String),
    /// The artifact does not match the manifest.
    Integrity(String),
    /// The installed binary cannot be replaced.
    NotWritable(String),
    /// There is not enough free space for the artifact.
    NotEnoughSpace { needed: u64, free: u64 },
    /// An update is already staged.
    AlreadyPending(String),
    /// Filesystem failure.
    Io(String),
    /// The rollback found nothing to restore.
    NothingToRollBack,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSource => write!(f, "no update source is configured"),
            Self::NotNewer { installed, candidate } => write!(
                f,
                "refusing {candidate}: it is not newer than the installed {installed}"
            ),
            Self::TooOld { installed, required } => write!(
                f,
                "refusing the update: it needs WineXP {required} or newer, this build is {installed}"
            ),
            Self::InsecureSource(source) => {
                write!(f, "refusing insecure update source {source}: only https:// and file:// are allowed")
            }
            Self::Fetch(why) => write!(f, "could not fetch the update: {why}"),
            Self::Manifest(why) => write!(f, "the update manifest is invalid: {why}"),
            Self::Integrity(why) => write!(f, "the update failed its integrity check: {why}"),
            Self::NotWritable(why) => write!(f, "the installed build cannot be replaced: {why}"),
            Self::NotEnoughSpace { needed, free } => write!(
                f,
                "not enough free space: {needed} bytes needed, {free} bytes available"
            ),
            Self::AlreadyPending(version) => write!(f, "version {version} is already staged and waiting"),
            Self::Io(why) => write!(f, "filesystem error: {why}"),
            Self::NothingToRollBack => write!(f, "no backup to roll back to"),
        }
    }
}

impl std::error::Error for UpdateError {}

/// The engine's result type.
pub type Result<T> = std::result::Result<T, UpdateError>;

fn io<T>(result: std::io::Result<T>) -> Result<T> {
    result.map_err(|e| UpdateError::Io(e.to_string()))
}

// ---- the manifest ------------------------------------------------------------

/// The document an update source serves (`manifest.json`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    /// e.g. `"0.2.0"`.
    pub version: String,
    /// Where the build can be downloaded from (https:// or file://).
    pub url: String,
    /// Lowercase hex SHA-256 of the artifact.
    pub sha256: String,
    /// Exact artifact length in bytes.
    pub size: u64,
    /// Release notes shown in the UI.
    #[serde(default)]
    pub notes: String,
    /// Optional: the oldest build allowed to install this update.
    #[serde(default)]
    pub min_supported: Option<String>,
}

impl Manifest {
    /// Validates the fields that can be checked without touching the network.
    pub fn validate(&self) -> Result<Version> {
        let version = Version::parse(&self.version)
            .ok_or_else(|| UpdateError::Manifest(format!("{:?} is not a version", self.version)))?;
        if self.sha256.len() != 64 || !self.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(UpdateError::Manifest(
                "sha256 must be 64 hexadecimal characters".to_owned(),
            ));
        }
        if self.size == 0 {
            return Err(UpdateError::Manifest("size must be greater than zero".to_owned()));
        }
        check_source(&self.url)?;
        Ok(version)
    }
}

// ---- sources -----------------------------------------------------------------

/// Where an update source points.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Source {
    /// `https://…`, fetched with `curl`.
    Https(String),
    /// `file://…` or an absolute path, read from the filesystem.
    File(PathBuf),
}

/// Classifies a source, refusing everything insecure.
pub fn classify(source: &str) -> Result<Source> {
    let source = source.trim();
    if let Some(rest) = source.strip_prefix("https://") {
        if rest.is_empty() {
            return Err(UpdateError::InsecureSource(source.to_owned()));
        }
        return Ok(Source::Https(format!("https://{rest}")));
    }
    if let Some(path) = source.strip_prefix("file://") {
        return Ok(Source::File(PathBuf::from(path)));
    }
    if source.starts_with("http://") {
        return Err(UpdateError::InsecureSource(source.to_owned()));
    }
    // A bare absolute path is treated as a local file source.
    if source.starts_with('/') {
        return Ok(Source::File(PathBuf::from(source)));
    }
    Err(UpdateError::InsecureSource(source.to_owned()))
}

fn check_source(source: &str) -> Result<Source> {
    classify(source)
}

/// Reads the manifest text from a source.
pub fn fetch_text(source: &str) -> Result<String> {
    match classify(source)? {
        Source::File(path) => {
            std::fs::read_to_string(&path).map_err(|e| UpdateError::Fetch(format!("{}: {e}", path.display())))
        }
        Source::Https(url) => {
            let output = Command::new("curl")
                .args(["-fsSL", "--proto", "=https", "--max-time"])
                .arg(FETCH_TIMEOUT.as_secs().to_string())
                .arg(&url)
                .output()
                .map_err(|e| UpdateError::Fetch(format!("could not run curl: {e}")))?;
            if !output.status.success() {
                return Err(UpdateError::Fetch(format!(
                    "curl exited with {} for {url}",
                    output.status
                )));
            }
            String::from_utf8(output.stdout).map_err(|e| UpdateError::Fetch(format!("response was not UTF-8: {e}")))
        }
    }
}

/// Reads `url` with extra request headers (used for the GitHub API).
fn fetch_text_with(url: &str, headers: &[String]) -> Result<String> {
    let mut command = Command::new("curl");
    command.args(["-fsSL", "--proto", "=https", "--max-time"]);
    command.arg(FETCH_TIMEOUT.as_secs().to_string());
    for header in headers {
        command.arg("-H").arg(header);
    }
    let output = command
        .arg(url)
        .output()
        .map_err(|e| UpdateError::Fetch(format!("could not run curl: {e}")))?;
    if !output.status.success() {
        return Err(UpdateError::Fetch(format!("curl exited with {} for {url}", output.status)));
    }
    String::from_utf8(output.stdout).map_err(|e| UpdateError::Fetch(format!("response was not UTF-8: {e}")))
}

// ---- GitHub releases ---------------------------------------------------------

/// One release asset as the GitHub API describes it.
#[derive(Clone, Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    /// GitHub reports `sha256:<hex>` here for uploaded assets.
    #[serde(default)]
    digest: Option<String>,
}

/// One release as the GitHub API describes it.
#[derive(Clone, Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

/// A release selected from the API, before its digest has been confirmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedRelease {
    pub version: Version,
    pub tag: String,
    pub notes: String,
    pub asset: String,
    pub url: String,
    pub size: u64,
    /// The `sha256:<hex>` GitHub reported, normalised to bare lowercase hex.
    pub digest: Option<String>,
    /// A published `<asset>.sha256` sibling, fetched when GitHub reports no digest.
    pub checksum_url: Option<String>,
}

/// The GitHub API endpoint listing a repository's releases, newest first.
pub fn releases_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/releases?per_page=30")
}

/// The repository WineXP checks: `WINXP_UPDATE_REPO`, else [`GITHUB_REPO`].
pub fn configured_repo() -> String {
    std::env::var("WINXP_UPDATE_REPO")
        .ok()
        .filter(|repo| !repo.trim().is_empty())
        .unwrap_or_else(|| GITHUB_REPO.to_owned())
}

/// The default update source: this project's GitHub releases.
pub fn default_source() -> String {
    format!("github:{}", configured_repo())
}

fn github_headers() -> Vec<String> {
    let mut headers = vec![
        "Accept: application/vnd.github+json".to_owned(),
        "User-Agent: WineXP".to_owned(),
        "X-GitHub-Api-Version: 2022-11-28".to_owned(),
    ];
    // An optional token lifts the anonymous rate limit (60 requests/hour).
    let token = std::env::var("WINXP_GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok());
    if let Some(token) = token.filter(|token| !token.trim().is_empty()) {
        headers.push(format!("Authorization: Bearer {}", token.trim()));
    }
    headers
}

/// Normalises a GitHub `digest` field to bare lowercase hex.
fn digest_hex(digest: &str) -> Option<String> {
    let (scheme, hex) = digest.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("sha256") {
        return None;
    }
    let hex = hex.trim().to_ascii_lowercase();
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex)
    } else {
        None
    }
}

/// Which asset of a release is the installable build.
fn pick_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    if let Some(exact) = assets.iter().find(|asset| asset.name == GITHUB_ASSET) {
        return Some(exact);
    }
    assets.iter().find(|asset| {
        let name = asset.name.to_ascii_lowercase();
        name.starts_with("winxp")
            && !name.ends_with(".sha256")
            && !name.ends_with(".json")
            && !name.ends_with(".sig")
            && !name.ends_with(".asc")
    })
}

fn checksum_sibling(assets: &[GhAsset], name: &str) -> Option<String> {
    let wanted = format!("{name}.sha256");
    assets
        .iter()
        .find(|asset| asset.name == wanted)
        .map(|asset| asset.browser_download_url.clone())
}

/// `min-winexp: 0.2.0` in the release body is the update's minimum base build.
fn min_supported_from(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim().eq_ignore_ascii_case("min-winexp") {
            let value = value.trim().trim_start_matches('v').to_owned();
            (!value.is_empty()).then_some(value)
        } else {
            None
        }
    })
}

fn notes_of(release: &GhRelease) -> String {
    let mut notes = String::new();
    if let Some(name) = release.name.as_deref().filter(|name| !name.trim().is_empty()) {
        notes.push_str(name.trim());
        notes.push('\n');
    }
    if release.prerelease {
        notes.push_str("(pre-release)\n");
    }
    notes.push_str(release.body.trim());
    notes.truncate(4000);
    notes
}

/// Picks the newest installable release out of a GitHub `/releases` response.
/// Drafts are never installable, and a release with no WineXP asset is skipped.
/// This is pure: it touches no network, so it is unit-tested against fixtures.
pub fn select_release(json: &str) -> Result<Option<SelectedRelease>> {
    let releases: Vec<GhRelease> = serde_json::from_str(json)
        .map_err(|e| UpdateError::Manifest(format!("GitHub returned invalid JSON: {e}")))?;
    let mut best: Option<SelectedRelease> = None;
    for release in releases {
        if release.draft {
            continue;
        }
        let Some(version) = Version::parse(&release.tag_name) else {
            continue;
        };
        let Some(asset) = pick_asset(&release.assets) else {
            continue;
        };
        let selected = SelectedRelease {
            version,
            tag: release.tag_name.clone(),
            notes: notes_of(&release),
            asset: asset.name.clone(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
            digest: asset.digest.as_deref().and_then(digest_hex),
            checksum_url: checksum_sibling(&release.assets, &asset.name),
        };
        if best.as_ref().map(|best| selected.version > best.version).unwrap_or(true) {
            best = Some(selected);
        }
    }
    Ok(best)
}

/// Reads a `<asset>.sha256` file (`<hex>  <name>`, or just the hex).
pub fn parse_checksum(text: &str, asset: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let hex = fields.next()?;
        let hex = hex.trim().to_ascii_lowercase();
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        match fields.next() {
            // Some checksum files name the file they describe; check when they do.
            Some(name) => (name == asset || name.trim_start_matches('*') == asset).then_some(hex),
            None => Some(hex),
        }
    })
}

/// The newest release on GitHub, before any version rail is applied. This is the
/// diagnostics view (`--github-latest`); installs go through [`check`].
pub fn latest_release(repo: &str) -> Result<Option<SelectedRelease>> {
    let json = fetch_text_with(&releases_url(repo), &github_headers())?;
    select_release(&json)
}

/// The newest release on GitHub, as a manifest. `Ok(None)` means the repository
/// has no release with an installable asset at all.
pub fn github_manifest(repo: &str) -> Result<Option<Manifest>> {
    let Some(selected) = latest_release(repo)? else {
        return Ok(None);
    };
    // Rail 3 for GitHub: an asset is only installable when its digest is known.
    let sha256 = match selected.digest.clone() {
        Some(digest) => digest,
        None => match selected.checksum_url.as_deref() {
            Some(url) => fetch_text(url)
                .ok()
                .and_then(|text| parse_checksum(&text, &selected.asset))
                .ok_or_else(|| {
                    UpdateError::Manifest(format!(
                        "release {} publishes no usable SHA-256 for {}; refusing to install it unverified",
                        selected.tag, selected.asset
                    ))
                })?,
            None => {
                return Err(UpdateError::Manifest(format!(
                    "release {} publishes no SHA-256 for {}; refusing to install it unverified",
                    selected.tag, selected.asset
                )))
            }
        },
    };
    let min_supported = min_supported_from(&selected.notes);
    let manifest = Manifest {
        version: selected.version.to_string(),
        url: selected.url,
        sha256,
        size: selected.size,
        notes: selected.notes,
        min_supported,
    };
    manifest.validate()?;
    Ok(Some(manifest))
}

/// Copies an artifact (or a local file) to `dest`.
pub fn fetch_file(url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        io(std::fs::create_dir_all(parent))?;
    }
    match classify(url)? {
        Source::File(path) => {
            io(std::fs::copy(&path, dest))?;
        }
        Source::Https(url) => {
            let status = Command::new("curl")
                .args(["-fsSL", "--proto", "=https", "--max-time"])
                .arg(FETCH_TIMEOUT.as_secs().to_string())
                .arg("-o")
                .arg(dest)
                .arg(&url)
                .status()
                .map_err(|e| UpdateError::Fetch(format!("could not run curl: {e}")))?;
            if !status.success() {
                let _ = std::fs::remove_file(dest);
                return Err(UpdateError::Fetch(format!("curl exited with {status} for {url}")));
            }
        }
    }
    Ok(())
}

// ---- paths -------------------------------------------------------------------

/// `$XDG_DATA_HOME/winxp/updates` (overridable with `WINXP_UPDATE_DIR`).
pub fn updates_dir() -> PathBuf {
    std::env::var_os("WINXP_UPDATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir().join("updates"))
}

fn staging_dir() -> PathBuf {
    updates_dir().join("staging")
}

fn backups_dir() -> PathBuf {
    updates_dir().join("backups")
}

fn pending_path() -> PathBuf {
    updates_dir().join("pending.json")
}

fn log_path() -> PathBuf {
    updates_dir().join("update-log.json")
}

// ---- the audit log -----------------------------------------------------------

/// One audited action.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub at: String,
    pub action: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub detail: String,
}

/// The audit log, newest first.
pub fn log_entries() -> Vec<LogEntry> {
    std::fs::read_to_string(log_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<LogEntry>>(&text).ok())
        .unwrap_or_default()
}

/// Appends one audited action.
pub fn audit(action: &str, version: Option<&str>, detail: &str) {
    let mut entries = log_entries();
    entries.insert(
        0,
        LogEntry {
            at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            action: action.to_owned(),
            version: version.map(str::to_owned),
            detail: detail.to_owned(),
        },
    );
    entries.truncate(LOG_LIMIT);
    if let Some(parent) = log_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = write_atomic(&log_path(), json.as_bytes());
    }
}

// ---- staging -----------------------------------------------------------------

/// A staged, verified update waiting for the next launch.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub version: String,
    pub sha256: String,
    pub size: u64,
    pub staged: PathBuf,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub staged_at: String,
}

/// The staged update, if any.
pub fn pending() -> Option<Pending> {
    std::fs::read_to_string(pending_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Pending>(&text).ok())
}

/// Applies the safety rails to a candidate. `Ok(None)` is never returned here:
/// a candidate that is not strictly newer is an [`UpdateError::NotNewer`].
pub fn evaluate(manifest: &Manifest) -> Result<Option<Manifest>> {
    let candidate = manifest.validate()?;
    let installed = Version::current();

    // Rail 1: never downgrade (and refuse an identical version).
    if !candidate.is_newer_than(&installed) {
        audit("check", Some(&manifest.version), "refused: not newer");
        return Err(UpdateError::NotNewer { installed, candidate });
    }
    // Rail 8b: refuse an update that needs a newer base than this build.
    if let Some(required) = manifest.min_supported.as_deref() {
        let required = Version::parse(required)
            .ok_or_else(|| UpdateError::Manifest(format!("{required:?} is not a version")))?;
        if installed < required {
            audit("check", Some(&manifest.version), "refused: base too old");
            return Err(UpdateError::TooOld { installed, required });
        }
    }
    audit("check", Some(&manifest.version), "offer accepted");
    Ok(Some(manifest.clone()))
}

/// Fetches the manifest and decides whether it may be installed.
/// `Ok(None)` means "already up to date"; every refusal is an `Err`.
///
/// A `github:owner/repo` source consults that repository's releases instead of a
/// manifest URL; there, "no release is newer" is a plain *up to date* rather
/// than an error, because it is the normal state of a current install.
pub fn check(source: &str) -> Result<Option<Manifest>> {
    if let Some(repo) = source.trim().strip_prefix("github:") {
        let Some(manifest) = github_manifest(repo)? else {
            return Ok(None);
        };
        return match evaluate(&manifest) {
            Err(UpdateError::NotNewer { installed, candidate }) => {
                audit(
                    "check",
                    Some(&manifest.version),
                    &format!("up to date: GitHub's newest is {candidate}, installed is {installed}"),
                );
                Ok(None)
            }
            other => other,
        };
    }
    let text = fetch_text(source)?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|e| UpdateError::Manifest(format!("not valid JSON: {e}")))?;
    evaluate(&manifest)
}

/// The "already up to date" case, for the UI's status line.
pub fn check_status(source: &str) -> std::result::Result<Option<Manifest>, UpdateError> {
    if source.trim().is_empty() {
        return Err(UpdateError::NoSource);
    }
    check(source)
}

/// Downloads, verifies and stages an update. Nothing installed is touched.
pub fn stage(manifest: &Manifest) -> Result<Pending> {
    let version = manifest.validate()?;
    // Rail 7: only one update at a time.
    if let Some(existing) = pending() {
        return Err(UpdateError::AlreadyPending(existing.version));
    }
    let target = current_exe()?;
    ensure_replaceable(&target)?;
    // Rail 8a: enough room for the artifact (and the backup we will make later).
    check_space(&target, manifest.size + SLACK)?;

    let staged = staging_dir().join(format!("winxp-{version}"));
    if staged.exists() {
        let _ = std::fs::remove_file(&staged);
    }
    fetch_file(&manifest.url, &staged)?;
    // Rail 3: size + SHA-256, and delete the download when it fails.
    if let Err(error) = verify_artifact(&staged, manifest) {
        let _ = std::fs::remove_file(&staged);
        audit("stage", Some(&manifest.version), &format!("rejected: {error}"));
        return Err(error);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        io(std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)))?;
    }

    let pending = Pending {
        version: manifest.version.clone(),
        sha256: manifest.sha256.clone(),
        size: manifest.size,
        staged,
        notes: manifest.notes.clone(),
        staged_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let json = serde_json::to_string_pretty(&pending).map_err(|e| UpdateError::Io(e.to_string()))?;
    write_atomic(&pending_path(), json.as_bytes())?;
    audit("stage", Some(&pending.version), "verified and staged");
    Ok(pending)
}

/// Checks the artifact's length and digest against the manifest.
pub fn verify_artifact(path: &Path, manifest: &Manifest) -> Result<()> {
    let meta = io(std::fs::metadata(path))?;
    if meta.len() != manifest.size {
        return Err(UpdateError::Integrity(format!(
            "size mismatch: manifest says {} bytes, artifact is {} bytes",
            manifest.size,
            meta.len()
        )));
    }
    // A build that is neither an ELF binary nor a script — an HTML error page
    // from a proxy, a truncated download, a text file someone renamed — is a
    // mismatch, not a build: artifacts here always become executables.
    let digest = sha256_file(path)?;
    if !digest.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(UpdateError::Integrity(format!(
            "SHA-256 mismatch: manifest says {}, artifact is {digest}",
            manifest.sha256
        )));
    }
    if !looks_executable(path)? {
        return Err(UpdateError::Integrity(
            "the artifact is not an executable (no ELF header and no `#!` line)".to_owned(),
        ));
    }
    Ok(())
}

/// Whether a file starts like something the shell or kernel can execute.
pub fn looks_executable(path: &Path) -> Result<bool> {
    let mut file = io(std::fs::File::open(path))?;
    let mut head = [0u8; 4];
    let read = io(file.read(&mut head))?;
    let head = &head[..read];
    Ok(head.starts_with(b"\x7fELF") || head.starts_with(b"#!"))
}

/// AppImage type-2 files carry the magic `AI\x02` at offset 8 (inside the ELF
/// header's padding), which is how we know to run the self-check with
/// `APPIMAGE_EXTRACT_AND_RUN=1` on machines without FUSE.
pub fn is_appimage(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 12];
    let Ok(read) = file.read(&mut head) else {
        return false;
    };
    head.len() >= 11 && read >= 11 && &head[..4] == b"\x7fELF" && &head[8..11] == b"AI\x02"
}

// ---- applying ----------------------------------------------------------------

/// What a launch did with a staged update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// Nothing staged.
    None,
    /// The swap succeeded.
    Updated { from: Version, to: Version },
    /// The swap was undone because the new build failed its self-check.
    RolledBack { from: Version, to: Version, reason: String },
}

/// Applies a staged update. Called once at startup, before the GUI exists, and
/// also reachable as `--apply-update`.
pub fn apply_pending() -> Result<Applied> {
    let Some(pending) = pending() else {
        return Ok(Applied::None);
    };
    let from = Version::current();
    let to = Version::parse(&pending.version)
        .ok_or_else(|| UpdateError::Manifest(format!("{:?} is not a version", pending.version)))?;

    // Rail 1, re-checked at the last moment: a staged update must still be newer
    // than the binary that is about to install it.
    if !to.is_newer_than(&from) {
        clear_pending();
        audit("apply", Some(&pending.version), "refused at apply time: not newer");
        return Err(UpdateError::NotNewer { installed: from, candidate: to });
    }

    // Rail 3, again: the artifact on disk must still match its manifest.
    let manifest = Manifest {
        version: pending.version.clone(),
        url: String::new(),
        sha256: pending.sha256.clone(),
        size: pending.size,
        notes: pending.notes.clone(),
        min_supported: None,
    };
    if let Err(error) = verify_artifact(&pending.staged, &manifest) {
        clear_pending();
        audit("apply", Some(&pending.version), &format!("rejected: {error}"));
        return Err(error);
    }

    // Rail 6a: the staged build must prove it runs before it is trusted.
    if !self_check(&pending.staged, &to) {
        let _ = std::fs::remove_file(&pending.staged);
        clear_pending();
        let detail = "the downloaded build failed its self-check (--version)";
        audit("apply", Some(&pending.version), detail);
        return Err(UpdateError::Integrity(detail.to_owned()));
    }

    let target = current_exe()?;
    ensure_replaceable(&target)?;

    // Rail 5: back up what we are about to replace.
    let backup = backups_dir().join(format!("winxp-{from}"));
    io(std::fs::create_dir_all(backups_dir()))?;
    io(std::fs::copy(&target, &backup))?;

    // Rail 4: swap via a same-directory rename, so it is atomic.
    let staged_next = target.with_extension("winexp-new");
    io(std::fs::copy(&pending.staged, &staged_next))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        io(std::fs::set_permissions(&staged_next, std::fs::Permissions::from_mode(0o755)))?;
    }
    io(std::fs::rename(&staged_next, &target))?;

    // Rail 6b: if the swap left a binary that cannot even report its version,
    // put the backup back.
    if !self_check(&target, &to) {
        io(std::fs::copy(&backup, &target))?;
        clear_pending();
        let detail = "the new build failed its self-check after the swap; the backup was restored";
        audit("rollback", Some(&pending.version), detail);
        return Ok(Applied::RolledBack { from, to, reason: detail.to_owned() });
    }

    clear_pending();
    audit("apply", Some(&pending.version), "applied successfully");
    Ok(Applied::Updated { from, to })
}

/// Restores the newest backup older than the running build.
pub fn rollback() -> Result<Version> {
    let target = current_exe()?;
    let current = Version::current();
    let mut candidates: Vec<(Version, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(backups_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(rest) = name.strip_prefix("winxp-") else {
                continue;
            };
            let Some(version) = Version::parse(rest) else {
                continue;
            };
            if version != current {
                candidates.push((version, entry.path()));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let Some((version, backup)) = candidates.into_iter().next() else {
        return Err(UpdateError::NothingToRollBack);
    };
    ensure_replaceable(&target)?;
    // Keep the build being replaced, so a rollback is itself undoable.
    let keep = backups_dir().join(format!("winxp-{current}"));
    if !keep.exists() {
        io(std::fs::create_dir_all(backups_dir()))?;
        io(std::fs::copy(&target, &keep))?;
    }
    let next = target.with_extension("winexp-rollback");
    io(std::fs::copy(&backup, &next))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        io(std::fs::set_permissions(&next, std::fs::Permissions::from_mode(0o755)))?;
    }
    io(std::fs::rename(&next, &target))?;
    audit("rollback", Some(&version.to_string()), "restored from backup");
    Ok(version)
}

/// Forgets the staged update (used by tests and by a manual cancel).
pub fn clear_pending() {
    let _ = std::fs::remove_file(pending_path());
}

// ---- helpers -----------------------------------------------------------------

/// This binary's path. Inside an AppImage the running executable lives in a
/// read-only squashfs mount, so `$APPIMAGE` — the single file the user actually
/// has — is the thing to replace and back up.
pub fn current_exe() -> Result<PathBuf> {
    if let Some(image) = std::env::var_os("APPIMAGE").filter(|image| !image.is_empty()) {
        return Ok(PathBuf::from(image));
    }
    std::env::current_exe().map_err(|e| UpdateError::NotWritable(e.to_string()))
}

/// Runs `<binary> --version` and checks it reports `expected`. A build that
/// crashes, prints nonsense or *hangs* is treated as broken rather than
/// trusted, and is killed so startup cannot hang with it.
pub fn self_check(binary: &Path, expected: &Version) -> bool {
    let mut command = Command::new(binary);
    command.arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    // An AppImage normally needs FUSE to mount; extracting is the portable way
    // to run one, and a self-check should not depend on a working FUSE.
    if is_appimage(binary) {
        command.env("APPIMAGE_EXTRACT_AND_RUN", "1");
    }
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    let deadline = Instant::now() + SELF_CHECK_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return false;
                }
                let mut text = String::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_string(&mut text);
                }
                return Version::parse(text.trim()).map(|v| v == *expected).unwrap_or(false);
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return false,
        }
    }
}

/// Rail 8: refuse to replace a binary we cannot write next to.
pub fn ensure_replaceable(target: &Path) -> Result<()> {
    let dir = target
        .parent()
        .ok_or_else(|| UpdateError::NotWritable("the binary has no parent directory".to_owned()))?;
    let probe = dir.join(".winexp-write-probe");
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => Err(UpdateError::NotWritable(format!("{} is not writable: {e}", dir.display()))),
    }
}

/// Rail 8a: `df` free space against the directory holding `target`.
pub fn free_space(dir: &Path) -> Option<u64> {
    let output = Command::new("df").arg("-Pk").arg(dir).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let field = line.split_whitespace().nth(3)?;
    field.parse::<u64>().ok().map(|kb| kb * 1024)
}

fn check_space(target: &Path, needed: u64) -> Result<()> {
    let Some(free) = target.parent().and_then(free_space) else {
        // If `df` is unavailable we do not block the update; the copy itself
        // will fail cleanly if the disk is actually full.
        return Ok(());
    };
    if free < needed {
        audit("stage", None, &format!("refused: needs {needed} bytes, {free} free"));
        return Err(UpdateError::NotEnoughSpace { needed, free });
    }
    Ok(())
}

/// Writes a file by way of a temporary sibling, so a crash cannot leave a
/// half-written manifest, pending marker or log.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        io(std::fs::create_dir_all(parent))?;
    }
    let temp = path.with_extension("tmp");
    {
        let mut file = io(std::fs::File::create(&temp))?;
        io(file.write_all(bytes))?;
        io(file.sync_all())?;
    }
    io(std::fs::rename(&temp, path))
}

// ---- SHA-256 (FIPS 180-4), so the crate needs no hashing dependency -----------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// A streaming SHA-256, so artifacts are hashed without being held in memory.
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffered: 0,
            length: 0,
        }
    }

    /// Absorbs more input.
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length += bytes.len() as u64;
        if self.buffered > 0 {
            let take = (64 - self.buffered).min(bytes.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&bytes[..take]);
            self.buffered += take;
            bytes = &bytes[take..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while bytes.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&bytes[..64]);
            self.compress(&block);
            bytes = &bytes[64..];
        }
        if !bytes.is_empty() {
            self.buffer[..bytes.len()].copy_from_slice(bytes);
            self.buffered = bytes.len();
        }
    }

    /// Finishes the digest as lowercase hex.
    pub fn finish(mut self) -> String {
        let bits = self.length * 8;
        self.update_raw(&[0x80]);
        while self.buffered != 56 {
            self.update_raw(&[0x00]);
        }
        self.update_raw(&bits.to_be_bytes());
        let mut out = String::with_capacity(64);
        for word in self.state {
            out.push_str(&format!("{word:08x}"));
        }
        out
    }

    /// Feeds padding bytes without disturbing the message length counter.
    fn update_raw(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.buffer[self.buffered] = byte;
            self.buffered += 1;
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

/// SHA-256 of a byte slice, lowercase hex. The engine itself streams files
/// ([`sha256_file`]); this slice form backs the digest tests.
#[cfg(test)]
pub fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finish()
}

/// SHA-256 of a file, read in chunks.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = io(std::fs::File::open(path))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = io(file.read(&mut buffer))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, size: u64) -> Manifest {
        Manifest {
            version: version.to_owned(),
            url: "file:///tmp/artifact".to_owned(),
            sha256: "a".repeat(64),
            size,
            notes: String::new(),
            min_supported: None,
        }
    }

    #[test]
    fn sha256_matches_the_nist_vectors() {
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Exactly one block boundary, and one byte past it.
        assert_eq!(
            sha256(&[b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
        // Cross-checked against `printf 'a%.0s' {1..65} | sha256sum`.
        assert_eq!(
            sha256(&[b'a'; 65]),
            "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0"
        );
    }

    #[test]
    fn sha256_streams_the_same_digest() {
        let mut hasher = Sha256::new();
        hasher.update(b"abcdbcdecdefdefgefghfghighijhijk");
        hasher.update(b"ijkljklmklmnlmnomnopnopq");
        assert_eq!(
            hasher.finish(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    /// A GitHub `/releases` response, shaped like the real API's.
    const RELEASES: &str = r#"[
      {
        "tag_name": "v0.3.0",
        "name": "WineXP 0.3.0",
        "body": "draft, must never install",
        "draft": true,
        "prerelease": false,
        "assets": [{
          "name": "winxp-linux-x86_64",
          "browser_download_url": "https://example.com/0.3.0",
          "size": 111,
          "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }]
      },
      {
        "tag_name": "v0.2.0",
        "name": "WineXP 0.2.0",
        "body": "min-winexp: 0.1.0\nSecond release.",
        "draft": false,
        "prerelease": true,
        "assets": [
          {
            "name": "winxp-linux-x86_64",
            "browser_download_url": "https://example.com/0.2.0",
            "size": 222,
            "digest": "SHA256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
          },
          {
            "name": "notes.json",
            "browser_download_url": "https://example.com/notes.json",
            "size": 5
          }
        ]
      },
      {
        "tag_name": "nightly",
        "body": "not a version",
        "draft": false,
        "prerelease": false,
        "assets": [{
          "name": "winxp-linux-x86_64",
          "browser_download_url": "https://example.com/nightly",
          "size": 1,
          "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        }]
      },
      {
        "tag_name": "v0.1.0",
        "name": "WineXP 0.1.0",
        "body": "First release.",
        "draft": false,
        "prerelease": false,
        "assets": [
          {
            "name": "winxp-linux-x86_64",
            "browser_download_url": "https://example.com/0.1.0",
            "size": 333,
            "digest": null
          },
          {
            "name": "winxp-linux-x86_64.sha256",
            "browser_download_url": "https://example.com/0.1.0.sha256",
            "size": 96
          }
        ]
      },
      {
        "tag_name": "v0.0.9",
        "name": "no asset at all",
        "body": "",
        "draft": false,
        "prerelease": false,
        "assets": [{
          "name": "source.tar.gz",
          "browser_download_url": "https://example.com/src",
          "size": 9
        }]
      }
    ]"#;

    #[test]
    fn github_releases_select_the_newest_installable_one() {
        let selected = select_release(RELEASES).unwrap().expect("a release is installable");
        // v0.3.0 is a draft, a plain `nightly` tag is not a version, and the
        // asset-less release is skipped — so the pre-release v0.2.0 wins.
        assert_eq!(selected.version, Version { major: 0, minor: 2, patch: 0 });
        assert_eq!(selected.tag, "v0.2.0");
        assert_eq!(selected.asset, "winxp-linux-x86_64");
        assert_eq!(selected.url, "https://example.com/0.2.0");
        assert_eq!(selected.size, 222);
        // GitHub reports `sha256:<hex>`; it is normalised to bare lowercase hex.
        assert_eq!(selected.digest, Some("b".repeat(64)));
        assert!(selected.notes.contains("Second release."));
        assert!(selected.notes.contains("(pre-release)"));
        assert_eq!(min_supported_from(&selected.notes).as_deref(), Some("0.1.0"));
    }

    #[test]
    fn a_release_without_a_digest_is_flagged_for_a_checksum_file() {
        let json = r#"[{
          "tag_name": "v0.1.0",
          "body": "",
          "draft": false,
          "assets": [
            {"name": "winxp-linux-x86_64", "browser_download_url": "https://example.com/a", "size": 5},
            {"name": "winxp-linux-x86_64.sha256", "browser_download_url": "https://example.com/a.sha256", "size": 96}
          ]
        }]"#;
        let selected = select_release(json).unwrap().unwrap();
        assert_eq!(selected.digest, None);
        assert_eq!(selected.checksum_url.as_deref(), Some("https://example.com/a.sha256"));
    }

    #[test]
    fn an_unparseable_or_assetless_release_list_selects_nothing() {
        assert_eq!(select_release("[]").unwrap(), None);
        let no_asset = r#"[{"tag_name": "v9.9.9", "body": "", "draft": false, "assets": []}]"#;
        assert_eq!(select_release(no_asset).unwrap(), None);
        // A release API response that is not JSON at all is an error, not a panic.
        assert!(matches!(select_release("<html>rate limited</html>"), Err(UpdateError::Manifest(_))));
    }

    #[test]
    fn digests_are_only_accepted_as_sha256() {
        assert_eq!(digest_hex(&format!("sha256:{}", "a".repeat(64))), Some("a".repeat(64)));
        assert_eq!(digest_hex("sha512:abcd"), None);
        assert_eq!(digest_hex("sha256:short"), None);
        assert_eq!(digest_hex("nonsense"), None);
    }

    #[test]
    fn published_checksum_files_are_parsed() {
        let hex = "d".repeat(64);
        assert_eq!(parse_checksum(&format!("{hex}  winxp-linux-x86_64\n"), "winxp-linux-x86_64").as_deref(), Some(hex.as_str()));
        assert_eq!(parse_checksum(&hex, "winxp-linux-x86_64").as_deref(), Some(hex.as_str()));
        assert_eq!(parse_checksum(&format!("{hex}  some-other-file"), "winxp-linux-x86_64"), None);
        assert_eq!(parse_checksum("not a checksum", "winxp-linux-x86_64"), None);
    }

    #[test]
    fn the_releases_endpoint_matches_github() {
        assert_eq!(
            releases_url("owner/repo"),
            "https://api.github.com/repos/owner/repo/releases?per_page=30"
        );
        assert!(default_source().starts_with("github:"));
        assert_eq!(default_source(), format!("github:{GITHUB_REPO}"));
        // `github:` is a recognised source, and it is not a plain fetch.
        assert!(matches!(classify("github:owner/repo"), Err(UpdateError::InsecureSource(_))));
    }

    #[test]
    fn the_display_version_matches_cargo() {
        // The version users see ("v0.01.0 pre-alpha") and the semver the updater
        // compares with must never drift apart.
        assert_eq!(Version::parse(DISPLAY_VERSION), Some(Version::current()));
        assert!(DISPLAY_VERSION.contains(CHANNEL));
        let mut manifest = manifest(&Version::current().to_string(), 1);
        manifest.version = DISPLAY_VERSION.to_owned();
        assert!(matches!(evaluate(&manifest), Err(UpdateError::NotNewer { .. })));
    }

    #[test]
    fn versions_parse_compare_and_order() {
        assert_eq!(Version::parse("1.2.3"), Some(Version { major: 1, minor: 2, patch: 3 }));
        assert_eq!(Version::parse("v0.2"), Some(Version { major: 0, minor: 2, patch: 0 }));
        assert_eq!(Version::parse("2.0.0-rc1"), Some(Version { major: 2, minor: 0, patch: 0 }));
        assert_eq!(Version::parse("not a version"), None);
        assert!(Version::parse("0.2.0").unwrap() > Version::parse("0.1.9").unwrap());
        assert!(Version::parse("1.0.0").unwrap() > Version::parse("0.99.99").unwrap());
        assert!(!Version::parse("0.1.0").unwrap().is_newer_than(&Version::current()));
    }

    #[test]
    fn only_https_and_file_sources_are_allowed() {
        assert_eq!(classify("https://example.com/m.json"), Ok(Source::Https("https://example.com/m.json".to_owned())));
        assert_eq!(classify("file:///tmp/m.json"), Ok(Source::File(PathBuf::from("/tmp/m.json"))));
        assert_eq!(classify("/tmp/m.json"), Ok(Source::File(PathBuf::from("/tmp/m.json"))));
        assert!(matches!(classify("http://example.com/m.json"), Err(UpdateError::InsecureSource(_))));
        assert!(matches!(classify("ftp://example.com/m.json"), Err(UpdateError::InsecureSource(_))));
        assert!(matches!(classify("https://"), Err(UpdateError::InsecureSource(_))));
    }

    #[test]
    fn manifests_are_validated() {
        assert!(manifest("0.2.0", 10).validate().is_ok());
        assert!(matches!(manifest("nope", 10).validate(), Err(UpdateError::Manifest(_))));
        assert!(matches!(manifest("0.2.0", 0).validate(), Err(UpdateError::Manifest(_))));
        let mut bad = manifest("0.2.0", 10);
        bad.sha256 = "xyz".to_owned();
        assert!(matches!(bad.validate(), Err(UpdateError::Manifest(_))));
        let mut insecure = manifest("0.2.0", 10);
        insecure.url = "http://example.com/winxp".to_owned();
        assert!(matches!(insecure.validate(), Err(UpdateError::InsecureSource(_))));
    }

    #[test]
    fn equal_and_older_versions_are_refused() {
        let installed = Version::current();
        for candidate in [installed.to_string(), "0.0.1".to_owned()] {
            let candidate = Version::parse(&candidate).unwrap();
            assert!(!candidate.is_newer_than(&installed), "{candidate} must not install over {installed}");
        }
        assert!(Version::parse("99.0.0").unwrap().is_newer_than(&installed));
    }

    #[test]
    fn artifact_verification_catches_size_and_digest_mismatches() {
        let dir = std::env::temp_dir().join("winxp-update-test-verify");
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("artifact");
        let script = b"#!/bin/sh\necho 9.9.9\n";
        std::fs::write(&artifact, script).unwrap();

        let mut good = manifest("9.9.9", script.len() as u64);
        good.sha256 = sha256(script);
        assert!(verify_artifact(&artifact, &good).is_ok());

        let wrong_size = manifest("9.9.9", 12);
        assert!(matches!(verify_artifact(&artifact, &wrong_size), Err(UpdateError::Integrity(_))));

        let mut wrong_digest = manifest("9.9.9", script.len() as u64);
        wrong_digest.sha256 = sha256(b"something else");
        assert!(matches!(verify_artifact(&artifact, &wrong_digest), Err(UpdateError::Integrity(_))));

        // A matching digest is not enough: an HTML error page is not a build.
        let page = b"<!DOCTYPE html><h1>404 Not Found</h1>".to_vec();
        std::fs::write(&artifact, &page).unwrap();
        let mut html = manifest("9.9.9", page.len() as u64);
        html.sha256 = sha256(&page);
        assert!(matches!(verify_artifact(&artifact, &html), Err(UpdateError::Integrity(_))));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_real_appimages_are_recognised() {
        let dir = std::env::temp_dir().join("winxp-update-test-appimage");
        let _ = std::fs::create_dir_all(&dir);

        // A minimal ELF header, with and without the AppImage magic at offset 8.
        let mut elf = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0];
        elf.extend_from_slice(b"\x02\x00\x00\x00");
        elf.extend_from_slice(&[0u8; 8]);
        let plain = dir.join("plain");
        std::fs::write(&plain, &elf).unwrap();
        assert!(looks_executable(&plain).unwrap());
        assert!(!is_appimage(&plain));

        elf[8..11].copy_from_slice(b"AI\x02");
        let appimage = dir.join("app.AppImage");
        std::fs::write(&appimage, &elf).unwrap();
        assert!(is_appimage(&appimage));

        let script = dir.join("script");
        std::fs::write(&script, b"#!/bin/sh\n").unwrap();
        assert!(looks_executable(&script).unwrap());
        assert!(!is_appimage(&script));
        assert!(!looks_executable(&dir).unwrap_or(false), "a directory is not an executable");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_appimage_environment_variable_wins_over_current_exe() {
        // Inside an AppImage the goal is to replace the .AppImage itself.
        let image = std::env::temp_dir().join("winxp-update-test.AppImage");
        std::env::set_var("APPIMAGE", &image);
        assert_eq!(current_exe().unwrap(), image);
        std::env::remove_var("APPIMAGE");
        assert_ne!(current_exe().unwrap(), image);
    }

    #[test]
    fn a_staged_update_blocks_a_second_one_and_can_be_cleared() {
        // Isolate the whole update tree for this test.
        let dir = std::env::temp_dir().join("winxp-update-test-pending");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("WINXP_UPDATE_DIR", &dir);
        std::fs::create_dir_all(&dir).unwrap();

        let staged = dir.join("staging").join("winxp-9.9.9");
        std::fs::create_dir_all(staged.parent().unwrap()).unwrap();
        std::fs::write(&staged, b"fake build").unwrap();
        let staged_update = Pending {
            version: "9.9.9".to_owned(),
            sha256: sha256(b"fake build"),
            size: 10,
            staged,
            notes: String::new(),
            staged_at: String::new(),
        };
        write_atomic(
            &pending_path(),
            serde_json::to_string(&staged_update).unwrap().as_bytes(),
        )
        .unwrap();
        assert_eq!(pending().map(|p| p.version), Some("9.9.9".to_owned()));

        let mut second = manifest("9.9.10", 1);
        second.url = "file:///tmp/whatever".to_owned();
        assert!(matches!(stage(&second), Err(UpdateError::AlreadyPending(_))));

        clear_pending();
        assert!(pending().is_none());
        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("WINXP_UPDATE_DIR");
    }
}
