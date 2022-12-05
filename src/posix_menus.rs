#![cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use std::fmt;
use std::collections::HashMap;
use log::{info};
use minifb::{Window, UnixMenu, Key, KeyRepeat, MENU_KEY_SHIFT, MENU_KEY_CTRL, MENU_KEY_ALT};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct KeyMod(Key, usize);
type KeyHashMap = HashMap<KeyMod, usize>;

pub struct AppMenu(KeyHashMap);

impl fmt::Debug for KeyMod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("KeyMod")
         .field(&self.0)
         .field(&format_args!("{}{}{}", if (self.1 & MENU_KEY_SHIFT) != 0 { "+SHIFT" } else { "" }
                                      , if (self.1 & MENU_KEY_CTRL) != 0 { "+CTRL" } else { "" }
                                      , if (self.1 & MENU_KEY_ALT) != 0 { "+ALT" } else { "" }))
         .finish()
    }
}

impl AppMenu {
    pub fn new(window: &Window) -> AppMenu {
        let mut keys = KeyHashMap::new();
        if let Some(menus) = window.get_posix_menus() {
            fn populate(menu: &UnixMenu, keys: &mut KeyHashMap) {
                for subitem in &menu.items {
                    if let Some(menu) = subitem.sub_menu.as_ref() {
                        populate(menu, keys);
                    }
                    else if subitem.enabled && subitem.key != Key::Unknown {
                        let kmod = KeyMod(subitem.key, subitem.modifier);
                        info!("Menu item {:>3} - {} - {:?}",
                            subitem.id, subitem.label, kmod);
                        keys.insert(kmod, subitem.id);
                    }
                }
            }
            for menu in menus {
                populate(menu, &mut keys);
            }
        }
        AppMenu(keys)
    }

    #[inline(always)]
    pub fn is_menu_pressed(&self, window: &mut Window) -> Option<usize> {
        let pressed = window.get_keys_pressed(KeyRepeat::No);
        if !pressed.is_empty() {
            let mut modifier = 0;
            if window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift) {
                modifier |= MENU_KEY_SHIFT;
            }
            if window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl) {
                modifier |= MENU_KEY_CTRL;
            }
            if window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt) {
                modifier |= MENU_KEY_ALT;
            }
            for key in pressed {
                let kmod = KeyMod(key, modifier);
                if let Some(menu) = self.0.get(&kmod) {
                    info!("Menu item {} by {:?}", menu, kmod);
                    return Some(*menu);
                }
            }
        }
        None
    }
}
