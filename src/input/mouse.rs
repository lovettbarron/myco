use std::time::Instant;
use winit::event::MouseButton;
use winit::keyboard::ModifiersState;

use crate::grid::divider::{hit_test_divider, DividerSet};
use crate::grid::layout::GridLayout;
use crate::grid::panel::PanelType;
use crate::grid::{Orientation, PanelId};

use super::{CursorStyle, InputAction};

/// Height of a panel's title bar region in pixels (top of panel rect).
const PANEL_TITLE_BAR_HEIGHT: f32 = 28.0;

/// Close button dimensions and offset from panel right edge.
const CLOSE_BUTTON_SIZE: f32 = 16.0;
const CLOSE_BUTTON_RIGHT_OFFSET: f32 = 40.0;
const CLOSE_BUTTON_TOP_OFFSET: f32 = 6.0;

/// Fullscreen button dimensions and offset from panel right edge.
const FULLSCREEN_BUTTON_RIGHT_OFFSET: f32 = 20.0;
const FULLSCREEN_BUTTON_TOP_OFFSET: f32 = 6.0;

/// Drag state machine for mouse interactions.
#[derive(Debug)]
#[allow(dead_code)]
pub enum DragState {
    /// No drag in progress.
    Idle,
    /// Dragging a divider to resize panels.
    DraggingDivider {
        divider_index: usize,
        orientation: Orientation,
        start_pos: f64,
        last_pos: f64,
    },
    /// Dragging a panel title bar for swap.
    DraggingTitleBar {
        panel_id: PanelId,
        start_pos: (f64, f64),
    },
    /// Dragging to select text in a terminal panel.
    DraggingTerminalSelection {
        panel_id: PanelId,
    },
}

/// Mouse input state tracking.
pub struct MouseState {
    /// Current drag state.
    pub drag: DragState,
    /// Current cursor X position.
    pub cursor_x: f64,
    /// Current cursor Y position.
    pub cursor_y: f64,
    /// Index of the currently hovered divider, if any.
    pub hovered_divider: Option<usize>,
    /// ID of the currently hovered panel, if any.
    pub hovered_panel: Option<PanelId>,
    /// Timestamp of last click for double/triple click detection.
    pub last_click_time: Instant,
    /// Position of last click.
    pub last_click_pos: (f64, f64),
    /// Click count (1=single, 2=double, 3=triple).
    pub click_count: u8,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            drag: DragState::Idle,
            cursor_x: 0.0,
            cursor_y: 0.0,
            hovered_divider: None,
            hovered_panel: None,
            last_click_time: Instant::now(),
            last_click_pos: (0.0, 0.0),
            click_count: 0,
        }
    }
}

impl MouseState {
    /// Handle cursor movement.
    ///
    /// Updates cursor position, processes drag state, and hit-tests dividers/panels.
    pub fn on_cursor_moved(
        &mut self,
        x: f64,
        y: f64,
        dividers: &DividerSet,
        grid: &GridLayout,
        title_bar_height: f32,
    ) -> Vec<InputAction> {
        let mut actions = Vec::new();
        self.cursor_x = x;
        self.cursor_y = y;

        match &mut self.drag {
            DragState::DraggingDivider {
                orientation,
                last_pos,
                ..
            } => {
                let current = match orientation {
                    Orientation::Vertical => x,
                    Orientation::Horizontal => y,
                };
                let delta = current - *last_pos;
                *last_pos = current;
                actions.push(InputAction::DividerDragMove {
                    delta_pixels: delta as f32,
                });
            }
            DragState::Idle => {
                // Hit-test dividers (positions are relative to grid, which is offset by title_bar_height)
                let grid_y = (y as f32) - title_bar_height;
                let hit = hit_test_divider(dividers, x as f32, grid_y);

                if let Some((idx, orientation)) = hit {
                    self.hovered_divider = Some(idx);
                    let cursor = match orientation {
                        Orientation::Vertical => CursorStyle::ColResize,
                        Orientation::Horizontal => CursorStyle::RowResize,
                    };
                    actions.push(InputAction::SetCursor(cursor));
                } else {
                    if self.hovered_divider.is_some() {
                        self.hovered_divider = None;
                        actions.push(InputAction::SetCursor(CursorStyle::Default));
                    }
                    // Hit-test panels for hover state
                    self.hovered_panel =
                        find_panel_at(grid, x as f32, y as f32, title_bar_height);
                }
            }
            DragState::DraggingTitleBar { .. } => {
                // While dragging title bar, just update position (swap happens on release)
            }
            DragState::DraggingTerminalSelection { panel_id } => {
                let pid = *panel_id;
                actions.push(InputAction::TerminalSelectionUpdate {
                    panel_id: pid,
                    x: x as f32,
                    y: y as f32,
                });
            }
        }

        actions
    }

    /// Handle mouse button press.
    ///
    /// Hit-testing order: close/fullscreen buttons first, then dividers,
    /// then panel title bars, then panel bodies (including terminal selection).
    pub fn on_mouse_press(
        &mut self,
        button: MouseButton,
        dividers: &DividerSet,
        grid: &GridLayout,
        title_bar_height: f32,
        panel_types: &dyn Fn(PanelId) -> Option<PanelType>,
        modifiers: &ModifiersState,
    ) -> Vec<InputAction> {
        let mut actions = Vec::new();
        let x = self.cursor_x;
        let y = self.cursor_y;

        match button {
            MouseButton::Left => {
                // Update click counting for double/triple click detection
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_click_time);
                let distance = ((x - self.last_click_pos.0).powi(2)
                    + (y - self.last_click_pos.1).powi(2))
                .sqrt();
                if elapsed < std::time::Duration::from_millis(500) && distance < 5.0 {
                    self.click_count = (self.click_count % 3) + 1; // cycles 1->2->3->1
                } else {
                    self.click_count = 1;
                }
                self.last_click_time = now;
                self.last_click_pos = (x, y);

                // 1. Hit-test close and fullscreen buttons first
                if let Some(action) =
                    hit_test_buttons(grid, x as f32, y as f32, title_bar_height)
                {
                    actions.push(action);
                    return actions;
                }

                // 2. Hit-test dividers
                let grid_y = (y as f32) - title_bar_height;
                if let Some((idx, orientation)) = hit_test_divider(dividers, x as f32, grid_y)
                {
                    let start = match orientation {
                        Orientation::Vertical => x,
                        Orientation::Horizontal => y,
                    };
                    self.drag = DragState::DraggingDivider {
                        divider_index: idx,
                        orientation,
                        start_pos: start,
                        last_pos: start,
                    };
                    actions.push(InputAction::DividerDragStart {
                        divider_index: idx,
                        orientation,
                    });
                    return actions;
                }

                // 3. Hit-test panel title bars
                if let Some(panel_id) =
                    find_panel_title_bar_at(grid, x as f32, y as f32, title_bar_height)
                {
                    self.drag = DragState::DraggingTitleBar {
                        panel_id,
                        start_pos: (x, y),
                    };
                    actions.push(InputAction::PanelSwapStart { panel_id });
                    return actions;
                }

                // 4. Hit-test panel body for focus
                if let Some(panel_id) =
                    find_panel_at(grid, x as f32, y as f32, title_bar_height)
                {
                    actions.push(InputAction::FocusPanel { panel_id });

                    // 5. Terminal selection handling
                    if let Some(PanelType::Terminal) = panel_types(panel_id) {
                        let block = modifiers.alt_key(); // D-14: Option+drag = block selection
                        actions.push(InputAction::TerminalSelectionStart {
                            panel_id,
                            x: x as f32,
                            y: y as f32,
                            block,
                        });
                        self.drag = DragState::DraggingTerminalSelection { panel_id };
                    }
                }
            }
            MouseButton::Right => {
                // Right-click: determine split direction based on cursor position
                // relative to panel center
                if let Some(panel_id) =
                    find_panel_at(grid, x as f32, y as f32, title_bar_height)
                {
                    if let Some(node) = grid.find_node(panel_id) {
                        let (px, py, pw, ph) = grid.get_panel_rect(node);
                        let py_offset = py + title_bar_height;

                        let rel_x = x as f32 - px;
                        let rel_y = y as f32 - py_offset;

                        // Determine direction: if cursor is in left/right third, split horizontal.
                        // If in top/bottom third, split vertical. Center defaults to horizontal.
                        let x_third = pw / 3.0;
                        let y_third = ph / 3.0;
                        let in_horizontal_third = rel_x < x_third || rel_x > x_third * 2.0;
                        let in_vertical_third = rel_y < y_third || rel_y > y_third * 2.0;

                        if in_vertical_third && !in_horizontal_third {
                            actions.push(InputAction::PanelSplitVertical { panel_id });
                        } else {
                            // Default to horizontal split (including center and left/right edges)
                            actions.push(InputAction::PanelSplitHorizontal { panel_id });
                        }
                    }
                }
            }
            _ => {}
        }

        actions
    }

    /// Handle mouse button release.
    pub fn on_mouse_release(
        &mut self,
        button: MouseButton,
        grid: &GridLayout,
        title_bar_height: f32,
    ) -> Vec<InputAction> {
        let mut actions = Vec::new();

        if button != MouseButton::Left {
            return actions;
        }

        match &self.drag {
            DragState::DraggingDivider { .. } => {
                actions.push(InputAction::DividerDragEnd);
                self.drag = DragState::Idle;
            }
            DragState::DraggingTitleBar { panel_id, .. } => {
                let dragged_id = *panel_id;
                self.drag = DragState::Idle;
                // Check if cursor is over a different panel
                if let Some(target_id) = find_panel_at(
                    grid,
                    self.cursor_x as f32,
                    self.cursor_y as f32,
                    title_bar_height,
                ) {
                    if target_id != dragged_id {
                        // Swap panels: we need to tell the app both IDs
                        actions.push(InputAction::PanelSwapDrop {
                            source_panel_id: dragged_id,
                            target_panel_id: target_id,
                        });
                    }
                }
            }
            DragState::DraggingTerminalSelection { panel_id } => {
                let pid = *panel_id;
                actions.push(InputAction::TerminalSelectionEnd { panel_id: pid });
                self.drag = DragState::Idle;
            }
            DragState::Idle => {}
        }

        actions
    }

    /// Handle mouse wheel/scroll events.
    ///
    /// Returns InputActions for terminal scrolling when cursor is over a terminal panel.
    pub fn on_mouse_wheel(
        &self,
        delta_lines: f32,
        grid: &GridLayout,
        title_bar_height: f32,
        panel_types: &dyn Fn(PanelId) -> Option<PanelType>,
    ) -> Vec<InputAction> {
        let mut actions = Vec::new();

        // Find which panel the cursor is over
        if let Some(panel_id) =
            find_panel_at(grid, self.cursor_x as f32, self.cursor_y as f32, title_bar_height)
        {
            match panel_types(panel_id) {
                Some(PanelType::Terminal) => {
                    let lines = delta_lines as i32;
                    if lines != 0 {
                        actions.push(InputAction::TerminalScroll {
                            panel_id,
                            delta: lines,
                        });
                    }
                }
                Some(PanelType::Markdown) => {
                    // Convert line delta to pixel delta (approx 21px per line)
                    let pixel_delta = delta_lines * 21.0;
                    if pixel_delta.abs() > 0.01 {
                        actions.push(InputAction::MarkdownScroll {
                            panel_id,
                            delta: pixel_delta,
                        });
                    }
                }
                Some(PanelType::Canvas) => {
                    actions.push(InputAction::CanvasZoom {
                        panel_id,
                        delta: delta_lines,
                    });
                }
                _ => {}
            }
        }

        actions
    }

    /// Get the current divider drag info (if dragging a divider).
    pub fn divider_drag_info(&self) -> Option<(usize, Orientation)> {
        match &self.drag {
            DragState::DraggingDivider {
                divider_index,
                orientation,
                ..
            } => Some((*divider_index, *orientation)),
            _ => None,
        }
    }
}

/// Find which panel contains the given screen coordinates.
fn find_panel_at(
    grid: &GridLayout,
    x: f32,
    y: f32,
    title_bar_height: f32,
) -> Option<PanelId> {
    for &(node, panel_id) in grid.panel_nodes() {
        let (px, py, pw, ph) = grid.get_panel_rect(node);
        let py_offset = py + title_bar_height;

        if x >= px && x <= px + pw && y >= py_offset && y <= py_offset + ph {
            return Some(panel_id);
        }
    }
    None
}

/// Find which panel's title bar region contains the given screen coordinates.
fn find_panel_title_bar_at(
    grid: &GridLayout,
    x: f32,
    y: f32,
    title_bar_height: f32,
) -> Option<PanelId> {
    for &(node, panel_id) in grid.panel_nodes() {
        let (px, py, pw, _ph) = grid.get_panel_rect(node);
        let py_offset = py + title_bar_height;

        if x >= px
            && x <= px + pw
            && y >= py_offset
            && y <= py_offset + PANEL_TITLE_BAR_HEIGHT
        {
            return Some(panel_id);
        }
    }
    None
}

/// Hit-test close and fullscreen buttons on all panels.
///
/// Per the plan: close button is at (panel_right - 40, panel_top + 6) 16x16.
/// Fullscreen button is at (panel_right - 20, panel_top + 6) 16x16.
fn hit_test_buttons(
    grid: &GridLayout,
    x: f32,
    y: f32,
    title_bar_height: f32,
) -> Option<InputAction> {
    for &(node, panel_id) in grid.panel_nodes() {
        let (px, py, pw, _ph) = grid.get_panel_rect(node);
        let py_offset = py + title_bar_height;
        let panel_right = px + pw;

        // Close button rect
        let close_x = panel_right - CLOSE_BUTTON_RIGHT_OFFSET;
        let close_y = py_offset + CLOSE_BUTTON_TOP_OFFSET;
        if x >= close_x
            && x <= close_x + CLOSE_BUTTON_SIZE
            && y >= close_y
            && y <= close_y + CLOSE_BUTTON_SIZE
        {
            return Some(InputAction::PanelClose { panel_id });
        }

        // Fullscreen button rect
        let fs_x = panel_right - FULLSCREEN_BUTTON_RIGHT_OFFSET;
        let fs_y = py_offset + FULLSCREEN_BUTTON_TOP_OFFSET;
        if x >= fs_x
            && x <= fs_x + CLOSE_BUTTON_SIZE
            && y >= fs_y
            && y <= fs_y + CLOSE_BUTTON_SIZE
        {
            return Some(InputAction::PanelToggleFullscreen { panel_id });
        }
    }
    None
}
