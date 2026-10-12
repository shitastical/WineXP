#!/usr/bin/python3
"""WebKitGTK rendering worker for WineXP's Rust Internet Explorer chrome.
No HTTP proxy or HTML simulation: WebKit owns HTTPS, JavaScript and navigation.
"""
import os
os.environ.setdefault("WEBKIT_DISABLE_COMPOSITING_MODE", "1")
# Renderer placement requires X11. Wayland sessions use XWayland when available.
if os.environ.get("DISPLAY"):
    os.environ.setdefault("GDK_BACKEND", "x11")
import json
import sys
import threading
import tempfile
import shutil
import time
import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import Gtk, WebKit2, Gdk, GLib


def emit(**data):
    print(json.dumps(data), flush=True)


class Browser:
    def __init__(self):
        self.directory = tempfile.mkdtemp(prefix="winxp-webkit-")
        # WebKit needs a native GdkWindow; Gtk.OffscreenWindow produces blank
        # pages on some WebKit builds. The renderer must never be visible:
        # some WMs (notably GNOME/Mutter, and any pure-Wayland session where
        # client positioning is ignored) clamp move(-12000, -12000) back onto
        # the visible screen, which left a "Google" window at the top-left
        # outside the WineXP shell. Use an override-redirect POPUP (unmanaged
        # by the WM on X11/XWayland) *plus* zero opacity (the Wayland-proof
        # hiding, where positioning is ignored) so the window stays invisible
        # even if it gets mapped at 0,0. It is only a pixel source; egui
        # draws the snapshots inside the XP IE window.
        self.window = Gtk.Window(type=Gtk.WindowType.POPUP)
        self.window.set_title("WineXP hidden renderer")
        self.window.set_role("winxp-hidden-renderer")
        self.window.set_decorated(False)
        self.window.set_resizable(False)
        self.window.set_skip_taskbar_hint(True)
        self.window.set_skip_pager_hint(True)
        self.window.set_accept_focus(False)
        self.window.set_focus_on_map(False)
        self.window.set_keep_below(True)
        try:
            self.window.set_opacity(0)
        except Exception:
            pass
        self.window.set_default_size(800, 600)
        self.window.move(-12000, -12000)
        self.view = WebKit2.WebView()
        self.view.get_settings().set_enable_javascript(True)
        self.window.add(self.view)
        self.window.show_all()
        # Re-assert after mapping: some WMs reposition on map.
        try:
            self.window.set_opacity(0)
        except Exception:
            pass
        self.window.move(-12000, -12000)
        try:
            self.window.lower()
            self.window.set_keep_below(True)
        except Exception:
            pass
        try:
            self.window.connect("map-event", lambda *a: self._rehide())
        except Exception:
            pass
        # Do NOT grab focus here: it raises the renderer window above the
        # shell on some WMs. Focus is grabbed per click in `command("press")`.
        self.busy = False
        self.dirty = True
        self.frame = 0
        self.last_snapshot = 0.0
        self.view.connect("notify::uri", self.state)
        self.view.connect("notify::title", self.state)
        self.view.connect("notify::estimated-load-progress", self.state)
        self.view.connect("load-changed", self.state)
        self.view.connect("load-failed", self.failed)
        self.view.connect("create", self.popup)
        self.view.connect("web-process-terminated", self.terminated)
        self.view.get_context().connect("download-started", self.download_started)
        self.downloads = {}
        GLib.timeout_add(250, self.snapshot)
        # Start blank; Rust sends the real homepage ("navigate") once the IE
        # window is actually drawn, so no Google content exists before then.
        self.view.load_uri("about:blank")
        threading.Thread(target=self.read_commands, daemon=True).start()
        emit(ready=True)

    def _rehide(self, *args):
        # Best-effort: keep the renderer off screen and fully transparent.
        try:
            self.window.set_opacity(0)
        except Exception:
            pass
        try:
            self.window.move(-12000, -12000)
            self.window.lower()
            self.window.set_keep_below(True)
        except Exception:
            pass
        return False

    def state(self, *_):
        self.dirty = True
        emit(uri=self.view.get_uri() or "", title=self.view.get_title() or "Internet Explorer",
             loading=self.view.is_loading(), progress=self.view.get_estimated_load_progress(),
             back=self.view.can_go_back(), forward=self.view.can_go_forward())

    def failed(self, view, event, uri, error):
        # WebKit intentionally interrupts a frame load when handing it to the
        # download manager; this is not a failed download or navigation.
        if error.matches(WebKit2.policy_error_quark(), WebKit2.PolicyError.FRAME_LOAD_INTERRUPTED_BY_POLICY_CHANGE):
            return True
        emit(error=str(error), uri=uri)
        self.dirty = True
        return False

    def terminated(self, *_):
        emit(error="The WebKit web process stopped. Press Refresh to reload the page.")

    def popup(self, view, action):
        request = action.get_request()
        if request.get_uri():
            self.view.load_uri(request.get_uri())
        return None

    def download_started(self, context, download):
        download.connect("decide-destination", self.download_destination)
        download.connect("received-data", self.download_progress)
        download.connect("finished", self.download_finished)
        download.connect("failed", self.download_failed)

    def download_destination(self, download, suggested):
        from pathlib import Path
        root = os.environ.get("WINXP_DRIVE_C") or os.path.join(os.environ.get("WINEPREFIX", os.path.expanduser("~/.wine")), "drive_c")
        directory = os.path.join(root, "users", os.environ.get("USER", "User"), "Downloads")
        os.makedirs(directory, exist_ok=True)
        name = os.path.basename(suggested).replace("\\\\", "_") or "download"
        destination = os.path.join(directory, name)
        stem, extension = os.path.splitext(name)
        index = 1
        while os.path.exists(destination):
            destination = os.path.join(directory, stem + " (" + str(index) + ")" + extension)
            index += 1
        download.set_destination(Path(destination).as_uri())
        self.downloads[download] = destination
        emit(download=destination, download_progress=0, download_state="started")
        return True

    def download_progress(self, download, length):
        emit(download=self.downloads.get(download, ""), download_progress=download.get_estimated_progress(), download_state="progress")

    def download_finished(self, download):
        path = self.downloads.pop(download, "")
        emit(download=path, download_progress=1, download_state="finished")

    def download_failed(self, download, error):
        emit(download=self.downloads.pop(download, ""), download_state="failed", error=str(error))

    def snapshot(self):
        if self.busy:
            return True
        # Active pages render at 4 fps; idle pages get a 1 fps heartbeat.
        # This is an intentional lightweight baseline, not a video player.
        if not self.dirty and not self.view.is_loading() and time.monotonic() - self.last_snapshot < 1.0:
            return True
        self.dirty = False
        self.last_snapshot = time.monotonic()
        self.busy = True
        self.view.get_snapshot(WebKit2.SnapshotRegion.VISIBLE, WebKit2.SnapshotOptions.NONE,
                               None, self.snapshot_done, None)
        return True

    def snapshot_done(self, view, result, _):
        try:
            surface = view.get_snapshot_finish(result)
            self.frame += 1
            path = os.path.join(self.directory, str(self.frame) + ".png")
            surface.write_to_png(path)
            emit(frame=path)
        except Exception as error:
            emit(error="Page snapshot failed: " + str(error))
        self.busy = False

    def read_commands(self):
        try:
            for line in sys.stdin:
                command = json.loads(line)
                GLib.idle_add(self.command, command)
        except Exception as error:
            emit(error=str(error))
        GLib.idle_add(Gtk.main_quit)

    def event(self, kind, command):
        event = Gdk.Event.new(kind)
        event.window = self.view.get_window()
        event.send_event = True
        event.time = Gdk.CURRENT_TIME
        event.x = float(command.get("x", 0))
        event.y = float(command.get("y", 0))
        event.state = Gdk.ModifierType(command.get("modifiers", 0))
        device = Gdk.Display.get_default().get_default_seat().get_pointer()
        event.set_device(device)
        if kind in (Gdk.EventType.BUTTON_PRESS, Gdk.EventType.BUTTON_RELEASE):
            event.button = int(command.get("button", 1))
        if kind == Gdk.EventType.SCROLL:
            event.direction = Gdk.ScrollDirection.SMOOTH
            event.delta_x = float(command.get("dx", 0))
            event.delta_y = float(command.get("dy", 0))
        self.view.event(event)

    def command(self, command):
        try:
            action = command.get("action")
            if action == "navigate": self.view.load_uri(command["uri"])
            elif action == "back": self.view.go_back()
            elif action == "forward": self.view.go_forward()
            elif action == "reload": self.view.reload()
            elif action == "stop": self.view.stop_loading()
            elif action == "resize":
                width, height = max(1, int(command["width"])), max(1, int(command["height"]))
                self.view.set_size_request(width, height)
                self.window.resize(width, height)
            elif action == "motion": self.event(Gdk.EventType.MOTION_NOTIFY, command)
            elif action == "press":
                self.view.grab_focus()
                self.event(Gdk.EventType.BUTTON_PRESS, command)
            elif action == "release": self.event(Gdk.EventType.BUTTON_RELEASE, command)
            elif action == "scroll": self.event(Gdk.EventType.SCROLL, command)
            elif action == "key":
                keyval = command.get("keyval", 0)
                text = command.get("text") or ""
                if text:
                    for character in text:
                        # Character by character: WebKit matches each event
                        # against its own key bindings, so the payload and the
                        # keyval always have to travel together.
                        self.key(
                            Gdk.unicode_to_keyval(ord(character)),
                            command.get("modifiers", 0),
                            character,
                        )
                elif keyval:
                    self.key(keyval, command.get("modifiers", 0), "")
            elif action == "settings":
                self.view.get_settings().set_enable_javascript(bool(command.get("javascript", True)))
                self.view.set_zoom_level(float(command.get("zoom", 1.0)))
            elif action == "zoom": self.view.set_zoom_level(float(command["value"]))
            elif action == "print": WebKit2.PrintOperation.new(self.view).run_dialog(None)
            elif action == "quit": Gtk.main_quit()
            self.dirty = True
        except Exception as error:
            emit(error=str(error))
        return False

    def key(self, keyval, modifiers, text=""):
        window = self.view.get_window()
        if window is None:
            return
        device = Gdk.Display.get_default().get_default_seat().get_keyboard()
        for kind in (Gdk.EventType.KEY_PRESS, Gdk.EventType.KEY_RELEASE):
            event = Gdk.EventKey.new(kind)
            event.window = window
            event.send_event = True
            event.time = Gdk.CURRENT_TIME
            event.keyval = keyval
            event.state = Gdk.ModifierType(modifiers)
            event.device = device
            # WebKit reads the characters to insert from `string`, never from
            # `keyval` alone. Leaving it empty is what made every page look
            # focused while silently swallowing everything typed into it.
            event.string = text if kind == Gdk.EventType.KEY_PRESS else ""
            event.length = len(event.string)
            self.view.event(event)


try:
    browser = Browser()
    Gtk.main()
    shutil.rmtree(browser.directory, ignore_errors=True)
except Exception as error:
    emit(error=str(error))
    sys.exit(1)
