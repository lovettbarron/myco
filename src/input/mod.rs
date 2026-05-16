pub mod keyboard;
pub mod mouse;

use crate::grid::{Orientation, PanelId};

/// Actions produced by the input system for the app to process.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InputAction {
    /// User started dragging a divider.
    DividerDragStart {
        divider_index: usize,
        orientation: Orientation,
    },
    /// User is moving a divider (delta in pixels along the drag axis).
    DividerDragMove { delta_pixels: f32 },
    /// User released the divider.
    DividerDragEnd,
    /// Split the focused panel horizontally (add a column).
    PanelSplitHorizontal { panel_id: PanelId },
    /// Split the focused panel vertically (add a row).
    PanelSplitVertical { panel_id: PanelId },
    /// Close a panel.
    PanelClose { panel_id: PanelId },
    /// Start dragging a panel for swap (title bar drag).
    PanelSwapStart { panel_id: PanelId },
    /// Drop a dragged panel onto a target to swap.
    PanelSwapDrop { source_panel_id: PanelId, target_panel_id: PanelId },
    /// Toggle fullscreen for a panel.
    PanelToggleFullscreen { panel_id: PanelId },
    /// Context menu requested at a position.
    ContextMenu { panel_id: PanelId, x: f32, y: f32 },
    /// Change the cursor style.
    SetCursor(CursorStyle),
    /// Focus a panel.
    FocusPanel { panel_id: PanelId },
}

/// Cursor styles for different interaction states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Default,
    ColResize,
    RowResize,
}
