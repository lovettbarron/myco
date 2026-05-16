use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSObjectProtocol, NSString};
use serde::Deserialize;
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;

thread_local! {
    static MENU_PROXY: RefCell<Option<EventLoopProxy<UserEvent>>> = const { RefCell::new(None) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    pub struct MenuActionHandler;

    unsafe impl NSObjectProtocol for MenuActionHandler {}

    impl MenuActionHandler {
        #[unsafe(method(handleMenuAction:))]
        fn handle_menu_action(&self, sender: &NSMenuItem) {
            let tag = sender.tag() as u32;
            MENU_PROXY.with(|p| {
                if let Some(proxy) = p.borrow().as_ref() {
                    let _ = proxy.send_event(UserEvent::MenuAction(tag));
                }
            });
        }
    }
);

impl MenuActionHandler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

// -- Config types --

#[derive(Debug, Deserialize)]
pub struct MenuConfig {
    pub menus: Vec<MenuDef>,
}

#[derive(Debug, Deserialize)]
pub struct MenuDef {
    pub title: String,
    #[serde(default)]
    pub system: bool,
    pub items: Vec<MenuItemDef>,
}

#[derive(Debug, Deserialize)]
pub struct MenuItemDef {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub modifiers: String,
    #[serde(default)]
    pub separator: bool,
    #[serde(default)]
    pub toggle: Option<ToggleDef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToggleDef {
    pub state_key: String,
    pub on_label: String,
    pub off_label: String,
}

#[derive(Debug, Clone)]
pub struct ToggleEntry {
    pub tag: u32,
    pub def: ToggleDef,
}

pub struct MenuState {
    pub action_map: HashMap<u32, String>,
    pub toggles: Vec<ToggleEntry>,
}

const MENU_JSON: &str = include_str!("../../resources/menu.json");

pub fn setup_menu_bar(proxy: EventLoopProxy<UserEvent>) -> MenuState {
    let mtm = MainThreadMarker::new()
        .expect("setup_menu_bar must be called from the main thread");

    MENU_PROXY.with(|p| {
        *p.borrow_mut() = Some(proxy);
    });

    let config: MenuConfig = serde_json::from_str(MENU_JSON)
        .expect("Failed to parse menu.json");

    let app = NSApplication::sharedApplication(mtm);
    let handler = MenuActionHandler::new(mtm);
    let menu_bar = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));

    let mut action_map = HashMap::new();
    let mut toggles = Vec::new();
    let mut next_tag: u32 = 100;

    for menu_def in &config.menus {
        let menu_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&menu_def.title),
                None,
                ns_string!(""),
            )
        };
        let submenu = NSMenu::initWithTitle(
            NSMenu::alloc(mtm),
            &NSString::from_str(&menu_def.title),
        );

        for item_def in &menu_def.items {
            if item_def.separator {
                submenu.addItem(&NSMenuItem::separatorItem(mtm));
                continue;
            }

            let tag = next_tag;
            next_tag += 1;

            let (action_sel, key_str) = match item_def.action.as_str() {
                "about" => (Some(sel!(orderFrontStandardAboutPanel:)), ns_string!("")),
                "quit" => (Some(sel!(terminate:)), &*NSString::from_str(&item_def.key)),
                "minimize" => (Some(sel!(performMiniaturize:)), &*NSString::from_str(&item_def.key)),
                "zoom" => (Some(sel!(performZoom:)), ns_string!("")),
                _ => (Some(sel!(handleMenuAction:)), &*NSString::from_str(&item_def.key)),
            };

            let ns_item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(&item_def.label),
                    action_sel,
                    key_str,
                )
            };

            ns_item.setTag(tag as isize);

            if matches!(item_def.action.as_str(), "about" | "quit" | "minimize" | "zoom") {
                // System actions use the responder chain
            } else {
                unsafe { ns_item.setTarget(Some(handler.as_ref() as &AnyObject)) };
            }

            if !item_def.modifiers.is_empty() {
                let mask = parse_modifiers(&item_def.modifiers);
                ns_item.setKeyEquivalentModifierMask(mask);
            }

            action_map.insert(tag, item_def.action.clone());

            if let Some(toggle) = &item_def.toggle {
                toggles.push(ToggleEntry { tag, def: toggle.clone() });
            }

            submenu.addItem(&ns_item);
        }

        menu_item.setSubmenu(Some(&submenu));
        menu_bar.addItem(&menu_item);
    }

    app.setMainMenu(Some(&menu_bar));

    // Keep handler alive for the lifetime of the app
    std::mem::forget(handler);

    MenuState { action_map, toggles }
}

pub fn update_toggle_labels(state: &MenuState, app_state: &HashMap<String, bool>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let Some(main_menu) = app.mainMenu() else { return };

    for toggle in &state.toggles {
        let current = app_state.get(&toggle.def.state_key).copied().unwrap_or(false);
        let label = if current { &toggle.def.on_label } else { &toggle.def.off_label };

        if let Some(item) = find_item_by_tag(&main_menu, toggle.tag as isize) {
            item.setTitle(&NSString::from_str(label));
        }
    }
}

fn find_item_by_tag(menu: &NSMenu, tag: isize) -> Option<Retained<NSMenuItem>> {
    let count = menu.numberOfItems();
    for i in 0..count {
        if let Some(item) = menu.itemAtIndex(i) {
            if item.tag() == tag {
                return Some(item);
            }
            if let Some(sub) = item.submenu() {
                if let Some(found) = find_item_by_tag(&sub, tag) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn parse_modifiers(s: &str) -> NSEventModifierFlags {
    let mut flags = NSEventModifierFlags::empty();
    let lower = s.to_lowercase();
    if lower.contains("cmd") || lower.contains("command") {
        flags |= NSEventModifierFlags::Command;
    }
    if lower.contains("shift") {
        flags |= NSEventModifierFlags::Shift;
    }
    if lower.contains("alt") || lower.contains("option") {
        flags |= NSEventModifierFlags::Option;
    }
    if lower.contains("ctrl") || lower.contains("control") {
        flags |= NSEventModifierFlags::Control;
    }
    flags
}
