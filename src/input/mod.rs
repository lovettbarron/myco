pub mod keyboard;
pub mod mouse;

use std::path::PathBuf;

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
    /// Write raw bytes to a terminal's PTY.
    TerminalInput { panel_id: PanelId, bytes: Vec<u8> },
    /// Scroll terminal by delta lines (positive = up/back in history).
    TerminalScroll { panel_id: PanelId, delta: i32 },
    /// Copy selected text or send SIGINT if no selection (D-13).
    TerminalCopy { panel_id: PanelId },
    /// Paste clipboard contents into terminal.
    TerminalPaste { panel_id: PanelId },
    /// Change terminal font size by delta (D-05/TERM-07).
    TerminalFontSizeChange { panel_id: PanelId, delta: f32 },
    /// Open search overlay (D-09).
    TerminalSearchOpen { panel_id: PanelId },
    /// Close search overlay.
    TerminalSearchClose { panel_id: PanelId },
    /// Navigate to next search match.
    TerminalSearchNext { panel_id: PanelId },
    /// Navigate to previous search match.
    TerminalSearchPrev { panel_id: PanelId },
    /// Update search query text.
    TerminalSearchUpdate { panel_id: PanelId, query: String },
    /// Character typed into search box.
    TerminalSearchChar { panel_id: PanelId, ch: char },
    /// Backspace in search box.
    TerminalSearchBackspace { panel_id: PanelId },
    /// Accept inline ghost text autocomplete suggestion (Right arrow).
    AutocompleteAccept { panel_id: PanelId },
    /// Open reverse history search (Ctrl+R).
    HistorySearchOpen { panel_id: PanelId },
    /// Close history search overlay.
    HistorySearchClose { panel_id: PanelId },
    /// Character typed in history search.
    HistorySearchChar { panel_id: PanelId, ch: char },
    /// Backspace in history search.
    HistorySearchBackspace { panel_id: PanelId },
    /// Next result in history search.
    HistorySearchNext { panel_id: PanelId },
    /// Previous result in history search.
    HistorySearchPrev { panel_id: PanelId },
    /// Accept selected history search result.
    HistorySearchAccept { panel_id: PanelId },
    /// Start text selection at a grid point.
    TerminalSelectionStart { panel_id: PanelId, x: f32, y: f32, block: bool },
    /// Update selection endpoint.
    TerminalSelectionUpdate { panel_id: PanelId, x: f32, y: f32 },
    /// End selection (mouse released).
    TerminalSelectionEnd { panel_id: PanelId },
    /// Create new terminal panel (from menu/shortcut).
    CreateTerminal,
    /// Create a new TLDraw canvas panel.
    CreateCanvas,
    /// Handle an IPC message from a canvas webview.
    CanvasIpcMessage { panel_id: PanelId, message: String },
    /// Open a markdown file in a markdown panel.
    OpenMarkdown { path: PathBuf },
    /// Scroll markdown panel.
    MarkdownScroll { panel_id: PanelId, delta: f32 },
    /// Zoom canvas panel (scroll wheel → zoom in/out).
    CanvasZoom { panel_id: PanelId, delta: f32 },
    /// Markdown file changed on disk (from watcher).
    MarkdownFileChanged { path: PathBuf },
    /// Toggle sidebar visibility.
    ToggleSidebar,
    /// Open file from sidebar.
    SidebarSelect { path: PathBuf },
    /// Create new canvas from sidebar.
    SidebarNewCanvas,
    /// Cycle focus to next panel.
    FocusNextPanel,
    /// Cycle focus to previous panel.
    FocusPrevPanel,
    /// Open a file in a new panel (from sidebar context menu).
    SidebarOpenInPane { path: PathBuf },
    /// Reveal file in macOS Finder.
    SidebarRevealInFinder { path: PathBuf },
    /// Rename a file or directory (from sidebar context menu).
    SidebarRename { path: PathBuf },
    /// Delete a file or directory (from sidebar context menu).
    SidebarDelete { path: PathBuf },
    /// Copy absolute path to clipboard.
    SidebarCopyPath { path: PathBuf },
    /// Copy relative path to clipboard.
    SidebarCopyRelativePath { path: PathBuf },
    /// User accepted the project initialization prompt.
    InitPromptAccept,
    /// User dismissed the project initialization prompt.
    InitPromptDismiss,
    /// Switch the active theme by name.
    ThemeSwitch { theme_name: String },
    /// Open the settings overlay (Cmd+,).
    OpenSettings,
    /// Close the settings overlay (Esc).
    CloseSettings,
    /// Switch to a different project by path (from sidebar project switcher).
    ProjectSwitch { path: PathBuf },
    /// Quit the application (Cmd+Q).
    Quit,
}

/// Convert an action ID string (from the shortcut registry) to an InputAction.
///
/// Takes the focused panel_id as context for panel-specific actions.
/// Returns None for unknown action IDs or actions handled at the app level (e.g., quit).
pub fn action_from_id(action_id: &str, panel_id: PanelId) -> Option<InputAction> {
    match action_id {
        "panel_split_h" => Some(InputAction::PanelSplitHorizontal { panel_id }),
        "panel_split_v" => Some(InputAction::PanelSplitVertical { panel_id }),
        "panel_close" => Some(InputAction::PanelClose { panel_id }),
        "create_terminal" => Some(InputAction::CreateTerminal),
        "create_canvas" => Some(InputAction::CreateCanvas),
        "toggle_sidebar" => Some(InputAction::ToggleSidebar),
        "focus_next_panel" => Some(InputAction::FocusNextPanel),
        "focus_prev_panel" => Some(InputAction::FocusPrevPanel),
        "terminal_copy" => Some(InputAction::TerminalCopy { panel_id }),
        "terminal_paste" => Some(InputAction::TerminalPaste { panel_id }),
        "terminal_search" => Some(InputAction::TerminalSearchOpen { panel_id }),
        "open_settings" => Some(InputAction::OpenSettings),
        "font_size_up" => Some(InputAction::TerminalFontSizeChange { panel_id, delta: 1.0 }),
        "font_size_down" => Some(InputAction::TerminalFontSizeChange { panel_id, delta: -1.0 }),
        "toggle_fullscreen" => Some(InputAction::PanelToggleFullscreen { panel_id }),
        "quit" => Some(InputAction::Quit),
        _ => None,
    }
}

/// Cursor styles for different interaction states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Default,
    ColResize,
    RowResize,
}
