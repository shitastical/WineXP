//! Pre-desktop screens: the Award BIOS POST, the boot-device menu, the XP
//! "starting" screen, the "Reinstall Windows XP" flow, the OOBE setup wizard
//! and the updates screen. Ported from Reborn XP (`xp.quenq.com`) — the BIOS
//! scenes, `#reinstall-confirm` and `js/apps/msoobe.js` — and from
//! `ducbao414/win32.run` (`xp/starting.svelte`).

pub mod bootmenu;
pub mod install;
pub mod login;
pub mod oobe;
pub mod post;
pub mod starting;
pub mod updates;

/// The pre-desktop boot stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BootStage {
    /// Award BIOS POST text.
    Post,
    /// The boot-device menu.
    BootMenu,
    /// The XP "starting" screen (logo + three-block loader).
    Starting,
    /// The Welcome splash.
    Welcome,
    /// The OOBE ("msoobe") setup wizard, shown until `oobepassed` is set.
    Oobe,
    /// The logon screen (click your user name).
    Logon,
    /// "Loading your personal settings...".
    LoggingIn,
    /// Windows Setup ("Install Windows").
    Installing,
    /// The desktop is up.
    Desktop,
}
