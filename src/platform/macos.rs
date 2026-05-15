use objc2_app_kit::{NSView, NSWindowButton, NSWindowTitleVisibility};
use objc2_foundation::NSPoint;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Set up the custom title bar with repositioned traffic light buttons.
///
/// This function:
/// 1. Sets title visibility to hidden (belt and suspenders with winit's `with_title_hidden`)
/// 2. Repositions the traffic light buttons (close, miniaturize, zoom) to account for
///    the custom title bar layout.
///
/// Must be called after window creation in `App::resumed()`.
/// May need to be re-called after resize events (Assumption A4 from RESEARCH.md).
///
/// # Safety
/// Uses unsafe to cast the raw window handle pointer to an NSView reference,
/// then gets the NSWindow from it.
/// The pointer comes from winit's window handle (trusted source, T-01-02 accepted risk).
pub fn setup_custom_title_bar(window: &winit::window::Window) {
    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
        return;
    };

    // SAFETY: The pointer comes from winit's window handle, which is a valid NSView.
    let ns_view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    // Get the NSWindow from the NSView
    let Some(ns_window) = ns_view.window() else {
        return;
    };

    ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);

    // Reposition traffic light buttons to custom positions.
    // Standard macOS positions are approximately (7, 6) from top-left.
    // We offset them to account for the custom title bar height.
    let traffic_light_offset_x = 12.0_f64;
    let traffic_light_offset_y = 16.0_f64;

    for button_type in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(button) = ns_window.standardWindowButton(button_type) {
            let frame = button.frame();
            button.setFrameOrigin(NSPoint::new(
                frame.origin.x + traffic_light_offset_x,
                frame.origin.y + traffic_light_offset_y,
            ));
        }
    }
}
