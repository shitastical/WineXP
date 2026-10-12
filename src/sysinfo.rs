//! Real host system information, gathered through a chain of fallbacks.
//!
//! Windows XP's System Properties (`sysdm.cpl`) and System Information
//! (`msinfo32.exe`) report the machine's real hardware. This module fills those
//! fields from whatever the host exposes, trying, in order:
//!
//! 1. `fastfetch` (structured, richest single source)
//! 2. `neofetch`
//! 3. `/proc` + `/sys` (procfs/sysfs — always available on Linux)
//! 4. `lscpu`, `lspci`, `uname` as further fallbacks
//!
//! Each source only fills fields the previous ones left empty, so a bare
//! machine with no fetch tool still reports a complete picture from procfs.
//! Everything here is safe Rust: no shell strings are built from user input.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Detection runs off the UI thread; callers keep rendering while it completes.
pub fn detect_async() -> Receiver<SystemInfo> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(SystemInfo::detect());
    });
    receiver
}

/// The detected host, with every field optional so partial data still renders.
#[derive(Clone, Default)]
pub struct SystemInfo {
    /// Which source produced the values ("fastfetch", "neofetch", "procfs").
    pub source: Option<String>,
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub kernel: Option<String>,
    pub cpu_name: Option<String>,
    pub cpu_mhz: Option<f64>,
    /// Logical processors, not physical cores.
    pub cpu_cores: Option<u32>,
    pub architecture: Option<String>,
    pub cpu_vendor: Option<String>,
    pub cpu_family: Option<String>,
    pub cpu_model: Option<String>,
    pub cpu_stepping: Option<String>,
    pub swap_total_kb: Option<u64>,
    pub swap_free_kb: Option<u64>,
    pub gpus: Vec<String>,
    pub ram_total_kb: Option<u64>,
    pub ram_used_kb: Option<u64>,
    pub sys_vendor: Option<String>,
    pub product_name: Option<String>,
    pub bios: Option<String>,
    pub uptime: Option<String>,
    pub disks: Vec<String>,
}

impl SystemInfo {
    /// Detect the host using every available method, in priority order.
    pub fn detect() -> Self {
        let mut info = SystemInfo::default();
        if let Some(fetched) = fetch_tool() {
            info = fetched;
        }
        read_procfs(&mut info);
        read_dmi(&mut info);
        read_lscpu(&mut info);
        read_lspci(&mut info);
        if info.architecture.is_none() {
            info.architecture = run("uname", &["-m"]).map(|s| s.trim().to_owned());
        }
        if info.hostname.is_none() {
            info.hostname = run("uname", &["-n"]).map(|s| s.trim().to_owned());
        }
        if info.disks.is_empty() {
            if let Some(text) = run("df", &["-h", "-l", "--output=source,size,used,avail,target"]) {
                info.disks = text.lines().skip(1).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
            }
        }
        if info.source.is_none() {
            info.source = Some("procfs".to_owned());
        }
        info
    }

    /// Convert KiB to GiB.
    pub fn ram_total_gb(&self) -> Option<f64> {
        self.ram_total_kb.map(|k| k as f64 / 1024.0 / 1024.0)
    }

    /// Free memory in GB.
    pub fn ram_available_gb(&self) -> Option<f64> {
        let total = self.ram_total_kb?;
        let used = self.ram_used_kb?;
        Some((total.saturating_sub(used)) as f64 / 1024.0 / 1024.0)
    }

    /// The "Computer:" line of System Properties, e.g.
    /// `"AMD Ryzen 7 5800H with Radeon Graphics    3.20 GHz"`.
    pub fn cpu_line(&self) -> String {
        let name = self.cpu_name.clone().unwrap_or_else(|| "Not available".to_owned());
        match self.cpu_mhz {
            Some(mhz) if mhz > 0.0 => format!("{name}    {:.2} GHz", mhz / 1000.0),
            _ => name,
        }
    }

    /// A CPU descriptor in msinfo32's `x86 Family ... ~NNNN Mhz` style.
    pub fn cpu_descriptor(&self) -> String {
        let mut parts = vec![self.cpu_name.clone().unwrap_or_else(|| "Not available".to_owned())];
        for (label, value) in [("Family", &self.cpu_family), ("Model", &self.cpu_model), ("Stepping", &self.cpu_stepping)] {
            if let Some(value) = value { parts.push(format!("{label} {value}")); }
        }
        if let Some(vendor) = &self.cpu_vendor { parts.push(vendor.clone()); }
        if let Some(mhz) = self.cpu_mhz { parts.push(format!("~{mhz:.0} Mhz")); }
        parts.join(" ")
    }
}

/// Tries `fastfetch` then `neofetch`; returns the first that reports anything.
fn fetch_tool() -> Option<SystemInfo> {
    for tool in ["fastfetch", "neofetch", "screenfetch"] {
        if let Some(mut info) = run_fetch(tool) {
            info.source = Some(tool.to_owned());
            return Some(info);
        }
    }
    None
}

fn run_fetch(tool: &str) -> Option<SystemInfo> {
    let args: &[&str] = match tool {
        "neofetch" => &["--stdout"],
        "screenfetch" => &["-N"],
        _ => &["--logo", "none"],
    };
    let text = strip_ansi(&run(tool, args)?);
    parse_fetch(&text)
}

fn parse_fetch(text: &str) -> Option<SystemInfo> {
    let map = parse_kv(&text);

    let mut info = SystemInfo::default();
    info.os_name = map_get(&map, &["os"]).cloned();
    info.kernel = map_get(&map, &["kernel"]).cloned();
    info.uptime = map_get(&map, &["uptime"]).cloned();
    if let Some(cpu) = map_get(&map, &["cpu"]) {
        info.cpu_mhz = parse_ghz(cpu);
        info.cpu_cores = parse_cores(cpu);
        info.cpu_name = Some(cpu.clone());
    }
    for (key, value) in &map {
        if key.starts_with("gpu") {
            info.gpus.push(value.clone());
        }
    }
    if let Some(mem) = map_get(&map, &["memory"]) {
        let (used, total) = parse_memory(mem);
        info.ram_used_kb = used;
        info.ram_total_kb = total;
    }
    if info.product_name.is_none() {
        info.product_name = map_get(&map, &["host", "model"]).cloned();
    }
    for (key, value) in &map {
        if key.starts_with("disk") { info.disks.push(value.clone()); }
    }

    // Nothing usable means the tool printed only usage / an error.
    if info.os_name.is_none() && info.cpu_name.is_none() && info.product_name.is_none() {
        return None;
    }
    Some(info)
}

/// Reads `/proc/cpuinfo`, `/proc/meminfo`, `/etc/os-release` and friends.
fn read_procfs(info: &mut SystemInfo) {
    if let Ok(cpu) = std::fs::read_to_string("/proc/cpuinfo") {
        let field = |key: &str| cpu.lines().filter_map(|l| l.split_once(':')).find(|(k, _)| k.trim() == key).map(|(_, v)| v.trim().to_owned());
        info.cpu_vendor = field("vendor_id");
        info.cpu_family = field("cpu family");
        info.cpu_model = field("model");
        info.cpu_stepping = field("stepping");
        if info.cpu_name.is_none() {
            info.cpu_name = cpu
                .lines()
                .find_map(|l| l.strip_prefix("model name"))
                .and_then(|rest| rest.split(':').nth(1))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);
        }
        if info.cpu_mhz.is_none() {
            info.cpu_mhz = cpu
                .lines()
                .find_map(|l| l.strip_prefix("cpu MHz"))
                .and_then(|rest| rest.split(':').nth(1))
                .and_then(|v| v.trim().parse::<f64>().ok());
        }
        if info.cpu_cores.is_none() {
            let n = cpu.lines().filter(|l| l.trim_start().starts_with("processor")).count();
            if n > 0 {
                info.cpu_cores = Some(n as u32);
            }
        }
    }

    if let Ok(mem) = std::fs::read_to_string("/proc/meminfo") {
        let field = |name: &str| -> Option<u64> {
            mem.lines()
                .find(|l| l.starts_with(name))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        };
        if let Some(total) = field("MemTotal:") {
            info.ram_total_kb = Some(total);
        }
        info.swap_total_kb = field("SwapTotal:");
        info.swap_free_kb = field("SwapFree:");
        if let (Some(total), Some(avail)) = (info.ram_total_kb, field("MemAvailable:")) {
            info.ram_used_kb = Some(total.saturating_sub(avail));
        }
    }

    if info.hostname.is_none() {
        info.hostname = read_trim("/proc/sys/kernel/hostname");
    }
    if info.kernel.is_none() {
        info.kernel = read_trim("/proc/version").map(|v| v.split_whitespace().take(3).collect::<Vec<_>>().join(" "));
    }
    if info.os_name.is_none() {
        if let Ok(release) = std::fs::read_to_string("/etc/os-release") {
            info.os_name = release
                .lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim().trim_matches('"').to_owned());
        }
    }
}

/// DMI/SMBIOS ground truth from `/sys/class/dmi/id` (authoritative; overrides).
fn read_dmi(info: &mut SystemInfo) {
    const BASE: &str = "/sys/class/dmi/id";
    if let Some(v) = read_trim(&format!("{BASE}/sys_vendor")) {
        info.sys_vendor = Some(v);
    }
    if let Some(v) = read_trim(&format!("{BASE}/product_name")) {
        info.product_name = Some(v);
    }
    if info.bios.is_none() {
        let version = read_trim(&format!("{BASE}/bios_version"));
        let date = read_trim(&format!("{BASE}/bios_date"));
        info.bios = match (version, date) {
            (Some(v), Some(d)) => Some(format!("{v}, {d}")),
            (Some(v), None) => Some(v),
            _ => None,
        };
    }
}

fn read_lscpu(info: &mut SystemInfo) {
    let Some(text) = run("lscpu", &[]) else {
        return;
    };
    let map = parse_kv(&text);
    info.architecture = map_get(&map, &["architecture"]).cloned();
    if info.cpu_vendor.is_none() { info.cpu_vendor = map_get(&map, &["vendor id"]).cloned(); }
    if info.cpu_family.is_none() { info.cpu_family = map_get(&map, &["cpu family"]).cloned(); }
    if info.cpu_model.is_none() { info.cpu_model = map_get(&map, &["model"]).cloned(); }
    if info.cpu_stepping.is_none() { info.cpu_stepping = map_get(&map, &["stepping"]).cloned(); }
    if info.cpu_name.is_none() {
        info.cpu_name = map_get(&map, &["model name"]).cloned();
    }
    if info.cpu_cores.is_none() {
        info.cpu_cores = map_get(&map, &["cpu(s)"])
            .and_then(|v| v.trim().parse::<u32>().ok());
    }
    if info.cpu_mhz.is_none() {
        info.cpu_mhz = map_get(&map, &["cpu mhz"])
            .and_then(|v| v.trim().parse::<f64>().ok());
    }
}

fn read_lspci(info: &mut SystemInfo) {
    if !info.gpus.is_empty() {
        return;
    }
    let Some(text) = run("lspci", &[]) else {
        return;
    };
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("vga compatible controller")
            || lower.contains("3d controller")
            || lower.contains("display controller")
        {
            if let Some(desc) = line.splitn(2, ": ").nth(1) {
                info.gpus.push(desc.trim().to_owned());
            }
        }
    }
}

/// Runs a program and returns its stdout, or `None` if it is missing / fails.
fn run(program: &str, args: &[&str]) -> Option<String> {
    run_with_timeout(program, args, Duration::from_secs(2))
}

pub(crate) fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut child = Command::new(program).args(args).env("LC_ALL", "C")
        .stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null()).spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(1024 * 1024).read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() { return None; }
                let bytes = receiver.recv_timeout(timeout.saturating_sub(start.elapsed())).ok()?.ok()?;
                return Some(String::from_utf8_lossy(&bytes).into_owned());
            }
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn read_trim(path: &str) -> Option<String> {
    let s = std::fs::read_to_string(path).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_owned())
    }
}

/// Collects `Key: value` pairs from fetch-tool output, lower-cased keys.
fn parse_kv(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.len() > 64 || key.contains('@') {
            continue;
        }
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        out.push((key.to_ascii_lowercase(), value.to_owned()));
    }
    out
}

fn map_get<'a>(map: &'a [(String, String)], keys: &[&str]) -> Option<&'a String> {
    for &key in keys {
        if let Some((_, value)) = map.iter().find(|(k, _)| k == key) {
            return Some(value);
        }
    }
    None
}

/// `"AMD Ryzen 7 5800H (16) @ 3.20 GHz"` -> `3200.0`.
fn parse_ghz(cpu: &str) -> Option<f64> {
    let idx = cpu.find('@')?;
    let rest = cpu[idx + 1..].trim();
    let mut parts = rest.split_whitespace();
    let value = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next().unwrap_or("GHz");
    if unit.to_ascii_lowercase().starts_with("ghz") {
        Some(value * 1000.0)
    } else {
        Some(value)
    }
}

/// `"AMD Ryzen 7 5800H (16) @ 3.20 GHz"` -> `16`.
fn parse_cores(cpu: &str) -> Option<u32> {
    let open = cpu.find('(')?;
    let close = cpu[open..].find(')')? + open;
    cpu[open + 1..close].trim().parse().ok()
}

/// `"7864MiB / 15872MiB"` -> `(used_kb, Some(total_kb))`.
fn parse_memory(mem: &str) -> (Option<u64>, Option<u64>) {
    let to_kb = |s: &str| -> Option<u64> {
        let s = s.trim();
        let (num, unit) = s.split_at(s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len()));
        let num: f64 = num.trim().parse().ok()?;
        if !num.is_finite() || num < 0.0 { return None; }
        let mult = if unit.to_ascii_lowercase().starts_with("gib") || unit.to_ascii_lowercase().starts_with("gb") {
            1024.0 * 1024.0
        } else if unit.to_ascii_lowercase().starts_with("kib") || unit.to_ascii_lowercase().starts_with("kb") {
            1.0
        } else {
            1024.0
        };
        Some((num * mult) as u64)
    };
    match mem.split_once('/') {
        Some((used, total)) => (to_kb(used), to_kb(total)),
        None => (None, to_kb(mem)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_formats_and_disk_paths() {
        let info = parse_fetch("OS: Linux\nHost: Real PC\nCPU: AMD Ryzen (16) @ 3.20 GHz\nGPU 1: Radeon\nMemory: 2.5 GiB / 16 GiB\nDisk (/): 10 GiB / 100 GiB").unwrap();
        assert_eq!(info.cpu_cores, Some(16));
        assert_eq!(info.cpu_mhz, Some(3200.0));
        assert_eq!(info.ram_total_kb, Some(16 * 1024 * 1024));
        assert_eq!(info.gpus, ["Radeon"]);
        assert_eq!(info.disks.len(), 1);
        assert!(parse_fetch("usage: fastfetch [options]").is_none());
    }

    #[test]
    fn memory_units_and_unknown_available() {
        assert_eq!(parse_memory("7864MiB / 15872MiB"), (Some(7864 * 1024), Some(15872 * 1024)));
        assert_eq!(parse_memory("-1 GiB / NaN GiB"), (None, None));
        let info = SystemInfo { ram_total_kb: Some(1024), ..Default::default() };
        assert_eq!(info.ram_available_gb(), None);
        assert_eq!(SystemInfo::default().cpu_descriptor(), "Not available");
    }

    #[test]
    fn command_failure_and_timeout() {
        assert!(run("sh", &["-c", "printf 'OS: fake'; exit 1"]).is_none());
        let start = Instant::now();
        assert!(run_with_timeout("sleep", &["5"], Duration::from_millis(50)).is_none());
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn ansi_and_cpu_units() {
        assert_eq!(strip_ansi("\x1b[32mCPU\x1b[0m: Test"), "CPU: Test");
        assert_eq!(parse_ghz("CPU @ 2400 MHz"), Some(2400.0));
        assert_eq!(parse_kv("Architecture:   x86_64")[0].1, "x86_64");
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip until the terminating 'm' of an SGR sequence.
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
