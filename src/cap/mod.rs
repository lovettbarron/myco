//! Cap trait system for extensible panel types.
//!
//! Caps are the content types that live inside grid panels. Each cap type
//! implements the `Cap` trait and optionally `GpuCap` or `WebviewCap`
//! depending on its rendering strategy.

use std::path::Path;

use crate::grid::PanelId;
use crate::renderer::quad_renderer::QuadInstance;
use crate::theme::Theme;

/// How a cap renders its content within a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// GPU-rendered via the shared wgpu pipeline. Produces quads and text areas
    /// that are composited in the main render pass.
    Gpu,
    /// Webview-based. Positioned as a native child view managed by wry.
    /// The render loop only draws the unfocused overlay on top.
    Webview,
}

/// Events a cap emits back to the app.
#[derive(Debug)]
pub enum CapEvent {
    /// Content changed — schedule a redraw.
    Dirty,
    /// The cap wants to close itself (e.g. shell exited).
    RequestClose { panel_id: PanelId },
    /// The cap wants to dispatch an app-level action.
    Action(crate::input::InputAction),
}

/// Content bounds for a panel's drawable area (after title bar, padding).
#[derive(Debug, Clone, Copy)]
pub struct CapBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CapBounds {
    pub fn as_tuple(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width, self.height)
    }
}

/// Shared services injected into caps. Avoids each cap storing its own
/// copy of project-wide state.
pub struct CapContext<'a> {
    pub project_dir: &'a Path,
    pub theme: &'a Theme,
    pub scale_factor: f32,
}

/// Scroll input that the app routes to the focused cap.
#[derive(Debug, Clone, Copy)]
pub enum ScrollInput {
    /// Line-based scroll (terminal). Positive = scroll back in history.
    Lines(i32),
    /// Pixel-based scroll (markdown, future rich content). Positive = scroll down.
    Pixels(f32),
}

/// The core capability trait. Every panel content type implements this.
///
/// Design principle: methods have default no-op implementations so caps only
/// override what they need. A minimal read-only viewer only needs `render_mode()`
/// and `title()`.
pub trait Cap: Send {
    /// Declares how this cap renders. The app uses this to route to the
    /// correct rendering path (GPU quad pipeline vs. webview positioning).
    fn render_mode(&self) -> RenderMode;

    /// Display title for the panel's title bar.
    fn title(&self) -> &str;

    /// Called when the panel content area is resized.
    fn resize(&mut self, _bounds: CapBounds) {}

    /// Called when the panel is being destroyed. Clean up resources.
    fn destroy(&mut self) {}

    /// Whether this cap captures keyboard input (preventing app shortcuts).
    /// Terminal returns true; markdown returns false.
    fn captures_keyboard(&self) -> bool {
        false
    }

    /// Handle scroll input. Return true if consumed.
    fn handle_scroll(&mut self, _input: ScrollInput, _viewport_height: f32) -> bool {
        false
    }

    /// Whether the cap needs a redraw (content changed since last frame).
    fn needs_redraw(&self) -> bool {
        false
    }

    /// Drain pending events from this cap.
    fn drain_events(&mut self) -> Vec<CapEvent> {
        Vec::new()
    }

    /// Optional: file path this cap is associated with (for file-watching integration).
    fn associated_file(&self) -> Option<&Path> {
        None
    }

    /// Called when an associated file changes on disk.
    fn file_changed(&mut self) {}
}

/// Extended interface for GPU-rendered caps.
///
/// Caps implementing this produce geometry (colored quads) and text areas
/// that the shared render pipeline composites each frame.
pub trait GpuCap: Cap {
    /// Produce quads for the current frame.
    fn build_quads(&self, bounds: CapBounds, theme: &Theme) -> Vec<QuadInstance>;

    /// Collect text areas for glyphon rendering. The lifetime ties to internal
    /// text buffer caches.
    fn collect_text_areas(&self, bounds: CapBounds, scale: f32) -> Vec<glyphon::TextArea<'_>>;

    /// Invalidate all cached render state (e.g. on theme change or font resize).
    fn invalidate_cache(&mut self);

    /// Update text buffer caches for the given bounds. Called before
    /// `build_quads` and `collect_text_areas` each frame when `needs_redraw()`.
    fn update_cache(
        &mut self,
        _bounds: CapBounds,
        _font_system: &mut glyphon::FontSystem,
        _theme: &Theme,
    ) {
    }
}

/// Extended interface for webview-based caps.
///
/// The app positions the webview as a child window and routes IPC messages.
/// Rendering is handled by the webview engine (WKWebView on macOS).
pub trait WebviewCap: Cap {
    /// Handle an IPC message from the webview JavaScript.
    fn handle_ipc(&mut self, message: &str) -> Vec<CapEvent>;

    /// Set visual focus state (controls CSS desaturation in the webview).
    fn set_focus(&self, focused: bool);

    /// Reposition the webview to new bounds.
    fn reposition(&self, bounds: CapBounds);
}
