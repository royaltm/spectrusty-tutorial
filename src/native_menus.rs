#![cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
)))]

use minifb::Window;

pub struct AppMenu;

impl AppMenu {
    pub fn new(_window: &Window) -> AppMenu {
        AppMenu
    }

    #[inline(always)]
    pub fn is_menu_pressed(&self, window: &mut Window) -> Option<usize> {
        window.is_menu_pressed()
    }
}
