//! Notepad — faithful port of `apps/Notepad`.

use egui::{Color32, FontId, Rect, Ui, Vec2};

use crate::apps::{place, AppCtx, XpApp};
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem};
use crate::theme;

pub struct Notepad {
    text: String,
    word_wrap: bool,
    dropdown: DropdownState,
    menus: Vec<DropdownMenu>,
}

impl Default for Notepad {
    fn default() -> Self {
        Self {
            text: String::new(),
            word_wrap: false,
            dropdown: DropdownState::default(),
            menus: menus(),
        }
    }
}

impl XpApp for Notepad {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        // Background: linear-gradient(to right, #edede5 0%, #ede8cd 100%)
        theme::hgrad(
            ui.painter(),
            area,
            theme::rgb(0xedede5),
            theme::rgb(0xede8cd),
        );

        // Toolbar: height 21px, border-bottom 1px white.
        let toolbar = Rect::from_min_size(area.min, Vec2::new(area.width(), 21.0));
        ui.painter().line_segment(
            [toolbar.left_bottom(), toolbar.right_bottom()],
            egui::Stroke::new(1.0, Color32::WHITE),
        );

        let clicked = menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &self.menus, 21.0);
        if let Some(item) = clicked {
            match item.as_str() {
                "Exit" => ctx.request_close(),
                "Word Wrap" => self.word_wrap = !self.word_wrap,
                "Time/Date" => {
                    let stamp = format!(
                        "{} {}",
                        ctx.now.format("%-I:%M:%S %p"),
                        ctx.now.format("%m/%d/%Y")
                    );
                    self.text.push_str(&stamp);
                }
                _ => {}
            }
        }

        // Text area fills the remainder.
        let text_rect = Rect::from_min_max(
            egui::pos2(area.left(), toolbar.bottom()),
            area.max,
        );
        ui.painter().rect_filled(text_rect, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(
            text_rect,
            0.0,
            egui::Stroke::new(1.0, theme::rgb(0x96abff)),
            egui::StrokeKind::Inside,
        );

        let inner = text_rect.shrink(3.0);
        let width = if self.word_wrap { inner.width() } else { 4000.0 };
        let buffer = &mut self.text;
        place(ui, inner, |ui| {
            let edit = egui::TextEdit::multiline(buffer)
                .font(FontId::monospace(13.0))
                .frame(egui::Frame::NONE)
                .desired_width(width)
                .desired_rows(1);
            ui.add(edit);
        });
    }
}

fn menus() -> Vec<DropdownMenu> {
    let item = |text: &str, disabled: bool| MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled,
        symbol: menu::Symbol::None,
    };
    vec![
        DropdownMenu {
            label: "File",
            items: vec![
                item("New", true),
                item("Open...", true),
                item("Save", true),
                item("Save As...", true),
                MenuItem::Separator,
                item("Page Setup...", true),
                item("Print...", true),
                MenuItem::Separator,
                item("Exit", false),
            ],
        },
        DropdownMenu {
            label: "Edit",
            items: vec![
                item("Undo...", true),
                MenuItem::Separator,
                item("Cut", true),
                item("Copy", true),
                item("Paste", true),
                item("Delete", true),
                MenuItem::Separator,
                item("Find...", true),
                item("Find Next", true),
                item("Replace...", true),
                item("Go To...", true),
                MenuItem::Separator,
                item("Select All", true),
                item("Time/Date", false),
            ],
        },
        DropdownMenu {
            label: "Format",
            items: vec![item("Word Wrap", false), item("Font...", true)],
        },
        DropdownMenu {
            label: "View",
            items: vec![item("Status Bar", true)],
        },
        DropdownMenu {
            label: "Help",
            items: vec![item("Help Topics", true), item("About Notepad", true)],
        },
    ]
}
