//! Pre-desktop screens: the Award BIOS POST, the boot-device menu and the XP
//! "starting" screen. Ported from `ducbao414/win32.run` (`boot_manager.svelte`,
//! `xp/starting.svelte`).

pub mod bootmenu;
pub mod post;
pub mod starting;

/// The pre-desktop boot stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BootStage {
    /// Award BIOS POST text.
    Post,
    /// The boot-device menu.
    BootMenu,
    /// The XP "starting" screen (logo + three-block loader).
    Starting,
    /// The desktop is up.
    Desktop,
}
