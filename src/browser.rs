//! Real WebKitGTK pages rendered inside egui, keeping all XP chrome in Rust.
//! The worker uses system Python/GI bindings, avoiding unsafe code in this crate.
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use egui::{Color32, Rect, Sense, TextureHandle, TextureOptions, Ui};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Download {
    pub path: String,
    pub progress: f64,
    pub finished: bool,
    pub failed: bool,
}

pub struct Browser {
    child: Option<Child>,
    input: Option<mpsc::SyncSender<Value>>,
    writer_error: Receiver<String>,
    events: Receiver<Value>,
    texture: Option<TextureHandle>,
    size: [u32; 2],
    pub uri: String,
    pub title: String,
    pub loading: bool,
    pub progress: f64,
    pub can_back: bool,
    pub can_forward: bool,
    pub error: String,
    pub ready: bool,
    pub downloads: Vec<Download>,
    keyboard_focus: bool,
    started: bool,
}

impl Default for Browser {
    fn default() -> Self {
        let (_sender, receiver) = mpsc::sync_channel(8);
        let (_, writer_error) = mpsc::channel();
        Self { child: None, input: None, writer_error, events: receiver, texture: None, size: [0; 2], uri: "https://www.google.com/".to_owned(), title: "Internet Explorer".to_owned(), loading: false, progress: 0.0, can_back: false, can_forward: false, error: String::new(), ready: false, downloads: Vec::new(), keyboard_focus: false, started: false }
    }
}

impl Browser {
    fn start(&mut self) {
        if self.started { return; }
        self.started = true;
        let (sender, receiver) = mpsc::sync_channel(8);
        self.events = receiver;
        let python = std::env::var("WINXP_BROWSER_PYTHON").unwrap_or_else(|_| "/usr/bin/python3".to_owned());
        match Command::new(python).args(["-u", "-c", include_str!("browser_helper.py")])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let (commands, receiver) = mpsc::sync_channel::<Value>(64);
                    let (errors, error_receiver) = mpsc::channel();
                    self.input = Some(commands);
                    self.writer_error = error_receiver;
                    std::thread::spawn(move || {
                        while let Ok(command) = receiver.recv() {
                            if let Err(error) = writeln!(stdin, "{command}") {
                                let _ = errors.send(error.to_string());
                                break;
                            }
                        }
                    });
                }
                if let Some(output) = child.stdout.take() {
                    std::thread::spawn(move || {
                        for line in BufReader::new(output).lines() {
                            let Ok(line) = line else { break; };
                            if let Ok(event) = serde_json::from_str(&line) {
                                if sender.send(event).is_err() { break; }
                            }
                        }
                    });
                }
                self.child = Some(child);
            }
            Err(error) => self.error = format!("Cannot start WebKit: {error}. Install python3-gi, python3-gi-cairo and gir1.2-webkit2-4.1."),
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        self.send(json!({"action":"quit"}));
        self.input.take();
        if let Some(mut child) = self.child.take() {
            std::thread::spawn(move || {
                for _ in 0..20 {
                    if matches!(child.try_wait(), Ok(Some(_))) { return; }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                let _ = child.kill(); let _ = child.wait();
            });
        }
    }
}

impl Browser {
    pub fn available(&self) -> bool { self.child.is_some() }
    pub fn send(&mut self, message: Value) {
        if let Some(input) = &self.input {
            match input.try_send(message) {
                Ok(()) => {},
                Err(mpsc::TrySendError::Full(message)) => {
                    // Pointer motion is expendable; never block egui on a
                    // worker pipe or silently lose clicks/navigation.
                    if message.get("action").and_then(Value::as_str) != Some("motion") {
                        self.error = "The browser is busy. Please retry your action.".to_owned();
                    }
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    self.error = "The browser connection was closed.".to_owned();
                    self.input = None;
                }
            }
        }
    }
    pub fn action(&mut self, action: &str) { self.send(json!({"action": action})); }
    pub fn navigate(&mut self, uri: &str) {
        let Some(uri) = normalize_uri(uri) else { return; };
        self.error.clear(); self.loading = true; self.uri = uri.clone();
        self.send(json!({"action":"navigate", "uri":uri}));
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        self.start();
        if let Ok(error) = self.writer_error.try_recv() {
            self.error = format!("Browser connection lost: {error}");
            self.input = None;
        }
        while let Ok(event) = self.events.try_recv() {
            if event.get("ready").and_then(Value::as_bool) == Some(true) { self.ready = true; }
            if let Some(uri) = event.get("uri").and_then(Value::as_str) { self.uri = uri.to_owned(); }
            if let Some(title) = event.get("title").and_then(Value::as_str) { self.title = title.to_owned(); }
            if let Some(loading) = event.get("loading").and_then(Value::as_bool) { self.loading = loading; }
            if let Some(progress) = event.get("progress").and_then(Value::as_f64) { self.progress = progress; }
            if let Some(back) = event.get("back").and_then(Value::as_bool) { self.can_back = back; }
            if let Some(forward) = event.get("forward").and_then(Value::as_bool) { self.can_forward = forward; }
            if let Some(error) = event.get("error").and_then(Value::as_str) { self.error = error.to_owned(); }
            if let Some(path) = event.get("download").and_then(Value::as_str).filter(|p| !p.is_empty()) {
                let state = event.get("download_state").and_then(Value::as_str).unwrap_or("");
                if !self.downloads.iter().any(|d| d.path == path) {
                    self.downloads.push(Download { path: path.to_owned(), progress: 0.0, finished: false, failed: false });
                }
                if let Some(download) = self.downloads.iter_mut().find(|d| d.path == path) {
                    download.progress = event.get("download_progress").and_then(Value::as_f64).unwrap_or(download.progress);
                    download.finished = state == "finished" || download.finished;
                    download.failed = state == "failed" || download.failed;
                }
            }
            if let Some(path) = event.get("frame").and_then(Value::as_str) {
                if let Ok(image) = image::open(path) {
                    let image = image.to_rgba8();
                    let color = egui::ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_raw());
                    if let Some(texture) = &mut self.texture { texture.set(color, TextureOptions::LINEAR); }
                    else { self.texture = Some(ctx.load_texture("webkit-page", color, TextureOptions::LINEAR)); }
                }
                let _ = std::fs::remove_file(path);
            }
        }
        if let Some(child) = &mut self.child {
            if let Ok(Some(status)) = child.try_wait() {
                if self.error.is_empty() { self.error = format!("WebKit exited ({status}). Required packages: python3-gi, python3-gi-cairo, gir1.2-webkit2-4.1."); }
                self.child = None; self.input = None;
            }
        }
    }
    pub fn ui(&mut self, ui: &mut Ui, rect: Rect, focused: bool) {
        let size = [rect.width().max(1.0) as u32, rect.height().max(1.0) as u32];
        if size != self.size { self.size = size; self.send(json!({"action":"resize", "width":size[0], "height":size[1]})); }
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        if let Some(texture) = &self.texture {
            ui.painter().image(texture.id(), rect, Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if self.error.is_empty() { "Starting WebKit browser…" } else { &self.error }, egui::FontId::proportional(13.0), Color32::BLACK);
        }
        let response = ui.interact(rect, egui::Id::new(("webkit-page", rect.left() as i32, rect.top() as i32)), Sense::click_and_drag());
        if !focused { self.keyboard_focus = false; }
        let events = ui.input(|i| i.events.clone());
        let modifiers = ui.input(|i| i.modifiers);
        let mask = u32::from(modifiers.shift) | (u32::from(modifiers.ctrl) << 2) | (u32::from(modifiers.alt) << 3);
        for event in events {
            match event {
                egui::Event::PointerMoved(position) if rect.contains(position) && response.hovered() => {
                    self.send(json!({"action":"motion", "x":position.x-rect.left(), "y":position.y-rect.top(), "modifiers":mask}));
                }
                egui::Event::PointerButton { pos, button, pressed, .. } => {
                    if pressed { self.keyboard_focus = rect.contains(pos) && response.hovered(); }
                    if self.keyboard_focus {
                        let button = match button { egui::PointerButton::Primary => 1, egui::PointerButton::Middle => 2, egui::PointerButton::Secondary => 3, _ => 1 };
                        self.send(json!({"action":if pressed {"press"} else {"release"}, "x":pos.x-rect.left(), "y":pos.y-rect.top(), "button":button, "modifiers":mask}));
                    }
                }
                egui::Event::MouseWheel { delta, .. } if response.hovered() => {
                    let position = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.min);
                    self.send(json!({"action":"scroll", "x":position.x-rect.left(), "y":position.y-rect.top(), "dx":-delta.x/40.0, "dy":-delta.y/40.0}));
                }
                egui::Event::Paste(text) if focused && self.keyboard_focus => self.send(json!({"action":"key", "text":text, "modifiers":0})),
                egui::Event::Text(text) if focused && self.keyboard_focus => self.send(json!({"action":"key", "text":text, "modifiers":mask})),
                egui::Event::Key { key, pressed: true, .. } if focused && self.keyboard_focus => {
                    let value = match key {
                        egui::Key::Enter => 0xff0d, egui::Key::Backspace => 0xff08, egui::Key::Tab => 0xff09,
                        egui::Key::Escape => 0xff1b, egui::Key::Delete => 0xffff,
                        egui::Key::ArrowLeft => 0xff51, egui::Key::ArrowUp => 0xff52, egui::Key::ArrowRight => 0xff53, egui::Key::ArrowDown => 0xff54,
                        egui::Key::Home => 0xff50, egui::Key::End => 0xff57, egui::Key::PageUp => 0xff55, egui::Key::PageDown => 0xff56,
                        egui::Key::A if modifiers.ctrl => u32::from('a'), egui::Key::C if modifiers.ctrl => u32::from('c'), egui::Key::V if modifiers.ctrl => u32::from('v'), egui::Key::X if modifiers.ctrl => u32::from('x'),
                        _ => 0,
                    };
                    if value != 0 { self.send(json!({"action":"key", "keyval":value, "modifiers":mask})); }
                }
                _ => {}
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn normalize_uri(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() { return None; }
    let lower = value.to_ascii_lowercase();
    if value.contains("://") || lower.starts_with("about:") || lower.starts_with("data:") {
        Some(value.to_owned())
    } else if value.chars().any(char::is_whitespace) {
        Some(format!("https://www.google.com/search?q={}", encode(value)))
    } else if lower.starts_with("localhost:") || lower.starts_with("127.0.0.1:") || lower.starts_with("[::1]:") {
        Some(format!("http://{value}"))
    } else {
        Some(format!("https://{value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_browser_addresses() {
        assert_eq!(normalize_uri("  "), None);
        assert_eq!(normalize_uri("example.com"), Some("https://example.com".to_owned()));
        assert_eq!(normalize_uri("localhost:8000/page"), Some("http://localhost:8000/page".to_owned()));
        assert_eq!(normalize_uri("about:blank"), Some("about:blank".to_owned()));
        assert_eq!(normalize_uri("data:text/html,hello"), Some("data:text/html,hello".to_owned()));
        assert_eq!(normalize_uri("rust café"), Some("https://www.google.com/search?q=rust%20caf%C3%A9".to_owned()));
    }
    #[test]
    fn backpressure_does_not_block_ui() {
        let mut browser = Browser::default();
        let (sender, _receiver) = mpsc::sync_channel(1);
        browser.input = Some(sender);
        browser.action("reload");
        browser.send(json!({"action":"motion"}));
        assert!(browser.error.is_empty());
        browser.action("back");
        assert!(browser.error.contains("busy"));
    }
}

pub fn encode(text: &str) -> String {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}
