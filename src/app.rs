use std::sync::Arc;
use tracing::{debug, info, warn};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window, WindowId};

use alacritty_terminal::grid::Dimensions as TermDimTrait;
use crate::grid::divider::{
    self, compute_dividers, DividerSet, Orientation, DIVIDER_VISUAL_WIDTH,
};
use crate::grid::layout::GridLayout;
use crate::grid::operations::{self, SplitDirection};
use crate::grid::panel::{Panel, PanelId, PanelType};
use crate::input::keyboard;
use crate::input::mouse::MouseState;
use crate::input::{CursorStyle, InputAction};
use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::renderer::Renderer;
use crate::terminal::renderer::TerminalRenderer;
use crate::terminal::TerminalManager;
use crate::theme::Theme;
use crate::window::create_window;

/// Height of the custom title bar area in pixels.
const TITLE_BAR_HEIGHT: f32 = 38.0;

/// Height of the panel title bar area in pixels.
const PANEL_TITLE_HEIGHT: f32 = 28.0;

/// Main application state.
///
/// Owns the window, renderer, grid layout, panels, theme, input state,
/// terminal manager, and terminal renderer.
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
    /// Manages all terminal instances (PTY lifecycle, event draining).
    terminal_manager: Option<TerminalManager>,
    /// GPU terminal renderer (snapshot + buffer building, quad generation).
    terminal_renderer: TerminalRenderer,
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
            terminal_manager: None,
            terminal_renderer: TerminalRenderer::new(),
        }
    }
}

impl App {
    /// Get the PanelType for the focused panel.
    fn focused_panel_type(&self) -> Option<PanelType> {
        self.focused_panel.and_then(|pid| {
            self.panels
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.panel_type)
        })
    }

    /// Process an InputAction, applying it to the grid, panels, and terminals.
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
                // Destroy terminal if this is a terminal panel
                if let Some(tm) = &mut self.terminal_manager {
                    tm.destroy_terminal(&panel_id);
                }
                if let Some(grid) = self.grid.as_mut() {
                    if operations::close_panel(grid, panel_id) {
                        self.panels.retain(|p| p.id != panel_id);
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
            InputAction::PanelSwapDrop {
                source_panel_id,
                target_panel_id,
            } => {
                if let Some(grid) = self.grid.as_mut() {
                    operations::swap_panels(grid, source_panel_id, target_panel_id);
                    let pos_a = self
                        .panels
                        .iter()
                        .position(|p| p.id == source_panel_id);
                    let pos_b = self
                        .panels
                        .iter()
                        .position(|p| p.id == target_panel_id);
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

            // === Terminal actions ===
            InputAction::TerminalInput { panel_id, bytes } => {
                let mut should_close = false;
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        // Per D-03: if terminal exited, any key closes the panel
                        if ts.exited {
                            should_close = true;
                        } else {
                            ts.write_to_pty(&bytes);
                            ts.reset_cursor_blink();
                        }
                    }
                }
                if should_close {
                    self.process_action(InputAction::PanelClose { panel_id });
                    return;
                }
            }
            InputAction::TerminalCopy { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        let term = ts.term.lock();
                        // D-13: if selection exists, copy; otherwise send SIGINT
                        if term.selection.is_some() {
                            if let Some(text) =
                                crate::terminal::selection::selection_to_string(&term)
                            {
                                drop(term); // Release lock before clipboard access
                                if let Ok(mut ctx) = copypasta::ClipboardContext::new() {
                                    use copypasta::ClipboardProvider;
                                    let _ = ctx.set_contents(text);
                                }
                                // Trigger copy flash (D-15)
                                ts.trigger_copy_flash();
                                // Clear selection after flash starts
                                let mut term = ts.term.lock();
                                crate::terminal::selection::clear_selection(&mut term);
                            } else {
                                drop(term);
                            }
                        } else {
                            // No selection: send SIGINT (Ctrl+C = 0x03)
                            drop(term);
                            ts.write_to_pty(&[0x03]);
                        }
                    }
                }
            }
            InputAction::TerminalPaste { panel_id } => {
                if let Some(tm) = &self.terminal_manager {
                    if let Some(ts) = tm.get(&panel_id) {
                        if let Ok(mut ctx) = copypasta::ClipboardContext::new() {
                            use copypasta::ClipboardProvider;
                            if let Ok(text) = ctx.get_contents() {
                                // Check if bracketed paste mode is enabled
                                let mode = *ts.term.lock().mode();
                                if mode.contains(
                                    alacritty_terminal::term::TermMode::BRACKETED_PASTE,
                                ) {
                                    ts.write_to_pty(b"\x1b[200~");
                                    ts.write_to_pty(text.as_bytes());
                                    ts.write_to_pty(b"\x1b[201~");
                                } else {
                                    ts.write_to_pty(text.as_bytes());
                                }
                            }
                        }
                    }
                }
            }
            InputAction::TerminalFontSizeChange { panel_id, delta } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        let new_size = (ts.font_size + delta).clamp(8.0, 32.0);
                        ts.font_size = new_size;
                        // Recalculate cell dimensions
                        ts.cell_width = new_size * 0.6;
                        ts.cell_height = new_size * 1.3;
                        // Update terminal renderer
                        self.terminal_renderer.font_size = new_size;
                        self.terminal_renderer.cell_width = ts.cell_width;
                        self.terminal_renderer.cell_height = ts.cell_height;
                        // Resize terminal grid and notify PTY
                        if let Some(grid) = &self.grid {
                            if let Some(node_id) = grid.find_node(panel_id) {
                                let (_, _, pw, ph) = grid.get_panel_rect(node_id);
                                let cols =
                                    (pw / ts.cell_width).max(2.0) as usize;
                                let rows = ((ph - PANEL_TITLE_HEIGHT) / ts.cell_height)
                                    .max(1.0) as usize;
                                let dims =
                                    crate::terminal::state::TermDimensions { cols, rows };
                                ts.term.lock().resize(dims);
                                let window_size =
                                    alacritty_terminal::event::WindowSize {
                                        num_lines: rows as u16,
                                        num_cols: cols as u16,
                                        cell_width: ts.cell_width.round() as u16,
                                        cell_height: ts.cell_height.round() as u16,
                                    };
                                let _ = ts.event_loop_sender.send(
                                    alacritty_terminal::event_loop::Msg::Resize(
                                        window_size,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            InputAction::CreateTerminal => {
                // Split the focused panel and create a terminal in the new slot
                if let Some(focused_id) = self.focused_panel {
                    if let Some(grid) = self.grid.as_mut() {
                        if let Some(new_id) =
                            operations::split_panel(grid, focused_id, SplitDirection::Horizontal)
                        {
                            let panel = Panel::new_terminal(new_id);
                            self.panels.push(panel);
                            self.focused_panel = Some(new_id);
                            self.recompute_layout();

                            // Create terminal in the new panel
                            if let Some(tm) = &mut self.terminal_manager {
                                if let Some(grid) = &self.grid {
                                    if let Some(node_id) = grid.find_node(new_id) {
                                        let (_, _, pw, ph) = grid.get_panel_rect(node_id);
                                        let cw = self.terminal_renderer.cell_width;
                                        let ch = self.terminal_renderer.cell_height;
                                        let cols = (pw / cw).max(2.0) as usize;
                                        let rows = ((ph - PANEL_TITLE_HEIGHT) / ch)
                                            .max(1.0)
                                            as usize;
                                        if let Err(e) =
                                            tm.create_terminal(new_id, cols, rows)
                                        {
                                            warn!(
                                                "Failed to create terminal: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            InputAction::TerminalScroll { panel_id, delta } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.scroll(delta);
                    }
                }
            }

            InputAction::TerminalSelectionStart {
                panel_id,
                x,
                y,
                block,
            } => {
                if let (Some(tm), Some(grid)) = (&mut self.terminal_manager, &self.grid) {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        // Check if click is on the "New output" indicator (D-10)
                        if ts.has_new_output_while_scrolled {
                            if let Some(node) = grid.find_node(panel_id) {
                                let (px, py, pw, ph) = grid.get_panel_rect(node);
                                let py_offset = py + TITLE_BAR_HEIGHT;
                                let indicator_w = 120.0_f32;
                                let indicator_h = 22.0_f32;
                                let indicator_x = px + pw / 2.0 - indicator_w / 2.0;
                                let indicator_y = py_offset + ph - indicator_h - 4.0;
                                if x >= indicator_x
                                    && x <= indicator_x + indicator_w
                                    && y >= indicator_y
                                    && y <= indicator_y + indicator_h
                                {
                                    ts.scroll_to_bottom();
                                    return;
                                }
                            }
                        }

                        if let Some(node) = grid.find_node(panel_id) {
                            let (px, py, _pw, _ph) = grid.get_panel_rect(node);
                            let viewport_x = px;
                            let viewport_y =
                                py + TITLE_BAR_HEIGHT + PANEL_TITLE_HEIGHT;
                            let display_offset =
                                ts.term.lock().grid().display_offset();
                            let point = crate::terminal::selection::pixel_to_point(
                                x,
                                y,
                                viewport_x,
                                viewport_y,
                                ts.cell_width,
                                ts.cell_height,
                                display_offset,
                            );
                            let click_count = self.mouse_state.click_count;
                            let mut term = ts.term.lock();
                            crate::terminal::selection::start_selection(
                                &mut term,
                                point,
                                click_count,
                                block,
                            );
                        }
                    }
                }
            }

            InputAction::TerminalSelectionUpdate { panel_id, x, y } => {
                if let (Some(tm), Some(grid)) = (&mut self.terminal_manager, &self.grid) {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        if let Some(node) = grid.find_node(panel_id) {
                            let (px, py, _pw, _ph) = grid.get_panel_rect(node);
                            let viewport_x = px;
                            let viewport_y =
                                py + TITLE_BAR_HEIGHT + PANEL_TITLE_HEIGHT;
                            let display_offset =
                                ts.term.lock().grid().display_offset();
                            let point = crate::terminal::selection::pixel_to_point(
                                x,
                                y,
                                viewport_x,
                                viewport_y,
                                ts.cell_width,
                                ts.cell_height,
                                display_offset,
                            );
                            let mut term = ts.term.lock();
                            crate::terminal::selection::update_selection(
                                &mut term, point,
                            );
                        }
                    }
                }
            }

            InputAction::TerminalSelectionEnd { panel_id } => {
                // Selection stays visible -- cleared on next click or Cmd+C
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        let mut term = ts.term.lock();
                        crate::terminal::selection::end_selection(&mut term);
                    }
                }
            }

            InputAction::TerminalSearchOpen { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.search.open();
                    }
                }
            }
            InputAction::TerminalSearchClose { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.search.close();
                    }
                }
            }
            InputAction::TerminalSearchChar { panel_id, ch } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        if let crate::terminal::search::SearchState::Open {
                            query, ..
                        } = &ts.search
                        {
                            let mut new_query = query.clone();
                            new_query.push(ch);
                            let mut term = ts.term.lock();
                            ts.search.update_query(&mut term, &new_query);
                        }
                    }
                }
            }
            InputAction::TerminalSearchBackspace { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        if let crate::terminal::search::SearchState::Open {
                            query, ..
                        } = &ts.search
                        {
                            let mut new_query = query.clone();
                            new_query.pop();
                            let mut term = ts.term.lock();
                            ts.search.update_query(&mut term, &new_query);
                        }
                    }
                }
            }
            InputAction::TerminalSearchNext { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        let mut term = ts.term.lock();
                        ts.search.next_match(&mut term);
                    }
                }
            }
            InputAction::TerminalSearchPrev { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        let mut term = ts.term.lock();
                        ts.search.prev_match(&mut term);
                    }
                }
            }
            InputAction::TerminalSearchUpdate { panel_id, query } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        let mut term = ts.term.lock();
                        ts.search.update_query(&mut term, &query);
                    }
                }
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

    /// Resize all terminals to match their panel dimensions and notify PTY.
    fn resize_terminals(&mut self) {
        if let (Some(grid), Some(tm)) = (&self.grid, &mut self.terminal_manager) {
            for &(node, panel_id) in grid.panel_nodes() {
                if let Some(ts) = tm.get_mut(&panel_id) {
                    let (_, _, pw, ph) = grid.get_panel_rect(node);
                    let cols = (pw / ts.cell_width).max(2.0) as usize;
                    let rows =
                        ((ph - PANEL_TITLE_HEIGHT) / ts.cell_height).max(1.0) as usize;
                    let dims = crate::terminal::state::TermDimensions { cols, rows };
                    ts.term.lock().resize(dims);
                    // CRITICAL: Notify PTY of new window size so it sends SIGWINCH
                    let window_size = alacritty_terminal::event::WindowSize {
                        num_lines: rows as u16,
                        num_cols: cols as u16,
                        cell_width: ts.cell_width.round() as u16,
                        cell_height: ts.cell_height.round() as u16,
                    };
                    let _ = ts.event_loop_sender.send(
                        alacritty_terminal::event_loop::Msg::Resize(window_size),
                    );
                }
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
            let py_offset = py + TITLE_BAR_HEIGHT;

            // Panel background quad
            quads.push(QuadInstance {
                position: [px, py_offset],
                size: [pw, ph],
                color: self.theme.panel_background,
                corner_radius: 0.0,
                _padding: 0.0,
            });

            // Close button quad
            let close_x = px + pw - 40.0;
            let close_y = py_offset + 6.0;
            quads.push(QuadInstance {
                position: [close_x, close_y],
                size: [16.0, 16.0],
                color: [0.3, 0.15, 0.15, 0.6],
                corner_radius: 2.0,
                _padding: 0.0,
            });

            // Fullscreen button quad
            let fs_x = px + pw - 20.0;
            let fs_y = py_offset + 6.0;
            quads.push(QuadInstance {
                position: [fs_x, fs_y],
                size: [16.0, 16.0],
                color: [0.15, 0.15, 0.3, 0.6],
                corner_radius: 2.0,
                _padding: 0.0,
            });

            // Focused panel indicator
            if self.focused_panel == Some(panel_id) {
                quads.push(QuadInstance {
                    position: [px, py_offset],
                    size: [pw, 2.0],
                    color: self.theme.divider_hover,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }

            // Terminal-specific quads (cell backgrounds, cursor)
            if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                if panel.panel_type == PanelType::Terminal {
                    if let Some(tm) = &self.terminal_manager {
                        if let Some(ts) = tm.get(&panel_id) {
                            let content_y = py_offset + PANEL_TITLE_HEIGHT;
                            let content_h = ph - PANEL_TITLE_HEIGHT;
                            let snapshot =
                                TerminalRenderer::snapshot(&ts.term);
                            let term_quads =
                                self.terminal_renderer.build_terminal_quads(
                                    &snapshot,
                                    px,
                                    content_y,
                                    pw,
                                    content_h,
                                    self.theme.panel_background,
                                    ts.cursor_blink_visible,
                                );
                            quads.extend(term_quads);

                            // Selection highlight and copy flash quads
                            {
                                let term = ts.term.lock();
                                let flash_opacity = ts.copy_flash_opacity();
                                let sel_quads =
                                    self.terminal_renderer.build_selection_quads(
                                        &term,
                                        px,
                                        content_y,
                                        ts.cell_width,
                                        ts.cell_height,
                                        flash_opacity,
                                    );
                                quads.extend(sel_quads);
                            }

                            // "New output" indicator (D-10): show when scrolled up and new output arrived
                            if ts.has_new_output_while_scrolled {
                                let indicator_w = 120.0_f32;
                                let indicator_h = 22.0_f32;
                                let indicator_x = px + pw / 2.0 - indicator_w / 2.0;
                                let indicator_y = py_offset + ph - indicator_h - 4.0;
                                quads.push(QuadInstance {
                                    position: [indicator_x, indicator_y],
                                    size: [indicator_w, indicator_h],
                                    color: [0.2, 0.4, 0.8, 0.7],
                                    corner_radius: 4.0,
                                    _padding: 0.0,
                                });
                            }

                            // Search overlay quads (D-09)
                            if ts.search.is_open() {
                                // Search bar background
                                let bar_quads = self
                                    .terminal_renderer
                                    .build_search_bar_quads(
                                        px,
                                        content_y,
                                        pw,
                                    );
                                quads.extend(bar_quads);

                                // Search match highlights
                                let term = ts.term.lock();
                                let display_offset =
                                    term.grid().display_offset();
                                let screen_lines = term.screen_lines();
                                drop(term);

                                let search_quads = self
                                    .terminal_renderer
                                    .build_search_quads(
                                        ts.search.match_positions(),
                                        ts.search.current_match_index(),
                                        px,
                                        content_y,
                                        ts.cell_width,
                                        ts.cell_height,
                                        display_offset,
                                        screen_lines,
                                    );
                                quads.extend(search_quads);
                            }
                        }
                    }
                }
            }
        }

        // Divider quads
        for (i, div) in self.dividers.dividers.iter().enumerate() {
            let is_hovered = self.mouse_state.hovered_divider == Some(i);
            let color = if is_hovered {
                self.theme.divider_hover
            } else {
                self.theme.divider
            };

            match div.orientation {
                Orientation::Vertical => {
                    let grid_height = height - TITLE_BAR_HEIGHT;
                    quads.push(QuadInstance {
                        position: [
                            div.position - DIVIDER_VISUAL_WIDTH / 2.0,
                            TITLE_BAR_HEIGHT,
                        ],
                        size: [DIVIDER_VISUAL_WIDTH, grid_height],
                        color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                Orientation::Horizontal => {
                    quads.push(QuadInstance {
                        position: [
                            0.0,
                            div.position + TITLE_BAR_HEIGHT
                                - DIVIDER_VISUAL_WIDTH / 2.0,
                        ],
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

            if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                // Panel title bar label
                labels.push(TextLabel {
                    text: panel.panel_type.to_string(),
                    x: px + 8.0,
                    y: py_offset + 4.0,
                    width: pw - 60.0,
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

                // Fullscreen button label
                labels.push(TextLabel {
                    text: "\u{25A1}".to_string(),
                    x: px + pw - 17.0,
                    y: py_offset + 6.0,
                    width: 16.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: glyphon::Color::rgba(200, 200, 200, 255),
                });

                // Terminal panels: show "Process exited" if shell exited (D-03)
                // Non-terminal panels: show centered type label
                if panel.panel_type == PanelType::Terminal {
                    if let Some(tm) = &self.terminal_manager {
                        if let Some(ts) = tm.get(&panel_id) {
                            if ts.exited {
                                let exit_msg = match ts.exit_code {
                                    Some(code) => format!("Process exited [{}]", code),
                                    None => "Process exited".to_string(),
                                };
                                let center_y = py_offset + ph / 2.0 - 7.0;
                                labels.push(TextLabel {
                                    text: exit_msg,
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
                            // "New output" indicator label (D-10)
                            if ts.has_new_output_while_scrolled {
                                let indicator_w = 120.0_f32;
                                let indicator_h = 22.0_f32;
                                let indicator_x = px + pw / 2.0 - indicator_w / 2.0;
                                let indicator_y = py_offset + ph - indicator_h - 4.0;
                                labels.push(TextLabel {
                                    text: "New output \u{25BC}".to_string(),
                                    x: indicator_x + 10.0,
                                    y: indicator_y + 3.0,
                                    width: indicator_w - 20.0,
                                    height: 16.0,
                                    font_size: 11.0,
                                    color: glyphon::Color::rgba(240, 240, 240, 255),
                                });
                            }
                            // Search overlay labels (D-09)
                            if ts.search.is_open() {
                                let content_y =
                                    py_offset + PANEL_TITLE_HEIGHT;
                                let bar_width = 250.0_f32.min(pw - 20.0);
                                let bar_x = px + pw - bar_width - 10.0;
                                let bar_y = content_y + 5.0;

                                // Search query text
                                let query_text = if ts.search.query().is_empty() {
                                    "Search...".to_string()
                                } else {
                                    ts.search.query().to_string()
                                };
                                labels.push(TextLabel {
                                    text: query_text,
                                    x: bar_x + 8.0,
                                    y: bar_y + 6.0,
                                    width: bar_width - 80.0,
                                    height: 16.0,
                                    font_size: 12.0,
                                    color: glyphon::Color::rgba(220, 220, 220, 255),
                                });

                                // Match count "N of M"
                                if let Some((current, total)) =
                                    ts.search.match_info()
                                {
                                    labels.push(TextLabel {
                                        text: format!("{} of {}", current, total),
                                        x: bar_x + bar_width - 70.0,
                                        y: bar_y + 6.0,
                                        width: 60.0,
                                        height: 16.0,
                                        font_size: 11.0,
                                        color: glyphon::Color::rgba(
                                            160, 160, 160, 255,
                                        ),
                                    });
                                }
                            }
                            // Terminal text is rendered via terminal_renderer, not labels
                        }
                    }
                } else {
                    // Centered type label in panel body (D-03) for non-terminal panels
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

        let mut renderer = Renderer::new(window.clone());

        // Re-apply traffic light positioning after renderer init
        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::setup_custom_title_bar(&window);
        }

        // Load JetBrains Mono font into the text engine (D-05)
        let font_data = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
        renderer.load_font_data(font_data.to_vec());

        // Compute cell dimensions from font metrics
        let (cell_width, cell_height) = TerminalRenderer::compute_cell_dimensions(
            renderer.text_engine_mut().font_system_mut(),
            self.terminal_renderer.font_size,
        );
        self.terminal_renderer.cell_width = cell_width;
        self.terminal_renderer.cell_height = cell_height;
        debug!(
            "Terminal cell dimensions: {}x{} (font_size={})",
            cell_width, cell_height, self.terminal_renderer.font_size
        );

        // Initialize grid with a single panel filling the window
        let mut grid = GridLayout::new_single_panel();
        let size = window.inner_size();
        if size.width > 0 && size.height > 0 {
            let grid_height = size.height as f32 - TITLE_BAR_HEIGHT;
            grid.compute(size.width as f32, grid_height.max(1.0));
            self.dividers =
                compute_dividers(&grid, size.width as f32, grid_height.max(1.0));
        }

        // Create the initial terminal panel (not placeholder)
        let panel = Panel::new_terminal(PanelId(0));
        self.panels = vec![panel];
        self.focused_panel = Some(PanelId(0));

        // Create terminal manager with current directory as project dir (D-02)
        let project_dir =
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        let mut tm = TerminalManager::new(project_dir);

        // Create terminal in the initial panel
        let (_, _, pw, ph) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let cols = (pw / cell_width).max(2.0) as usize;
        let rows = ((ph - PANEL_TITLE_HEIGHT) / cell_height).max(1.0) as usize;
        if let Err(e) = tm.create_terminal(PanelId(0), cols, rows) {
            warn!("Failed to create initial terminal: {}", e);
        } else {
            // Update terminal state with computed cell dimensions
            if let Some(ts) = tm.get_mut(&PanelId(0)) {
                ts.cell_width = cell_width;
                ts.cell_height = cell_height;
            }
        }

        self.terminal_manager = Some(tm);
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.grid = Some(grid);

        info!("Application initialization complete with terminal");
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
                if let Some(grid) = &self.grid {
                    let actions = self.mouse_state.on_cursor_moved(
                        position.x,
                        position.y,
                        &self.dividers,
                        grid,
                        TITLE_BAR_HEIGHT,
                    );
                    let actions: Vec<_> = actions;
                    for action in actions {
                        self.process_action(action);
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(grid) = &self.grid {
                    let panels = &self.panels;
                    let panel_types = |pid: PanelId| -> Option<PanelType> {
                        panels.iter().find(|p| p.id == pid).map(|p| p.panel_type)
                    };
                    let actions = match state {
                        ElementState::Pressed => self.mouse_state.on_mouse_press(
                            button,
                            &self.dividers,
                            grid,
                            TITLE_BAR_HEIGHT,
                            &panel_types,
                            &self.modifiers,
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

            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => (y * 3.0) as i32,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
                };
                if lines != 0 {
                    if let Some(grid) = &self.grid {
                        let panels = &self.panels;
                        let panel_types = |pid: PanelId| -> Option<PanelType> {
                            panels.iter().find(|p| p.id == pid).map(|p| p.panel_type)
                        };
                        let actions = self.mouse_state.on_mouse_wheel(
                            lines as f32,
                            grid,
                            TITLE_BAR_HEIGHT,
                            &panel_types,
                        );
                        for action in actions {
                            self.process_action(action);
                        }
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let panel_type = self.focused_panel_type();
                let search_open = self
                    .focused_panel
                    .and_then(|pid| {
                        self.terminal_manager
                            .as_ref()?
                            .get(&pid)
                            .map(|ts| ts.search.is_open())
                    })
                    .unwrap_or(false);
                let term_mode = self
                    .focused_panel
                    .and_then(|pid| self.terminal_manager.as_ref()?.get(&pid))
                    .map(|ts| *ts.term.lock().mode())
                    .unwrap_or(alacritty_terminal::term::TermMode::empty());
                if let Some(action) = keyboard::handle_key_event(
                    &event,
                    &self.modifiers,
                    self.focused_panel,
                    panel_type,
                    search_open,
                    term_mode,
                ) {
                    self.process_action(action);
                }
            }

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }
                    self.recompute_layout();
                    // Resize all terminals and notify PTY (SIGWINCH)
                    self.resize_terminals();

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

                    // Build frame data
                    let quads = self.build_quads(vw, vh);
                    let labels = self.build_labels(vw, vh);

                    // Prepare terminal text (snapshot + buffer building)
                    // This must happen before renderer.render() since it sets
                    // terminal buffers on the text engine.
                    if let Some(renderer) = &mut self.renderer {
                        let mut terminal_buffers = Vec::new();
                        let mut terminal_metas = Vec::new();

                        if let Some(tm) = &self.terminal_manager {
                            if let Some(grid) = &self.grid {
                                let font_system =
                                    renderer.text_engine_mut().font_system_mut();
                                for &(node, panel_id) in grid.panel_nodes() {
                                    if let Some(ts) = tm.get(&panel_id) {
                                        let is_terminal = self
                                            .panels
                                            .iter()
                                            .any(|p| {
                                                p.id == panel_id
                                                    && p.panel_type
                                                        == PanelType::Terminal
                                            });
                                        if is_terminal && !ts.exited {
                                            let (px, py, pw, ph) =
                                                grid.get_panel_rect(node);
                                            let content_y =
                                                py + TITLE_BAR_HEIGHT + PANEL_TITLE_HEIGHT;
                                            let content_h = ph - PANEL_TITLE_HEIGHT;

                                            let snapshot =
                                                TerminalRenderer::snapshot(&ts.term);
                                            let (bufs, metas) = self
                                                .terminal_renderer
                                                .prepare_buffers(
                                                    font_system,
                                                    &snapshot,
                                                    px,
                                                    content_y,
                                                    pw,
                                                    content_h,
                                                );
                                            terminal_buffers.extend(bufs);
                                            terminal_metas.extend(metas);
                                        }
                                    }
                                }
                            }
                        }

                        renderer
                            .text_engine_mut()
                            .set_terminal_buffers(terminal_buffers, terminal_metas);

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
                                warn!(
                                    "Surface lost -- will attempt recovery next frame"
                                );
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Drain terminal events and update cursor blinks
        if let Some(tm) = &mut self.terminal_manager {
            tm.drain_all_events();
            tm.update_all_cursor_blinks();
            // Clear expired copy flash animations (D-15)
            for ts in tm.terminals_mut().values_mut() {
                ts.clear_expired_flash();
            }
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
