use std::sync::Arc;
use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::grid::layout::GridLayout;
use crate::grid::panel::{Panel, PanelId, PanelType};
use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::renderer::Renderer;
use crate::theme::Theme;
use crate::window::create_window;

/// Height of the custom title bar area in pixels.
const TITLE_BAR_HEIGHT: f32 = 38.0;

/// Main application state.
///
/// Owns the window, renderer, grid layout, panels, and theme.
/// Renders a themed panel grid with GPU quads and glyphon text.
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    theme: Theme,
    grid: Option<GridLayout>,
    panels: Vec<Panel>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            theme: Theme::default(),
            grid: None,
            panels: Vec::new(),
        }
    }
}

impl App {
    /// Build quad instances for the current frame.
    fn build_quads(&self, width: f32, _height: f32) -> Vec<QuadInstance> {
        let mut quads = Vec::new();
        let grid = match &self.grid {
            Some(g) => g,
            None => return quads,
        };

        // Title bar background quad (full width, TITLE_BAR_HEIGHT tall)
        // Per D-02: subtle/borderless -- use main background color (no distinct strip)
        quads.push(QuadInstance {
            position: [0.0, 0.0],
            size: [width, TITLE_BAR_HEIGHT],
            color: self.theme.background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Panel quads
        for &(node, _panel_id) in grid.panel_nodes() {
            let (px, py, pw, ph) = grid.get_panel_rect(node);
            // Offset by title bar height
            let py_offset = py + TITLE_BAR_HEIGHT;

            // Panel background quad (themed dark per D-03)
            quads.push(QuadInstance {
                position: [px, py_offset],
                size: [pw, ph],
                color: self.theme.panel_background,
                corner_radius: 0.0,
                _padding: 0.0,
            });
        }

        quads
    }

    /// Build text labels for the current frame.
    #[allow(clippy::unused_self)]
    fn build_labels(&self, _width: f32, _height: f32) -> Vec<TextLabel> {
        let mut labels = Vec::new();
        let grid = match &self.grid {
            Some(g) => g,
            None => return labels,
        };

        // Title bar breadcrumb (D-14): "Myco > Untitled Project"
        // Positioned after traffic lights at x=80, y=10
        labels.push(TextLabel {
            text: "Myco > Untitled Project".to_string(),
            x: 80.0,
            y: 10.0,
            width: 300.0,
            height: TITLE_BAR_HEIGHT,
            font_size: 13.0,
            color: glyphon::Color::rgba(
                (self.theme.title_bar_text[0] * 255.0) as u8,
                (self.theme.title_bar_text[1] * 255.0) as u8,
                (self.theme.title_bar_text[2] * 255.0) as u8,
                (self.theme.title_bar_text[3] * 255.0) as u8,
            ),
        });

        // Panel labels
        for (i, &(node, _panel_id)) in grid.panel_nodes().iter().enumerate() {
            let (px, py, pw, ph) = grid.get_panel_rect(node);
            let py_offset = py + TITLE_BAR_HEIGHT;

            if let Some(panel) = self.panels.get(i) {
                // Panel title bar label (D-01): type name at top-left of panel
                labels.push(TextLabel {
                    text: panel.panel_type.to_string(),
                    x: px + 8.0,
                    y: py_offset + 4.0,
                    width: pw - 16.0,
                    height: 20.0,
                    font_size: 12.0,
                    color: glyphon::Color::rgba(
                        (self.theme.title_bar_text[0] * 255.0) as u8,
                        (self.theme.title_bar_text[1] * 255.0) as u8,
                        (self.theme.title_bar_text[2] * 255.0) as u8,
                        (self.theme.title_bar_text[3] * 255.0) as u8,
                    ),
                });

                // Centered type label in panel body (D-03)
                // Approximate center: use the panel's width for text area,
                // position at vertical center minus half font size
                let center_y = py_offset + ph / 2.0 - 7.0;
                labels.push(TextLabel {
                    text: panel.title.clone(),
                    x: px,
                    y: center_y,
                    width: pw,
                    height: 28.0,
                    font_size: 14.0,
                    color: glyphon::Color::rgba(
                        (self.theme.panel_label_text[0] * 255.0) as u8,
                        (self.theme.panel_label_text[1] * 255.0) as u8,
                        (self.theme.panel_label_text[2] * 255.0) as u8,
                        (self.theme.panel_label_text[3] * 255.0) as u8,
                    ),
                });
            }
        }

        labels
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only initialize once
        if self.window.is_some() {
            return;
        }

        info!("Application resumed -- creating window and GPU state");

        let window = create_window(event_loop);

        // Set up custom title bar with traffic lights (D-14)
        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::setup_custom_title_bar(&window);
        }

        let renderer = Renderer::new(window.clone());

        // Re-apply traffic light positioning after renderer init to handle layout settling
        // (Open Question 4 resolution from RESEARCH.md)
        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::setup_custom_title_bar(&window);
        }

        // Initialize grid with a single panel filling the window (D-12)
        let mut grid = GridLayout::new_single_panel();
        let size = window.inner_size();
        if size.width > 0 && size.height > 0 {
            let grid_height = size.height as f32 - TITLE_BAR_HEIGHT;
            grid.compute(size.width as f32, grid_height.max(1.0));
        }

        // Create the initial placeholder panel
        let panel = Panel::new_placeholder(PanelId(0));
        self.panels = vec![panel];

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.grid = Some(grid);

        info!("Application initialization complete");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested -- exiting");
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                // Guard against zero dimensions (Pitfall 2 / T-01-01 mitigation)
                if size.width > 0 && size.height > 0 {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }

                    // Recompute grid layout for new size
                    if let Some(grid) = &mut self.grid {
                        let grid_height = size.height as f32 - TITLE_BAR_HEIGHT;
                        grid.compute(size.width as f32, grid_height.max(1.0));
                    }

                    // Re-apply traffic light positioning after resize (Assumption A4)
                    #[cfg(target_os = "macos")]
                    if let Some(window) = &self.window {
                        crate::platform::macos::setup_custom_title_bar(window);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    let vw = size.width as f32;
                    let vh = size.height as f32;

                    // Build frame data before borrowing renderer mutably
                    let quads = self.build_quads(vw, vh);
                    let labels = self.build_labels(vw, vh);

                    if let Some(renderer) = &mut self.renderer {
                        match renderer.render(
                            self.theme.background,
                            &quads,
                            &labels,
                            vw,
                            vh,
                        ) {
                            crate::renderer::RenderResult::Ok => {}
                            crate::renderer::RenderResult::SkipFrame => {}
                            crate::renderer::RenderResult::SurfaceLost => {
                                warn!("Surface lost -- will attempt recovery next frame");
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
