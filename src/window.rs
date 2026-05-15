use std::sync::Arc;
use tracing::info;
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

/// Create the application window with custom title bar configuration.
///
/// Window properties per design decisions:
/// - Title: "Myco"
/// - Default size: 1280x800 logical pixels
/// - Minimum size: 640x480 logical pixels
/// - macOS: transparent title bar, fullsize content view, hidden title (D-14)
/// - Centered at ~80% of screen size (D-13)
///
/// Does NOT call `with_decorations(false)` -- that would remove traffic lights (Pitfall 3).
pub fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attrs = WindowAttributes::default()
        .with_title("Myco")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(640.0, 480.0));

    #[cfg(target_os = "macos")]
    let attrs = attrs
        .with_titlebar_transparent(true)
        .with_fullsize_content_view(true)
        .with_title_hidden(true);

    let window = Arc::new(event_loop.create_window(attrs).unwrap());

    // Center on screen at ~80% size (D-13)
    if let Some(monitor) = window.current_monitor() {
        let screen = monitor.size();
        let w = (screen.width as f64 * 0.8) as u32;
        let h = (screen.height as f64 * 0.8) as u32;
        let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(w, h));
        info!(
            screen_width = screen.width,
            screen_height = screen.height,
            window_width = w,
            window_height = h,
            "Window sized to ~80% of screen"
        );
    }

    window
}
