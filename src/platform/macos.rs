use std::sync::OnceLock;

use objc2_app_kit::{NSView, NSWindowButton, NSWindowTitleVisibility};
use objc2_foundation::NSPoint;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Original traffic light button positions, captured before any repositioning.
static ORIGINAL_BUTTON_POSITIONS: OnceLock<[(f64, f64); 3]> = OnceLock::new();

/// Set up the custom title bar with repositioned traffic light buttons.
///
/// Called multiple times (after window creation, renderer init, and on resize).
/// Captures original positions on first call and always sets absolute targets
/// to avoid cumulative drift.
///
/// # Safety
/// Uses unsafe to cast the raw window handle pointer to an NSView reference.
/// The pointer comes from winit's window handle (trusted source).
pub fn setup_custom_title_bar(window: &winit::window::Window) {
    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
        return;
    };

    let ns_view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    let Some(ns_window) = ns_view.window() else {
        return;
    };

    ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);

    let traffic_light_offset_x = 8.0_f64;
    let traffic_light_offset_y = 0.0_f64;

    let buttons = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ];

    let originals = ORIGINAL_BUTTON_POSITIONS.get_or_init(|| {
        let mut positions = [(0.0, 0.0); 3];
        for (i, button_type) in buttons.iter().enumerate() {
            if let Some(button) = ns_window.standardWindowButton(*button_type) {
                let frame = button.frame();
                positions[i] = (frame.origin.x, frame.origin.y);
            }
        }
        positions
    });

    for (i, button_type) in buttons.iter().enumerate() {
        if let Some(button) = ns_window.standardWindowButton(*button_type) {
            button.setFrameOrigin(NSPoint::new(
                originals[i].0 + traffic_light_offset_x,
                originals[i].1 + traffic_light_offset_y,
            ));
        }
    }
}
