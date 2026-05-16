use std::sync::Arc;
use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window, WindowId};

use crate::grid::divider::{
    self, compute_dividers, DividerSet, Orientation, DIVIDER_VISUAL_WIDTH,
};
use crate::grid::layout::GridLayout;
use crate::grid::operations::{self, SplitDirection};
use crate::grid::panel::{Panel, PanelId};
use crate::input::keyboard;
use crate::input::mouse::MouseState;
use crate::input::{CursorStyle, InputAction};
use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::renderer::Renderer;
use crate::theme::Theme;
use crate::window::create_window;

/// Height of the custom title bar area in pixels.
const TITLE_BAR_HEIGHT: f32 = 38.0;

/// Main application state.
///
/// Owns the window, renderer, grid layout, panels, theme, and input state.
/// Renders a themed panel grid with GPU quads and glyphon text.
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    theme: Theme,
    grid: Option<GridLayout>,
    panels: Vec<Panel>,
    mouse_state: MouseState,
    dividers: DividerSet,
    focused_panel: Option<PanelId>,
    modifiers: ModifiersState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            theme: Theme::default(),
            grid: None,
            panels: Vec::new(),
            mouse_state: MouseState::default(),
            dividers: DividerSet {
                dividers: Vec::new(),
            },
            focused_panel: Some(PanelId(0)),
            modifiers: ModifiersState::empty(),
        }
    }
}

impl App {
    /// Process an InputAction, applying it to the grid and panels.
    fn process_action(&mut self, action: InputAction) {
        match action {
            InputAction::DividerDragMove { delta_pixels } => {
                if let (Some(grid), Some((div_idx, orientation))) = (
                    self.grid.as_mut(),
                    self.mouse_state.divider_drag_info(),
                ) {
                    let window = self.window.as_ref();
                    let total_size = match (orientation, window) {
                        (Orientation::Vertical, Some(w)) => w.inner_size().width as f32,
                        (Orientation::Horizontal, Some(w)) => {
                            w.inner_size().height as f32 - TITLE_BAR_HEIGHT
                        }
                        _ => return,
                    };
                    divider::apply_divider_drag(
                        grid,
                        orientation,
                        div_idx,
                        delta_pixels,
                        total_size,
                    );
                    self.recompute_layout();
                }
            }
            InputAction::DividerDragEnd => {
                // Drag end is handled by MouseState state transition
            }
            InputAction::DividerDragStart { .. } => {
                // Drag start is handled by MouseState state transition
            }
            InputAction::PanelSplitHorizontal { panel_id } => {
                if let Some(grid) = self.grid.as_mut() {
                    if let Some(new_id) =
                        operations::split_panel(grid, panel_id, SplitDirection::Horizontal)
                    {
                        let panel = Panel::new_placeholder(new_id);
                        self.panels.push(panel);
                        self.recompute_layout();
                    }
                }
            }
            InputAction::PanelSplitVertical { panel_id } => {
                if let Some(grid) = self.grid.as_mut() {
                    if let Some(new_id) =
                        operations::split_panel(grid, panel_id, SplitDirection::Vertical)
                    {
                        let panel = Panel::new_placeholder(new_id);
                        self.panels.push(panel);
                        self.recompute_layout();
                    }
                }
            }
            InputAction::PanelClose { panel_id } => {
                if let Some(grid) = self.grid.as_mut() {
                    if operations::close_panel(grid, panel_id) {
                        self.panels.retain(|p| p.id != panel_id);
                        // If focused panel was closed, focus first remaining panel
                        if self.focused_panel == Some(panel_id) {
                            self.focused_panel =
                                grid.panel_nodes().first().map(|(_, id)| *id);
                        }
                        self.recompute_layout();
                    }
                }
            }
            InputAction::PanelSwapStart { .. } => {
                // Swap start tracked by MouseState
            }
            InputAction::PanelSwapDrop { source_panel_id, target_panel_id } => {
                if let Some(grid) = self.grid.as_mut() {
                    operations::swap_panels(grid, source_panel_id, target_panel_id);
                    // Also swap in the panels vec
                    let pos_a = self.panels.iter().position(|p| p.id == source_panel_id);
                    let pos_b =
                        self.panels.iter().position(|p| p.id == target_panel_id);
                    if let (Some(a), Some(b)) = (pos_a, pos_b) {
                        self.panels.swap(a, b);
                    }
                }
            }
            InputAction::PanelToggleFullscreen { panel_id } => {
                if let Some(grid) = self.grid.as_mut() {
                    operations::toggle_fullscreen(grid, panel_id);
                    self.recompute_layout();
                }
            }
            InputAction::ContextMenu { .. } => {
                // Reserved for future use
            }
            InputAction::SetCursor(style) => {
                if let Some(window) = &self.window {
                    let icon = match style {
                        CursorStyle::ColResize => CursorIcon::ColResize,
                        CursorStyle::RowResize => CursorIcon::RowResize,
                        CursorStyle::Default => CursorIcon::Default,
                    };
                    window.set_cursor(icon);
                }
            }
            InputAction::FocusPanel { panel_id } => {
                self.focused_panel = Some(panel_id);
            }
        }
    }

    /// Recompute grid layout and divider positions.
    fn recompute_layout(&mut self) {
        if let (Some(grid), Some(window)) = (self.grid.as_mut(), self.window.as_ref()) {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                let grid_height = size.height as f32 - TITLE_BAR_HEIGHT;
                grid.compute(size.width as f32, grid_height.max(1.0));
                self.dividers =
                    compute_dividers(grid, size.width as f32, grid_height.max(1.0));
            }
        }
    }

    /// Build quad instances for the current frame.
    fn build_quads(&self, width: f32, height: f32) -> Vec<QuadInstance> {
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
        for &(node, panel_id) in grid.panel_nodes() {
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

            // Close button quad (top-right of panel title bar)
            let close_x = px + pw - 40.0;
            let close_y = py_offset + 6.0;
            quads.push(QuadInstance {
                position: [close_x, close_y],
                size: [16.0, 16.0],
                color: [0.3, 0.15, 0.15, 0.6],
                corner_radius: 2.0,
                _padding: 0.0,
            });

            // Fullscreen button quad (next to close button)
            let fs_x = px + pw - 20.0;
            let fs_y = py_offset + 6.0;
            quads.push(QuadInstance {
                position: [fs_x, fs_y],
                size: [16.0, 16.0],
                color: [0.15, 0.15, 0.3, 0.6],
                corner_radius: 2.0,
                _padding: 0.0,
            });

            // Focused panel indicator: subtle top border
            if self.focused_panel == Some(panel_id) {
                quads.push(QuadInstance {
                    position: [px, py_offset],
                    size: [pw, 2.0],
                    color: self.theme.divider_hover,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }
        }

        // Divider quads (D-04: 1px visual width)
        for (i, div) in self.dividers.dividers.iter().enumerate() {
            let is_hovered = self.mouse_state.hovered_divider == Some(i);
            let color = if is_hovered {
                self.theme.divider_hover
            } else {
                self.theme.divider
            };

            match div.orientation {
                Orientation::Vertical => {
                    // Vertical divider: thin line from top to bottom of grid area
                    let grid_height = height - TITLE_BAR_HEIGHT;
                    quads.push(QuadInstance {
                        position: [div.position - DIVIDER_VISUAL_WIDTH / 2.0, TITLE_BAR_HEIGHT],
                        size: [DIVIDER_VISUAL_WIDTH, grid_height],
                        color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                Orientation::Horizontal => {
                    // Horizontal divider: thin line from left to right of grid area
                    quads.push(QuadInstance {
                        position: [0.0, div.position + TITLE_BAR_HEIGHT - DIVIDER_VISUAL_WIDTH / 2.0],
                        size: [width, DIVIDER_VISUAL_WIDTH],
                        color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
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
        for &(node, panel_id) in grid.panel_nodes() {
            let (px, py, pw, ph) = grid.get_panel_rect(node);
            let py_offset = py + TITLE_BAR_HEIGHT;

            // Find the panel data by ID (not by index, since panels may be reordered)
            if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                // Panel title bar label (D-01): type name at top-left of panel
                labels.push(TextLabel {
                    text: panel.panel_type.to_string(),
                    x: px + 8.0,
                    y: py_offset + 4.0,
                    width: pw - 60.0, // Leave room for buttons
                    height: 20.0,
                    font_size: 12.0,
                    color: glyphon::Color::rgba(
                        (self.theme.title_bar_text[0] * 255.0) as u8,
                        (self.theme.title_bar_text[1] * 255.0) as u8,
                        (self.theme.title_bar_text[2] * 255.0) as u8,
                        (self.theme.title_bar_text[3] * 255.0) as u8,
                    ),
                });

                // Close button label "x"
                labels.push(TextLabel {
                    text: "x".to_string(),
                    x: px + pw - 37.0,
                    y: py_offset + 6.0,
                    width: 16.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: glyphon::Color::rgba(200, 200, 200, 255),
                });

                // Fullscreen button label (small square unicode)
                labels.push(TextLabel {
                    text: "\u{25A1}".to_string(), // White square
                    x: px + pw - 17.0,
                    y: py_offset + 6.0,
                    width: 16.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: glyphon::Color::rgba(200, 200, 200, 255),
                });

                // Centered type label in panel body (D-03)
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
            self.dividers =
                compute_dividers(&grid, size.width as f32, grid_height.max(1.0));
        }

        // Create the initial placeholder panel
        let panel = Panel::new_placeholder(PanelId(0));
        self.panels = vec![panel];
        self.focused_panel = Some(PanelId(0));

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

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::CursorMoved { position, .. } => {
                // Route to mouse state; it will produce InputActions
                if let Some(grid) = &self.grid {
                    let actions = self.mouse_state.on_cursor_moved(
                        position.x,
                        position.y,
                        &self.dividers,
                        grid,
                        TITLE_BAR_HEIGHT,
                    );
                    // Collect actions first, then process them (avoids borrow issues)
                    let actions: Vec<_> = actions;
                    for action in actions {
                        self.process_action(action);
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(grid) = &self.grid {
                    let actions = match state {
                        ElementState::Pressed => self.mouse_state.on_mouse_press(
                            button,
                            &self.dividers,
                            grid,
                            TITLE_BAR_HEIGHT,
                        ),
                        ElementState::Released => self.mouse_state.on_mouse_release(
                            button,
                            grid,
                            TITLE_BAR_HEIGHT,
                        ),
                    };
                    let actions: Vec<_> = actions;
                    for action in actions {
                        self.process_action(action);
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(action) = keyboard::handle_key_event(
                    &event,
                    &self.modifiers,
                    self.focused_panel,
                ) {
                    self.process_action(action);
                }
            }

            WindowEvent::Resized(size) => {
                // Guard against zero dimensions (Pitfall 2 / T-01-01 mitigation)
                if size.width > 0 && size.height > 0 {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }

                    // Recompute grid layout and dividers for new size
                    self.recompute_layout();

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
