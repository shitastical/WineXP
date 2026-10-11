//! Command Prompt (`cmd.exe`).
//!
//! Ported from Reborn XP's `js/apps/cmd.js` (`xp.quenq.com`): the VGA 16-colour
//! palette, the `$P$G` prompt, the command set (cd/chdir, cls, color, dir, echo,
//! exit, help, md/mkdir, rd/rmdir, del/erase, type, ver, date, time, set, path,
//! prompt, copy, move, ren/rename, start, ping, title, rem) and the up/down
//! command history. The filesystem is Reborn XP's DiskManager, backed here by
//! the real Wine `C:\` (see [`crate::vfs`]).

use std::collections::BTreeMap;

use egui::{Color32, FontId, Id, Key, RichText, Sense};

use crate::apps::{AppCtx, XpApp};
use crate::vfs;

const VERSION_STR: &str = "Microsoft Windows XP [Version 5.1.2600]";
const COPYRIGHT_STR: &str = "(C) Copyright 1985-2001 Microsoft Corp.";

/// `cmd-fg-N` from `cmd.js`: `.cmd-fg-0 { color: #000000 } … .cmd-fg-f { #ffffff }`.
const FG: [Color32; 16] = [
    Color32::from_rgb(0x00, 0x00, 0x00),
    Color32::from_rgb(0x00, 0x00, 0x80),
    Color32::from_rgb(0x00, 0x80, 0x00),
    Color32::from_rgb(0x00, 0x80, 0x80),
    Color32::from_rgb(0x80, 0x00, 0x00),
    Color32::from_rgb(0x80, 0x00, 0x80),
    Color32::from_rgb(0x80, 0x80, 0x00),
    Color32::from_rgb(0xc0, 0xc0, 0xc0),
    Color32::from_rgb(0x80, 0x80, 0x80),
    Color32::from_rgb(0x00, 0x00, 0xff),
    Color32::from_rgb(0x00, 0xff, 0x00),
    Color32::from_rgb(0x00, 0xff, 0xff),
    Color32::from_rgb(0xff, 0x00, 0x00),
    Color32::from_rgb(0xff, 0x00, 0xff),
    Color32::from_rgb(0xff, 0xff, 0x00),
    Color32::from_rgb(0xff, 0xff, 0xff),
];

/// `cmd-bg-N` from `cmd.js` (note the upstream `bg-6`/#800000 quirk is kept).
const BG: [Color32; 16] = [
    Color32::from_rgb(0x00, 0x00, 0x00),
    Color32::from_rgb(0x00, 0x00, 0x80),
    Color32::from_rgb(0x00, 0x80, 0x00),
    Color32::from_rgb(0x00, 0x80, 0x80),
    Color32::from_rgb(0x80, 0x00, 0x00),
    Color32::from_rgb(0x80, 0x00, 0x80),
    Color32::from_rgb(0x80, 0x00, 0x00),
    Color32::from_rgb(0xc0, 0xc0, 0xc0),
    Color32::from_rgb(0x80, 0x80, 0x80),
    Color32::from_rgb(0x00, 0x00, 0xff),
    Color32::from_rgb(0x00, 0xff, 0x00),
    Color32::from_rgb(0x00, 0xff, 0xff),
    Color32::from_rgb(0xff, 0x00, 0x00),
    Color32::from_rgb(0xff, 0x00, 0xff),
    Color32::from_rgb(0xff, 0xff, 0x00),
    Color32::from_rgb(0xff, 0xff, 0xff),
];

/// Commands offered by `help` (ported from the `helpTopics` table).
const HELP_TOPICS: &[(&str, &str)] = &[
    ("cd", "Displays the name of or changes the current directory."),
    ("chdir", "Displays the name of or changes the current directory."),
    ("cls", "Clears the screen."),
    ("color", "Sets the default console foreground and background colors."),
    ("copy", "Copies one or more files to another location."),
    ("date", "Displays or sets the date."),
    ("del", "Deletes one or more files."),
    ("dir", "Displays a list of files and subdirectories in a directory."),
    ("echo", "Displays messages, or turns command echoing on or off."),
    ("erase", "Deletes one or more files. (Same as DEL)"),
    ("exit", "Quits the CMD.EXE program (command interpreter)."),
    ("help", "Provides Help information for Windows XP commands."),
    ("md", "Creates a directory."),
    ("mkdir", "Creates a directory."),
    ("move", "Moves files and renames files and directories."),
    ("path", "Displays or sets a search path for executable files."),
    ("ping", "Sends ICMP ECHO_REQUEST packets to network hosts."),
    ("prompt", "Changes the cmd.exe command prompt."),
    ("rd", "Removes (deletes) a directory."),
    ("rem", "Records comments (remarks) in batch files or CONFIG.SYS."),
    ("ren", "Renames a file or files."),
    ("rename", "Renames a file or files."),
    ("rmdir", "Removes (deletes) a directory."),
    ("set", "Displays, sets, or removes cmd.exe environment variables."),
    ("start", "Starts a separate window to run a specified program or command."),
    ("time", "Displays or sets the system time."),
    ("title", "Sets the window title for a CMD.EXE session."),
    ("type", "Displays the contents of a text file or files."),
    ("ver", "Displays the Windows XP version."),
];

pub struct Cmd {
    lines: Vec<String>,
    input: String,
    cwd: String,
    env: BTreeMap<String, String>,
    history: Vec<String>,
    hist_index: i32,
    fg: usize,
    bg: usize,
    pending_focus: bool,
    user: String,
    ready: bool,
}

impl Default for Cmd {
    fn default() -> Self {
        Self::new()
    }
}

impl Cmd {
    pub fn new() -> Self {
        let home = "C:/Documents and Settings/User".to_owned();
        let mut env = BTreeMap::new();
        for (key, value) in [
            ("PROMPT", "$P$G"),
            ("PATH", "C:/WINDOWS/system32;C:/WINDOWS;"),
            ("TEMP", "C:/WINDOWS/Temp"),
            ("TMP", "C:/WINDOWS/Temp"),
            ("USERNAME", "User"),
            ("USERPROFILE", home.as_str()),
            ("SystemDrive", "C:"),
            ("SystemRoot", "C:/WINDOWS"),
            ("WINDIR", "C:/WINDOWS"),
            ("ComSpec", "C:/WINDOWS/system32/cmd.exe"),
            ("ERRORLEVEL", "0"),
        ] {
            env.insert(key.to_owned(), value.to_owned());
        }
        let mut cmd = Self {
            lines: Vec::new(),
            input: String::new(),
            cwd: home,
            env,
            history: Vec::new(),
            hist_index: -1,
            fg: 7,
            bg: 0,
            pending_focus: true,
            user: "User".to_owned(),
            ready: false,
        };
        cmd.out(VERSION_STR);
        cmd.out(COPYRIGHT_STR);
        cmd.lines.push(String::new());
        cmd
    }

    /// Adopts the logged-on account and starts in their real home directory
    /// inside the Wine prefix (`C:\users\<user>`, else the first profile found).
    fn adopt_user(&mut self, fs: &crate::vfs::Vfs, user: &str) {
        if self.ready && self.user == user {
            return;
        }
        let home = resolve_home(fs, user);
        if !self.ready || !fs.is_dir(&self.cwd) {
            self.cwd = home.clone();
        }
        self.env.insert("USERNAME".to_owned(), user.to_owned());
        self.env.insert("USERPROFILE".to_owned(), home);
        self.user = user.to_owned();
        self.ready = true;
    }

    fn out(&mut self, text: &str) {
        for line in text.split('\n') {
            self.lines.push(line.to_owned());
        }
    }

    fn fg(&self) -> Color32 {
        FG[self.fg]
    }

    fn prompt(&self) -> String {
        let mut p = self.env.get("PROMPT").cloned().unwrap_or_else(|| "$P$G".to_owned());
        p = replace_ci(&p, "$P", &self.cwd.replace('/', "\\"));
        p = replace_ci(&p, "$G", ">");
        p = replace_ci(&p, "$N", self.cwd.split(':').next().unwrap_or("C"));
        p = replace_ci(&p, "$Q", "=");
        p = replace_ci(&p, "$$", "$");
        p
    }

    /// `%VAR%` expansion (ported from `utils.expandEnvVars`).
    fn expand(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = String::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if let Some(end) = text[i + 1..].find('%') {
                    let name = &text[i + 1..i + 1 + end];
                    let key = name.to_ascii_uppercase();
                    match self.env.get(&key) {
                        Some(value) => out.push_str(value),
                        None => out.push_str(&text[i..i + 2 + end]),
                    }
                    i += end + 2;
                    continue;
                }
            }
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    /// `utils.resolvePath`: expands env vars, then joins against `cwd` unless the
    /// result is already absolute.
    fn resolve(&self, arg: &str) -> String {
        let expanded = self.expand(arg);
        if vfs::is_absolute(&expanded) {
            vfs::normalize_path(&expanded)
        } else if expanded.starts_with('\\') || expanded.starts_with('/') {
            // Drive-root-relative, e.g. `cd \` or `\WINDOWS`.
            vfs::normalize_path(&format!("C:{expanded}"))
        } else {
            vfs::join(&self.cwd, &expanded)
        }
    }

    fn parse_args(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        for ch in text.chars() {
            match ch {
                '"' => in_quotes = !in_quotes,
                c if c.is_whitespace() && !in_quotes => {
                    if !current.is_empty() {
                        out.push(std::mem::take(&mut current));
                    }
                }
                c => current.push(c),
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
        out
    }

    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.hist_index < self.history.len() as i32 - 1 {
            self.hist_index += 1;
            self.input = self.history[self.hist_index as usize].clone();
        }
    }

    fn history_next(&mut self) {
        if self.hist_index > 0 {
            self.hist_index -= 1;
            self.input = self.history[self.hist_index as usize].clone();
        } else if self.hist_index == 0 {
            self.hist_index = -1;
            self.input.clear();
        }
    }

    fn execute_line(&mut self, ctx: &mut AppCtx, raw: &str) {
        let prompt = self.prompt();
        let line = raw.trim_end().to_owned();
        self.lines.push(format!("{prompt}{line}"));
        if line.trim().is_empty() {
            return;
        }
        self.history.insert(0, line.clone());
        self.history.truncate(20);
        self.hist_index = -1;

        let expanded = self.expand(&line);
        let mut parts = Self::parse_args(&expanded);
        if parts.is_empty() {
            return;
        }
        let command = parts.remove(0).to_ascii_lowercase();
        let args = parts.join(" ");
        self.env.insert("ERRORLEVEL".to_owned(), "0".to_owned());

        let fs = ctx.vfs;
        match command.as_str() {
            "cd" | "chdir" => self.cmd_cd(fs, &args),
            "cls" => self.lines.clear(),
            "color" => self.cmd_color(&args),
            "dir" => self.cmd_dir(fs, &args),
            "echo" => self.cmd_echo(&args),
            "exit" => ctx.request_close(),
            "help" => self.cmd_help(&args),
            "md" | "mkdir" => self.cmd_mkdir(fs, &args),
            "rd" | "rmdir" => self.cmd_rmdir(fs, &args),
            "del" | "erase" => self.cmd_del(fs, &args),
            "type" => self.cmd_type(fs, &args),
            "ver" => {
                self.out("");
                self.out(VERSION_STR);
                self.out("");
            }
            "date" => {
                let now = chrono::Local::now();
                self.out(&format!("The current date is: {}", now.format("%m/%d/%Y")));
            }
            "time" => {
                let now = chrono::Local::now();
                self.out(&format!("The current time is: {}", now.format("%I:%M:%S %p")));
            }
            "set" => self.cmd_set(&args),
            "path" => self.cmd_path(&args),
            "prompt" => self.cmd_prompt(&args),
            "copy" => self.cmd_copy(fs, &args),
            "move" => self.cmd_move(fs, &args),
            "ren" | "rename" => self.cmd_ren(fs, &args),
            "start" => self.cmd_start(ctx, &args),
            "ping" => self.cmd_ping(&args),
            "title" => {}
            "rem" => {}
            "mode" => self.out(&format!("Simulated MODE command: {args}")),
            _ => {
                self.out(&format!(
                    "'{command}' is not recognized as an internal or external command,\noperable program or batch file.\n"
                ));
                self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            }
        }
    }

    fn cmd_cd(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let arg = Self::parse_args(args).first().cloned().unwrap_or_default();
        if arg.is_empty() || arg == "." || arg == "./" {
            self.out(&self.cwd.replace('/', "\\"));
            return;
        }
        if arg == ".." {
            let parent = vfs::dirname(&self.cwd);
            if parent != self.cwd {
                self.cwd = parent;
            }
            return;
        }
        let target = self.resolve(&arg);
        if fs.is_dir(&target) {
            self.cwd = target;
        } else if fs.is_file(&target) {
            self.out("The directory name is invalid.");
        } else {
            self.out("The system cannot find the path specified.");
        }
    }

    fn cmd_color(&mut self, args: &str) {
        let arg = args.trim().to_ascii_lowercase();
        if arg.is_empty() {
            self.fg = 7;
            self.bg = 0;
            return;
        }
        let valid = |c: char| c.is_ascii_hexdigit();
        match arg.len() {
            1 if valid(arg.chars().next().unwrap()) => {
                self.fg = digit(&arg);
            }
            2 if arg.chars().all(valid) => {
                self.bg = digit(&arg[0..1]);
                self.fg = digit(&arg[1..2]);
            }
            _ => self.out("Invalid color attribute specified."),
        }
    }

    fn cmd_dir(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let arg = Self::parse_args(args).first().cloned().unwrap_or_default();
        let target = if arg.is_empty() { self.cwd.clone() } else { self.resolve(&arg) };
        if fs.is_file(&target) {
            let name = vfs::basename(&target);
            let size = std::fs::metadata(fs.resolve(&target))
                .map(|m| m.len())
                .unwrap_or(0);
            self.out(&format!("{size:>14} {name}"));
            self.out(&format!("\n{:>16} File(s) {size:>14} bytes", 1));
            return;
        }
        let entries = match fs.list(&target).ok() {
            Some(entries) => entries,
            None => {
                self.out("File Not Found");
                return;
            }
        };
        let drive = target.chars().next().unwrap_or('C');
        self.out(&format!(" Volume in drive {drive} has no label."));
        self.out(" Volume Serial Number is FAKE-BEEF");
        self.out(&format!("\n Directory of {}\n", target.replace('/', "\\")));

        let mut files = 0usize;
        let mut dirs = 0usize;
        let mut bytes = 0u64;
        for entry in &entries {
            let (date, time) = format_time(entry.modified);
            if entry.is_dir {
                self.out(&format!("{date}  {time}    <DIR>          {}", entry.name));
                dirs += 1;
            } else {
                self.out(&format!("{date}  {time} {:>16} {}", entry.size, entry.name));
                files += 1;
                bytes += entry.size;
            }
        }
        self.out(&format!("\n{files:>16} File(s) {bytes:>14} bytes"));
        self.out(&format!("{dirs:>16} Dir(s)  2147483648 bytes free"));
    }

    fn cmd_echo(&mut self, args: &str) {
        let text = self.expand(args);
        self.out(&text);
    }

    fn cmd_help(&mut self, args: &str) {
        let topic = args.trim().to_ascii_lowercase();
        if !topic.is_empty() {
            match HELP_TOPICS.iter().find(|(name, _)| *name == topic) {
                Some((_, text)) => self.out(text),
                None => self.out("This command is not supported by the help utility."),
            }
            return;
        }
        self.out("For more information on a specific command, type HELP command-name\n");
        let mut topics: Vec<&(&str, &str)> = HELP_TOPICS.iter().collect();
        topics.sort_by_key(|(name, _)| *name);
        for (name, description) in topics {
            self.out(&format!(" {:<12} {}", name.to_ascii_uppercase(), description));
        }
    }

    fn cmd_mkdir(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let arg = Self::parse_args(args).first().cloned().unwrap_or_default();
        if arg.is_empty() {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let target = self.resolve(&arg);
        match std::fs::create_dir(fs.resolve(&target)) {
            Ok(()) => {}
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    self.out(&format!("A subdirectory or file {} already exists.", vfs::basename(&target)));
                } else {
                    self.out(&err.to_string());
                }
                self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            }
        }
    }

    fn cmd_rmdir(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let arg = Self::parse_args(args).first().cloned().unwrap_or_default();
        if arg.is_empty() {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let target = self.resolve(&arg);
        if !fs.exists(&target) {
            self.out("The system cannot find the file specified.");
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            return;
        }
        if !fs.is_dir(&target) {
            self.out("The directory name is invalid.");
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            return;
        }
        if fs.list(&target).map(|e| !e.is_empty()).unwrap_or(false) {
            self.out("The directory is not empty.");
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            return;
        }
        if let Err(err) = std::fs::remove_dir(fs.resolve(&target)) {
            self.out(&format!("Error: {err}"));
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
        }
    }

    fn cmd_del(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let arg = Self::parse_args(args).first().cloned().unwrap_or_default();
        if arg.is_empty() {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let target = self.resolve(&arg);
        if !fs.exists(&target) {
            self.out(&format!("Could Not Find {}", target.replace('/', "\\")));
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            return;
        }
        if fs.is_dir(&target) {
            self.out(&format!("{} is a directory. Use RMDIR.", target.replace('/', "\\")));
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
            return;
        }
        if let Err(err) = std::fs::remove_file(fs.resolve(&target)) {
            self.out(&format!("Error: {err}"));
            self.env.insert("ERRORLEVEL".to_owned(), "1".to_owned());
        }
    }

    fn cmd_type(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let arg = Self::parse_args(args).first().cloned().unwrap_or_default();
        if arg.is_empty() {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let target = self.resolve(&arg);
        if !fs.exists(&target) {
            self.out("The system cannot find the file specified.");
            return;
        }
        if fs.is_dir(&target) {
            self.out("Access is denied.");
            return;
        }
        match std::fs::read_to_string(fs.resolve(&target)) {
            Ok(text) => self.out(text.trim_end_matches('\n')),
            Err(_) => self.out("The system cannot find the file specified."),
        }
    }

    fn cmd_set(&mut self, args: &str) {
        let arg = args.trim();
        if arg.is_empty() {
            let pairs: Vec<(String, String)> =
                self.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            for (key, value) in pairs {
                self.out(&format!("{key}={value}"));
            }
            return;
        }
        if let Some((key, value)) = arg.split_once('=') {
            self.env.insert(key.trim().to_ascii_uppercase(), value.trim().to_owned());
        } else {
            let key = arg.to_ascii_uppercase();
            match self.env.get(&key) {
                Some(value) => self.out(&format!("{key}={value}")),
                None => self.out(&format!("Environment variable {key} not defined")),
            }
        }
    }

    fn cmd_path(&mut self, args: &str) {
        let arg = args.trim();
        if arg.is_empty() {
            self.out(&format!("PATH={}", self.env.get("PATH").cloned().unwrap_or_default()));
        } else {
            self.env.insert("PATH".to_owned(), arg.to_owned());
        }
    }

    fn cmd_prompt(&mut self, args: &str) {
        let arg = args.trim();
        if arg.is_empty() {
            self.out(&format!(
                "Current prompt is: {}",
                self.env.get("PROMPT").cloned().unwrap_or_else(|| "$P$G".to_owned())
            ));
        } else {
            self.env.insert("PROMPT".to_owned(), arg.to_owned());
        }
    }

    fn cmd_copy(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let parts = Self::parse_args(args);
        if parts.len() < 2 {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let from = self.resolve(&parts[0]);
        let mut to = self.resolve(&parts[1..].join(" "));
        if !fs.is_file(&from) {
            self.out("The system cannot find the file specified.");
            return;
        }
        if fs.is_dir(&to) {
            to = vfs::join(&to, &vfs::basename(&from));
        }
        match std::fs::copy(fs.resolve(&from), fs.resolve(&to)) {
            Ok(_) => self.out("        1 file(s) copied."),
            Err(err) => self.out(&err.to_string()),
        }
    }

    fn cmd_move(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let parts = Self::parse_args(args);
        if parts.len() < 2 {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let from = self.resolve(&parts[0]);
        let mut to = self.resolve(&parts[1..].join(" "));
        if !fs.exists(&from) {
            self.out("The system cannot find the file specified.");
            return;
        }
        if fs.is_dir(&to) {
            to = vfs::join(&to, &vfs::basename(&from));
        } else if parts[1].ends_with('\\') || parts[1].ends_with('/') {
            let _ = std::fs::create_dir(fs.resolve(&to));
            to = vfs::join(&to, &vfs::basename(&from));
        }
        match std::fs::rename(fs.resolve(&from), fs.resolve(&to)) {
            Ok(()) => self.out("        1 file(s) moved."),
            Err(err) => self.out(&err.to_string()),
        }
    }

    fn cmd_ren(&mut self, fs: &crate::vfs::Vfs, args: &str) {
        let parts = Self::parse_args(args);
        if parts.len() != 2 {
            self.out("The syntax of the command is incorrect.");
            return;
        }
        let from = self.resolve(&parts[0]);
        let to = vfs::join(&vfs::dirname(&from), &parts[1]);
        match std::fs::rename(fs.resolve(&from), fs.resolve(&to)) {
            Ok(()) => {}
            Err(err) => self.out(&err.to_string()),
        }
    }

    fn cmd_start(&mut self, ctx: &mut AppCtx, args: &str) {
        let parts = Self::parse_args(args);
        let Some(target) = parts.first() else {
            self.out("No application specified.");
            return;
        };
        let key = target.to_ascii_lowercase();
        let key = key.strip_suffix(".exe").or_else(|| key.strip_suffix(".com")).unwrap_or(&key);
        let app = match key {
            "notepad" => "Notepad",
            "mspaint" | "paint" => "Paint",
            "explorer" => "Windows Explorer",
            "iexplore" => "Internet Explorer",
            "winmine" => "Minesweeper",
            "cmd" => "Command Prompt",
            "control" => "Control Panel",
            "winecfg" | "wine" => "Wine Configuration",
            "winamp" => "Winamp",
            _ => {
                self.out(&format!("'{target}' is not recognized."));
                return;
            }
        };
        ctx.launch.push(app.to_owned());
    }

    fn cmd_ping(&mut self, args: &str) {
        let parts = Self::parse_args(args);
        let mut host = "127.0.0.1".to_owned();
        let mut count = 4;
        let mut i = 0;
        while i < parts.len() {
            let token = &parts[i];
            if token.eq_ignore_ascii_case("-n") {
                if let Some(value) = parts.get(i + 1) {
                    count = value.parse().unwrap_or(4);
                    i += 1;
                }
            } else if !token.starts_with('-') {
                host = token.clone();
            }
            i += 1;
        }
        self.out(&format!("\nPinging {host} with 32 bytes of data:"));
        for _ in 0..count {
            self.out(&format!("Reply from {host}: bytes=32 time<1ms TTL=128"));
        }
        self.out(&format!(
            "\nPing statistics for {host}:\n    Packets: Sent = {count}, Received = {count}, Lost = 0 (0% loss),"
        ));
    }
}

/// The logged-on user's home inside the Wine prefix: `C:/users/<user>` if it
/// exists, otherwise the first profile directory, otherwise the drive root.
fn resolve_home(fs: &crate::vfs::Vfs, user: &str) -> String {
    for candidate in [format!("C:/users/{user}"), format!("C:/Documents and Settings/{user}")] {
        if fs.is_dir(&candidate) {
            return candidate;
        }
    }
    if let Ok(entries) = fs.list("C:/users") {
        if let Some(entry) = entries
            .into_iter()
            .find(|entry| entry.is_dir && !entry.name.eq_ignore_ascii_case("Public"))
        {
            return format!("C:/users/{}", entry.name);
        }
    }
    "C:/".to_owned()
}

fn format_time(time: Option<std::time::SystemTime>) -> (String, String) {
    let Some(time) = time else {
        return ("       ".to_owned(), "     ".to_owned());
    };
    let dt: chrono::DateTime<chrono::Local> = time.into();
    (dt.format("%m/%d/%Y").to_string(), dt.format("%I:%M %p").to_string())
}

fn digit(hex: &str) -> usize {
    usize::from_str_radix(hex, 16).unwrap_or(7)
}

/// Case-insensitive `replace_all` (ported from `utils.getPrompt`'s `$` tokens).
fn replace_ci(haystack: &str, needle: &str, with: &str) -> String {
    let hay = haystack.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while i < haystack.len() {
        if hay[i..].starts_with(&needle) {
            out.push_str(with);
            i += needle.len();
        } else {
            let ch = haystack[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

impl XpApp for Cmd {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let user = ctx.user_name.clone();
        self.adopt_user(ctx.vfs, &user);
        let fg = self.fg();
        let bg = BG[self.bg];
        ui.painter().rect_filled(area, 0.0, bg);

        // Clicking empty space refocuses the prompt (added before the scroll area
        // so the input keeps click priority inside its own rect).
        if ui.interact(area, Id::new("cmd-bg"), Sense::click()).clicked() {
            self.pending_focus = true;
        }

        let prompt = self.prompt();
        let font = FontId::monospace(13.0);
        let mut submit: Option<String> = None;
        let mut move_history = 0i32;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                for line in &self.lines {
                    ui.label(RichText::new(line).font(font.clone()).color(fg));
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(RichText::new(&prompt).font(font.clone()).color(fg));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .font(font.clone())
                            .text_color(fg)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                    if self.pending_focus {
                        response.request_focus();
                        self.pending_focus = false;
                    }
                    if response.has_focus() {
                        if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                            move_history = -1;
                        } else if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                            move_history = 1;
                        }
                    }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        submit = Some(std::mem::take(&mut self.input));
                    }
                });
            });

        if move_history == -1 {
            self.history_prev();
        } else if move_history == 1 {
            self.history_next();
        }
        if let Some(line) = submit {
            self.execute_line(ctx, &line);
            self.pending_focus = true;
        }
    }
}
