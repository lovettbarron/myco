use objc2::{msg_send, sel, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSMenu, NSMenuItem, NSTextField, NSView,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub const CTX_TAG_OPEN_IN_PANE: u32 = 2000;
pub const CTX_TAG_REVEAL_IN_FINDER: u32 = 2001;
pub const CTX_TAG_RENAME: u32 = 2002;
pub const CTX_TAG_DELETE: u32 = 2003;
pub const CTX_TAG_COPY_PATH: u32 = 2004;
pub const CTX_TAG_COPY_RELATIVE_PATH: u32 = 2005;

pub const CTX_TAG_FREEZE: u32 = 3000;
pub const CTX_TAG_UNFREEZE: u32 = 3001;
pub const CTX_TAG_CLOSE_PANEL: u32 = 3002;

pub const CTX_TAG_AGENT_FOCUS: u32 = 4000;
pub const CTX_TAG_AGENT_FREEZE: u32 = 4001;
pub const CTX_TAG_AGENT_UNFREEZE: u32 = 4002;
pub const CTX_TAG_AGENT_KILL: u32 = 4003;
pub const CTX_TAG_AGENT_COPY_STATS: u32 = 4004;

pub fn show_sidebar_context_menu(
    window: &winit::window::Window,
    x: f32,
    y: f32,
    is_dir: bool,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
        return;
    };

    let ns_view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    super::menu::with_menu_handler(|handler| {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));
        let action_sel = sel!(handleMenuAction:);

        if !is_dir {
            let item = make_item(mtm, "Open in New Pane", action_sel, CTX_TAG_OPEN_IN_PANE);
            unsafe { item.setTarget(Some(handler)) };
            menu.addItem(&item);
        }

        let item = make_item(mtm, "Reveal in Finder", action_sel, CTX_TAG_REVEAL_IN_FINDER);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let item = make_item(mtm, "Rename", action_sel, CTX_TAG_RENAME);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        let item = make_item(mtm, "Delete", action_sel, CTX_TAG_DELETE);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let item = make_item(mtm, "Copy Path", action_sel, CTX_TAG_COPY_PATH);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        let item = make_item(mtm, "Copy Relative Path", action_sel, CTX_TAG_COPY_RELATIVE_PATH);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        // winit's content view is flipped (isFlipped = YES), so y=0 is at
        // the top — same as our logical coordinate system. No transform needed.
        let ns_point = NSPoint::new(x as f64, y as f64);

        menu.popUpMenuPositioningItem_atLocation_inView(None, ns_point, Some(ns_view));
    });
}

fn make_item(
    mtm: MainThreadMarker,
    title: &str,
    action: objc2::runtime::Sel,
    tag: u32,
) -> objc2::rc::Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            ns_string!(""),
        )
    };
    item.setTag(tag as isize);
    item
}

/// Show a native context menu for a panel header.
///
/// Displays Freeze/Unfreeze options for panels with processes, plus Close Panel.
pub fn show_panel_context_menu(
    window: &winit::window::Window,
    x: f32,
    y: f32,
    is_frozen: bool,
    has_process: bool,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
        return;
    };

    let ns_view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    super::menu::with_menu_handler(|handler| {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));
        let action_sel = sel!(handleMenuAction:);

        if has_process {
            if is_frozen {
                let item = make_item(mtm, "Unfreeze Process", action_sel, CTX_TAG_UNFREEZE);
                unsafe { item.setTarget(Some(handler)) };
                menu.addItem(&item);
            } else {
                let item = make_item(mtm, "Freeze Process", action_sel, CTX_TAG_FREEZE);
                unsafe { item.setTarget(Some(handler)) };
                menu.addItem(&item);
            }

            menu.addItem(&NSMenuItem::separatorItem(mtm));
        }

        let item = make_item(mtm, "Close Panel", action_sel, CTX_TAG_CLOSE_PANEL);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        // winit's content view is flipped (isFlipped = YES), so y=0 is at
        // the top -- same as our logical coordinate system. No transform needed.
        let ns_point = NSPoint::new(x as f64, y as f64);

        menu.popUpMenuPositioningItem_atLocation_inView(None, ns_point, Some(ns_view));
    });
}

/// Show a native context menu for an agent row in the Agent Monitor panel.
///
/// Displays Focus Terminal, Freeze/Unfreeze (conditional), Kill, separator, Copy Stats.
pub fn show_agent_monitor_context_menu(
    window: &winit::window::Window,
    x: f32,
    y: f32,
    is_frozen: bool,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
        return;
    };

    let ns_view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    super::menu::with_menu_handler(|handler| {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));
        let action_sel = sel!(handleMenuAction:);

        // Focus Terminal
        let item = make_item(mtm, "Focus Terminal", action_sel, CTX_TAG_AGENT_FOCUS);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        // Freeze/Unfreeze (conditional)
        if is_frozen {
            let item = make_item(mtm, "Unfreeze Process", action_sel, CTX_TAG_AGENT_UNFREEZE);
            unsafe { item.setTarget(Some(handler)) };
            menu.addItem(&item);
        } else {
            let item = make_item(mtm, "Freeze Process", action_sel, CTX_TAG_AGENT_FREEZE);
            unsafe { item.setTarget(Some(handler)) };
            menu.addItem(&item);
        }

        // Kill
        let item = make_item(mtm, "Kill Agent", action_sel, CTX_TAG_AGENT_KILL);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Copy Stats
        let item = make_item(mtm, "Copy Stats", action_sel, CTX_TAG_AGENT_COPY_STATS);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        let ns_point = NSPoint::new(x as f64, y as f64);
        menu.popUpMenuPositioningItem_atLocation_inView(None, ns_point, Some(ns_view));
    });
}

pub fn show_rename_dialog(current_name: &str) -> Option<String> {
    let Some(mtm) = MainThreadMarker::new() else {
        return None;
    };

    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str("Rename"));
    alert.setInformativeText(&NSString::from_str(&format!(
        "Enter a new name for \"{}\":",
        current_name
    )));
    alert.addButtonWithTitle(&NSString::from_str("Rename"));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));

    let input = NSTextField::initWithFrame(NSTextField::alloc(mtm), NSRect {
        origin: NSPoint::new(0.0, 0.0),
        size: NSSize::new(300.0, 24.0),
    });
    input.setStringValue(&NSString::from_str(current_name));
    alert.setAccessoryView(Some(&input));

    unsafe {
        let _: () = msg_send![&*alert.window(), makeFirstResponder: &*input];
    }

    let response = alert.runModal();
    if response == NSAlertFirstButtonReturn {
        Some(input.stringValue().to_string())
    } else {
        None
    }
}

pub fn show_delete_confirmation(name: &str) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };

    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Warning);
    alert.setMessageText(&NSString::from_str(&format!("Delete \"{}\"?", name)));
    alert.setInformativeText(&NSString::from_str("This action cannot be undone."));
    alert.addButtonWithTitle(&NSString::from_str("Delete"));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));

    alert.runModal() == NSAlertFirstButtonReturn
}
