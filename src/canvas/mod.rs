//! TLDraw canvas cap module.
//!
//! Manages wry WebView instances for TLDraw canvas panels, including creation,
//! destruction, IPC message handling (auto-save, shortcut forwarding), and
//! webview focus management.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::debug;
use wry::WebView;
use winit::event_loop::EventLoopProxy;

use crate::grid::PanelId;
use crate::app::UserEvent;

mod assets;
mod state;

pub use state::CanvasState;

/// Manages all canvas (TLDraw webview) instances in the workspace.
///
/// Maps PanelId to CanvasState and WebView, following the TerminalManager pattern.
pub struct CanvasManager {
    canvases: HashMap<PanelId, CanvasState>,
    webviews: HashMap<PanelId, WebView>,
    project_dir: PathBuf,
}

impl CanvasManager {
    pub fn new(project_dir: PathBuf) -> Self {
        Self {
            canvases: HashMap::new(),
            webviews: HashMap::new(),
            project_dir,
        }
    }

    /// Create a new canvas webview for the given panel.
    pub fn create_canvas(
        &mut self,
        panel_id: PanelId,
        canvas_id: &str,
        window: &winit::window::Window,
        bounds: (f32, f32, f32, f32),
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure .myco/canvas/ directory exists
        let canvas_dir = self.project_dir.join(".myco").join("canvas");
        std::fs::create_dir_all(&canvas_dir)?;

        // Ensure .myco/context/ has default AI context files
        if let Err(e) = crate::context::ensure_context_files(&self.project_dir) {
            tracing::warn!("Failed to write context files: {}", e);
        }

        let tldr_path = canvas_dir.join(format!("{}.tldr", canvas_id));
        let state = CanvasState::new(canvas_id.to_string(), tldr_path.clone());

        // Create webview with custom protocol and IPC
        let webview = self.build_webview(window, bounds, panel_id, proxy)?;

        // If .tldr file exists, load it into the webview
        if tldr_path.exists() {
            let content = std::fs::read_to_string(&tldr_path)?;
            let escaped = content.replace('\\', "\\\\").replace('\'', "\\'");
            let _ = webview.evaluate_script(&format!(
                "setTimeout(() => window.__myco_load('{}'), 500)",
                escaped
            ));
        }

        self.canvases.insert(panel_id, state);
        self.webviews.insert(panel_id, webview);
        debug!("Created canvas for panel {:?} at {:?}", panel_id, tldr_path);
        Ok(())
    }

    fn build_webview(
        &self,
        window: &winit::window::Window,
        bounds: (f32, f32, f32, f32),
        panel_id: PanelId,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<WebView, Box<dyn std::error::Error>> {
        use wry::{WebViewBuilder, Rect, dpi::{LogicalPosition, LogicalSize}};
        let (x, y, w, h) = bounds;

        let webview = WebViewBuilder::new()
            .with_bounds(Rect {
                position: LogicalPosition::new(x as f64, y as f64).into(),
                size: LogicalSize::new(w as f64, h as f64).into(),
            })
            .with_custom_protocol("myco".into(), move |_webview_id, request| {
                let path = request.uri().path();
                let (content, mime) = assets::load_bundled_asset(path);
                http::Response::builder()
                    .header("Content-Type", mime)
                    .status(200)
                    .body(Cow::from(content))
                    .unwrap()
            })
            .with_url("myco://localhost/index.html")
            .with_ipc_handler(move |request| {
                let msg = request.body().to_string();
                let _ = proxy.send_event(UserEvent::CanvasMessage(panel_id, msg));
            })
            .with_focused(false)
            .with_navigation_handler(|url| url.starts_with("myco://")) // Allow custom protocol, block external (T-03-03)
            .build_as_child(window)?;

        Ok(webview)
    }

    /// Destroy a canvas webview and its state.
    pub fn destroy_canvas(&mut self, panel_id: &PanelId) {
        self.webviews.remove(panel_id);
        if self.canvases.remove(panel_id).is_some() {
            debug!("Destroyed canvas for panel {:?}", panel_id);
        }
    }

    /// Get an immutable reference to a canvas state.
    pub fn get(&self, panel_id: &PanelId) -> Option<&CanvasState> {
        self.canvases.get(panel_id)
    }

    /// Get a mutable reference to a canvas state.
    #[allow(dead_code)]
    pub fn get_mut(&mut self, panel_id: &PanelId) -> Option<&mut CanvasState> {
        self.canvases.get_mut(panel_id)
    }

    /// Get a reference to a canvas webview.
    #[allow(dead_code)]
    pub fn get_webview(&self, panel_id: &PanelId) -> Option<&WebView> {
        self.webviews.get(panel_id)
    }

    /// Get all webviews.
    #[allow(dead_code)]
    pub fn webviews(&self) -> &HashMap<PanelId, WebView> {
        &self.webviews
    }

    /// Handle IPC message from canvas JS. Returns true if state changed.
    pub fn handle_ipc_message(&mut self, panel_id: &PanelId, message: &str) -> bool {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(message) {
            match parsed.get("type").and_then(|t| t.as_str()) {
                Some("save") => {
                    if let Some(data) = parsed.get("data") {
                        if let Some(state) = self.canvases.get(panel_id) {
                            // Write .tldr file (D-02: auto-save)
                            let content = serde_json::to_string_pretty(data)
                                .unwrap_or_default();
                            if content.len() <= 50 * 1024 * 1024 {
                                // 50MB limit (security T-03-02)
                                let _ = std::fs::write(&state.tldr_path, &content);
                                debug!(
                                    "Auto-saved canvas {:?} ({} bytes)",
                                    panel_id,
                                    content.len()
                                );
                            } else {
                                tracing::warn!(
                                    "Canvas save rejected: {} bytes exceeds 50MB limit",
                                    content.len()
                                );
                            }
                        }
                    }
                    return true;
                }
                Some("shortcut") => {
                    // Handled at app level -- forwarded as InputAction
                    return false;
                }
                _ => {
                    tracing::warn!(
                        "Unknown canvas IPC message type from panel {:?}",
                        panel_id
                    );
                }
            }
        }
        false
    }

    /// Resize a canvas webview to new bounds.
    pub fn resize(&self, panel_id: &PanelId, bounds: (f32, f32, f32, f32)) {
        use wry::{Rect, dpi::{LogicalPosition, LogicalSize}};
        if let Some(wv) = self.webviews.get(panel_id) {
            let (x, y, w, h) = bounds;
            let _ = wv.set_bounds(Rect {
                position: LogicalPosition::new(x as f64, y as f64).into(),
                size: LogicalSize::new(w as f64, h as f64).into(),
            });
        }
    }

    /// Set focus state for a webview panel (D-16 desaturation).
    pub fn set_focus(&self, panel_id: &PanelId, focused: bool) {
        if let Some(wv) = self.webviews.get(panel_id) {
            let script = format!("window.__myco_set_focus({})", focused);
            let _ = wv.evaluate_script(&script);
            if focused {
                let _ = wv.focus();
            }
        }
    }

    /// Return focus from webview to parent window (for GPU panel focus).
    pub fn unfocus_all(&self) {
        for (_id, wv) in &self.webviews {
            let _ = wv.evaluate_script("window.__myco_set_focus(false)");
            let _ = wv.focus_parent();
        }
    }

    /// Check if any canvases exist.
    #[allow(dead_code)]
    pub fn has_canvases(&self) -> bool {
        !self.canvases.is_empty()
    }
}
