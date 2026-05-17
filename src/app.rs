use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, trace, warn};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window, WindowId};

use alacritty_terminal::grid::Dimensions as TermDimTrait;
use crate::grid::divider::{
    self, compute_dividers, DividerSet, Orientation, DIVIDER_VISUAL_WIDTH,
};
use crate::grid::layout::GridLayout;
use crate::grid::operations::{self, SplitDirection};
use crate::grid::panel::{Panel, PanelId, PanelType};

/// Custom event type for waking winit from background threads.
#[derive(Debug, Clone)]
pub enum UserEvent {
    TerminalEvent,
    FileChanged(Vec<std::path::PathBuf>),
    CanvasMessage(PanelId, String),
    ResourceUpdate(Vec<crate::monitor::ResourceUpdate>),
    #[cfg(target_os = "macos")]
    MenuAction(u32),
}
use crate::input::keyboard;
use crate::input::mouse::MouseState;
use crate::input::{CursorStyle, InputAction};
use crate::shortcuts::{ChordStateMachine, ShortcutRegistry};
use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::renderer::Renderer;
use crate::canvas::CanvasManager;
use crate::markdown::{MarkdownManager, MarkdownRenderer};
use crate::sidebar::{SidebarState, SidebarAction, SIDEBAR_WIDTH};
use crate::sidebar::renderer::SidebarRenderer;
use crate::config::registry::ProjectRegistry;
use crate::picker::{PickerAction, PickerState};
use crate::settings::{SettingsClickResult, SettingsRenderer, SettingsState};
use crate::status_bar::{BottomBar, StatsBar, BOTTOM_BAR_HEIGHT, STATS_BAR_HEIGHT};
use crate::terminal::renderer::{TerminalRenderer, TerminalSnapshot};
use crate::terminal::TerminalManager;
use crate::theme::{Theme, ThemeRegistry, linear_to_srgb_u8};
use crate::watcher::FileWatcher;
use crate::window::create_window;

/// Height of the app title bar in logical points.
const TITLE_BAR_HEIGHT: f32 = 38.0;

/// Combined top chrome height (title bar + stats bar) in logical points.
/// The grid content area starts below this offset.
const TOP_CHROME_HEIGHT: f32 = TITLE_BAR_HEIGHT + STATS_BAR_HEIGHT;

/// Whether the project initialization prompt is being shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitPrompt {
    /// No prompt — either `.myco` exists or user dismissed it.
    None,
    /// Prompt is visible, waiting for user input.
    Showing,
}

/// Top-level application state: picker or workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    /// Project picker is showing (no project loaded yet).
    Picker,
    /// Workspace is active (project loaded, grid rendered).
    Workspace,
}

/// Height of the panel title bar area in logical points.
const PANEL_TITLE_HEIGHT: f32 = 28.0;

/// Horizontal padding inside panel content areas (e.g. terminal text inset from panel edge).
const PANEL_CONTENT_PADDING: f32 = 8.0;

/// State for a resource dot tooltip (shown on hover after 300ms).
struct TooltipState {
    /// Panel whose resource dot is hovered.
    panel_id: PanelId,
    /// CPU percentage to display.
    cpu_percent: f32,
    /// Memory in bytes to display.
    memory_bytes: u64,
    /// Tooltip position (x).
    x: f32,
    /// Tooltip position (y).
    y: f32,
    /// When hovering began.
    hover_start: Instant,
}

enum AutocompleteAction {
    None,
    Enter,
    Reset,
    Backspace,
    Chars(Vec<char>),
}

fn classify_input_for_autocomplete(bytes: &[u8]) -> AutocompleteAction {
    match bytes {
        [0x0d] => AutocompleteAction::Enter,
        [0x03] | [0x15] | [0x1b] => AutocompleteAction::Reset,
        [0x7f] | [0x08] => AutocompleteAction::Backspace,
        _ => {
            if bytes.len() == 1 && bytes[0] >= 0x20 && bytes[0] < 0x7f {
                AutocompleteAction::Chars(vec![bytes[0] as char])
            } else if let Ok(s) = std::str::from_utf8(bytes) {
                if s.len() <= 4 && !s.starts_with('\x1b') {
                    let chars: Vec<char> = s.chars().filter(|c| !c.is_control()).collect();
                    if chars.is_empty() {
                        AutocompleteAction::None
                    } else {
                        AutocompleteAction::Chars(chars)
                    }
                } else {
                    AutocompleteAction::Reset
                }
            } else {
                AutocompleteAction::None
            }
        }
    }
}

/// Accumulates per-frame performance metrics (frame stats) for periodic logging.
///
/// Records frame timing, quad count, and terminal cell count.
/// Logs a summary at `debug!` level every 60 frames, then resets.
/// Activate with `RUST_LOG=myco=debug`.
struct FrameStats {
    frame_count: u64,
    frame_time_sum: Duration,
    frame_time_max: Duration,
    quad_count_sum: u64,
    cell_count_sum: u64,
    last_log: Instant,
}

impl FrameStats {
    fn new() -> Self {
        Self {
            frame_count: 0,
            frame_time_sum: Duration::ZERO,
            frame_time_max: Duration::ZERO,
            quad_count_sum: 0,
            cell_count_sum: 0,
            last_log: Instant::now(),
        }
    }

    fn record(&mut self, frame_time: Duration, quad_count: usize, cell_count: usize) {
        self.frame_count += 1;
        self.frame_time_sum += frame_time;
        self.frame_time_max = self.frame_time_max.max(frame_time);
        self.quad_count_sum += quad_count as u64;
        self.cell_count_sum += cell_count as u64;
    }

    fn should_log(&self) -> bool {
        self.frame_count >= 60 || self.last_log.elapsed() >= Duration::from_secs(5)
    }

    fn log_and_reset(&mut self) {
        if self.frame_count == 0 {
            return;
        }
        let avg = self.frame_time_sum / self.frame_count as u32;
        debug!(
            frames = self.frame_count,
            avg_ms = format!("{:.2}", avg.as_secs_f64() * 1000.0),
            max_ms = format!("{:.2}", self.frame_time_max.as_secs_f64() * 1000.0),
            avg_quads = self.quad_count_sum / self.frame_count,
            avg_cells = self.cell_count_sum / self.frame_count,
            "frame stats"
        );
        *self = Self::new();
    }
}

/// Main application state.
///
/// Owns the window, renderer, grid layout, panels, theme, input state,
/// terminal manager, and terminal renderer.
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    theme: Theme,
    theme_registry: ThemeRegistry,
    grid: Option<GridLayout>,
    panels: Vec<Panel>,
    mouse_state: MouseState,
    dividers: DividerSet,
    focused_panel: Option<PanelId>,
    modifiers: ModifiersState,
    /// Manages all terminal instances (PTY lifecycle, event draining).
    terminal_manager: Option<TerminalManager>,
    /// Manages all canvas (TLDraw webview) instances.
    canvas_manager: Option<CanvasManager>,
    /// Manages all markdown viewer instances.
    markdown_manager: Option<MarkdownManager>,
    /// GPU terminal renderer (snapshot + buffer building, quad generation).
    terminal_renderer: TerminalRenderer,
    /// GPU markdown renderer (buffer caching, quad generation for code blocks/blockquotes/HRs).
    markdown_renderer: crate::markdown::renderer::MarkdownRenderer,
    /// File sidebar state (project file tree browser).
    sidebar: Option<SidebarState>,
    /// Sidebar text buffers (cached for rendering).
    sidebar_buffers: Vec<glyphon::Buffer>,
    /// Sidebar text area metadata (positions for each buffer).
    sidebar_metas: Vec<crate::sidebar::renderer::SidebarTextAreaMeta>,
    /// File watcher monitoring the project directory for changes.
    file_watcher: Option<FileWatcher>,
    /// Proxy for waking the event loop from background threads.
    proxy: Option<EventLoopProxy<UserEvent>>,
    /// Pending actions to process after the current action completes.
    /// Used to avoid re-entrancy when forwarding IPC shortcuts.
    pending_actions: Vec<InputAction>,
    /// Whether a redraw has been requested for the current frame.
    redraw_pending: bool,
    /// Per-frame performance stats, logged every 60 frames at debug level.
    frame_stats: FrameStats,
    /// Display scale factor (2.0 on Retina, 1.0 on standard displays).
    scale_factor: f32,
    /// Project initialization prompt state.
    init_prompt: InitPrompt,
    /// Project directory path (set during resumed()).
    project_dir: Option<std::path::PathBuf>,
    /// Menu bar state (action map and toggle entries).
    #[cfg(target_os = "macos")]
    menu_state: Option<crate::platform::menu::MenuState>,
    /// Path of the file/dir targeted by the sidebar context menu.
    context_menu_target: Option<std::path::PathBuf>,
    /// Accumulated sub-line pixel scroll delta for smooth trackpad scrolling.
    scroll_pixel_accumulator: f64,
    /// Top stats bar (panel count, uptime).
    stats_bar: StatsBar,
    /// Bottom project info bar (git branch, dirty indicator, path).
    bottom_bar: Option<BottomBar>,
    /// Settings overlay state (opened by Cmd+,, closed by Esc).
    settings: SettingsState,
    /// Auto-save state for debounced config persistence (D-07, D-08).
    auto_save: crate::config::AutoSaveState,
    /// Shortcut registry mapping key combos to action IDs (D-02, D-14, D-18).
    shortcut_registry: ShortcutRegistry,
    /// Chord state machine for multi-key shortcut sequences (D-15).
    chord_state: ChordStateMachine,
    /// Top-level application state: picker or workspace.
    app_state: AppState,
    /// Picker state (only present when app_state == Picker).
    picker_state: Option<PickerState>,
    /// Project registry (persists across picker and workspace).
    project_registry: ProjectRegistry,
    /// Resource monitor for per-process CPU/RAM polling.
    resource_monitor: Option<crate::monitor::ResourceMonitor>,
    /// Current resource state per PID.
    resource_states: HashMap<u32, crate::monitor::ResourceState>,
    /// Unified toast notification manager.
    toast_manager: crate::toast::ToastManager,
    /// Tooltip state for resource dot hover.
    tooltip_state: Option<TooltipState>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            window: None,
            renderer: None,
            theme: Theme::default(),
            theme_registry: ThemeRegistry::new(),
            grid: None,
            panels: Vec::new(),
            mouse_state: MouseState::default(),
            dividers: DividerSet {
                dividers: Vec::new(),
            },
            focused_panel: Some(PanelId(0)),
            modifiers: ModifiersState::empty(),
            terminal_manager: None,
            canvas_manager: None,
            markdown_manager: Some(MarkdownManager::new()),
            pending_actions: Vec::new(),
            terminal_renderer: TerminalRenderer::new(),
            markdown_renderer: crate::markdown::renderer::MarkdownRenderer::new(),
            sidebar: None,
            sidebar_buffers: Vec::new(),
            sidebar_metas: Vec::new(),
            file_watcher: None,
            proxy: Some(proxy),
            redraw_pending: false,
            frame_stats: FrameStats::new(),
            scale_factor: 1.0,
            init_prompt: InitPrompt::None,
            project_dir: None,
            #[cfg(target_os = "macos")]
            menu_state: None,
            context_menu_target: None,
            scroll_pixel_accumulator: 0.0,
            stats_bar: StatsBar::new(),
            bottom_bar: None,
            settings: SettingsState::new(),
            auto_save: crate::config::AutoSaveState::new(),
            shortcut_registry: ShortcutRegistry::new(),
            chord_state: ChordStateMachine::new(),
            app_state: AppState::Picker,
            picker_state: None,
            project_registry: ProjectRegistry::new(),
            resource_monitor: None,
            resource_states: HashMap::new(),
            toast_manager: crate::toast::ToastManager::new(),
            tooltip_state: None,
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
                        (Orientation::Vertical, Some(w)) => {
                            w.inner_size().width as f32 / self.scale_factor
                        }
                        (Orientation::Horizontal, Some(w)) => {
                            w.inner_size().height as f32 / self.scale_factor
                                - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT
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
                        self.auto_save.mark_dirty();
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
                        self.auto_save.mark_dirty();
                    }
                }
            }
            InputAction::PanelClose { panel_id } => {
                // Destroy terminal if this is a terminal panel
                if let Some(tm) = &mut self.terminal_manager {
                    tm.destroy_terminal(&panel_id);
                }
                self.terminal_renderer.invalidate_panel_cache(&panel_id);
                // Destroy canvas if this is a canvas panel
                if let Some(cm) = &mut self.canvas_manager {
                    cm.destroy_canvas(&panel_id);
                }
                // Destroy markdown viewer if this is a markdown panel
                if let Some(mm) = &mut self.markdown_manager {
                    mm.destroy_markdown(&panel_id);
                }
                self.markdown_renderer.invalidate_panel_cache(&panel_id);
                if let Some(grid) = self.grid.as_mut() {
                    if operations::close_panel(grid, panel_id) {
                        self.panels.retain(|p| p.id != panel_id);
                        if self.focused_panel == Some(panel_id) {
                            self.focused_panel =
                                grid.panel_nodes().first().map(|(_, id)| *id);
                        }
                        self.recompute_layout();
                        self.sync_child_pids();
                        self.auto_save.mark_dirty();
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
            InputAction::SidebarOpenInPane { path } => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                match ext {
                    "md" | "markdown" => {
                        self.process_action(InputAction::OpenMarkdown { path });
                    }
                    "tldr" => {
                        let canvas_id = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        self.create_canvas_with_id(&canvas_id);
                    }
                    _ => {
                        debug!("Cannot open file type in pane: {}", ext);
                    }
                }
            }
            InputAction::SidebarRevealInFinder { path } => {
                let target = if path.is_file() {
                    path.parent().unwrap_or(&path).to_path_buf()
                } else {
                    path.clone()
                };
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open").arg(&target).spawn();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = std::process::Command::new("xdg-open").arg(&target).spawn();
                }
            }
            InputAction::SidebarRename { path } => {
                #[cfg(target_os = "macos")]
                {
                    let old_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if let Some(new_name) =
                        crate::platform::context_menu::show_rename_dialog(&old_name)
                    {
                        if !new_name.is_empty() && new_name != old_name {
                            if let Some(parent) = path.parent() {
                                let new_path = parent.join(&new_name);
                                if let Err(e) = std::fs::rename(&path, &new_path) {
                                    warn!("Failed to rename: {}", e);
                                } else {
                                    if let Some(sidebar) = &mut self.sidebar {
                                        sidebar.refresh_file_tree();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            InputAction::SidebarDelete { path } => {
                #[cfg(target_os = "macos")]
                {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if crate::platform::context_menu::show_delete_confirmation(&name) {
                        let result = if path.is_dir() {
                            std::fs::remove_dir_all(&path)
                        } else {
                            std::fs::remove_file(&path)
                        };
                        if let Err(e) = result {
                            warn!("Failed to delete: {}", e);
                        } else {
                            if let Some(sidebar) = &mut self.sidebar {
                                sidebar.selected = None;
                                sidebar.refresh_file_tree();
                            }
                        }
                    }
                }
            }
            InputAction::SidebarCopyPath { path } => {
                if let Ok(mut ctx) = copypasta::ClipboardContext::new() {
                    use copypasta::ClipboardProvider;
                    let _ = ctx.set_contents(path.to_string_lossy().to_string());
                }
            }
            InputAction::SidebarCopyRelativePath { path } => {
                let relative = self
                    .project_dir
                    .as_ref()
                    .and_then(|proj| path.strip_prefix(proj).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                if let Ok(mut ctx) = copypasta::ClipboardContext::new() {
                    use copypasta::ClipboardProvider;
                    let _ = ctx.set_contents(relative);
                }
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
                let old_focus = self.focused_panel;
                self.focused_panel = Some(panel_id);

                // Handle webview focus transitions (D-15, D-16)
                if let Some(cm) = &self.canvas_manager {
                    // Unfocus previous canvas if it was focused
                    if let Some(old_id) = old_focus {
                        if self.panels.iter().any(|p| p.id == old_id && p.panel_type == PanelType::Canvas) {
                            cm.set_focus(&old_id, false);
                        }
                    }
                    // Focus new canvas if target is a canvas
                    if self.panels.iter().any(|p| p.id == panel_id && p.panel_type == PanelType::Canvas) {
                        cm.set_focus(&panel_id, true);
                    } else {
                        // Focusing a GPU panel: return focus from any webview to parent window
                        cm.unfocus_all();
                    }
                }
            }

            // === Terminal actions ===
            InputAction::TerminalInput { panel_id, bytes } => {
                let mut should_close = false;
                // Determine what autocomplete action to take before borrowing mutably
                let mut ac_action = AutocompleteAction::None;
                if let Some(tm) = &self.terminal_manager {
                    if let Some(ts) = tm.get(&panel_id) {
                        if ts.exited {
                            should_close = true;
                        } else {
                            let in_alt = ts.term.lock().mode().contains(
                                alacritty_terminal::term::TermMode::ALT_SCREEN,
                            );
                            if !in_alt {
                                ac_action = classify_input_for_autocomplete(&bytes);
                            }
                        }
                    }
                }
                if should_close {
                    self.process_action(InputAction::PanelClose { panel_id });
                    return;
                }
                if let Some(tm) = &mut self.terminal_manager {
                    // Apply autocomplete tracking then write to PTY
                    match ac_action {
                        AutocompleteAction::Enter => {
                            if let Some(ts) = tm.terminals.get_mut(&panel_id) {
                                ts.autocomplete.on_enter(&mut tm.history);
                            }
                        }
                        AutocompleteAction::Reset => {
                            if let Some(ts) = tm.get_mut(&panel_id) {
                                ts.autocomplete.on_control_reset();
                            }
                        }
                        AutocompleteAction::Backspace => {
                            if let Some(ts) = tm.terminals.get_mut(&panel_id) {
                                ts.autocomplete.on_backspace(&tm.history);
                            }
                        }
                        AutocompleteAction::Chars(chars) => {
                            if let Some(ts) = tm.terminals.get_mut(&panel_id) {
                                for ch in chars {
                                    ts.autocomplete.on_char(ch, &tm.history);
                                }
                            }
                        }
                        AutocompleteAction::None => {}
                    }
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.write_to_pty(&bytes);
                        ts.reset_cursor_blink();
                    }
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
                        // Resize terminal grid and notify PTY
                        if let Some(grid) = &self.grid {
                            if let Some(node_id) = grid.find_node(panel_id) {
                                let (_, _, pw, ph) = grid.get_panel_rect(node_id);
                                let cols =
                                    ((pw - PANEL_CONTENT_PADDING * 2.0) / ts.cell_width).max(2.0) as usize;
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
                                        let cols = ((pw - PANEL_CONTENT_PADDING * 2.0) / cw).max(2.0) as usize;
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
                            self.sync_child_pids();
                            self.auto_save.mark_dirty();
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
                let sidebar_off = self.sidebar_offset();
                if let (Some(tm), Some(grid)) = (&mut self.terminal_manager, &self.grid) {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        // Check if click is on the "New output" indicator (D-10)
                        if ts.has_new_output_while_scrolled {
                            if let Some(node) = grid.find_node(panel_id) {
                                let (px, py, pw, ph) = grid.get_panel_rect(node);
                                let py_offset = py + TOP_CHROME_HEIGHT;
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
                            let (px, py, _pw, ph) = grid.get_panel_rect(node);
                            let viewport_x = px + sidebar_off + PANEL_CONTENT_PADDING;
                            let content_h = ph - PANEL_TITLE_HEIGHT;
                            let snapshot = TerminalRenderer::snapshot(&ts.term);
                            let display_offset = snapshot.display_offset;
                            let bottom_offset = if ts.scroll_offset == 0 {
                                snapshot.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                            } else {
                                0.0
                            };
                            let viewport_y =
                                py + TOP_CHROME_HEIGHT + PANEL_TITLE_HEIGHT + bottom_offset;
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
                let sidebar_off = self.sidebar_offset();
                if let (Some(tm), Some(grid)) = (&mut self.terminal_manager, &self.grid) {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        if let Some(node) = grid.find_node(panel_id) {
                            let (px, py, _pw, ph) = grid.get_panel_rect(node);
                            let viewport_x = px + sidebar_off + PANEL_CONTENT_PADDING;
                            let content_h = ph - PANEL_TITLE_HEIGHT;
                            let snapshot = TerminalRenderer::snapshot(&ts.term);
                            let display_offset = snapshot.display_offset;
                            let bottom_offset = if ts.scroll_offset == 0 {
                                snapshot.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                            } else {
                                0.0
                            };
                            let viewport_y =
                                py + TOP_CHROME_HEIGHT + PANEL_TITLE_HEIGHT + bottom_offset;
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

            // === Autocomplete / History search actions ===
            InputAction::AutocompleteAccept { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        if let Some(text) = ts.autocomplete.accept_ghost() {
                            ts.write_to_pty(text.as_bytes());
                            ts.reset_cursor_blink();
                        }
                    }
                }
            }
            InputAction::HistorySearchOpen { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.terminals.get_mut(&panel_id) {
                        ts.autocomplete.open_history_search(&tm.history);
                    }
                }
            }
            InputAction::HistorySearchClose { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.autocomplete.close_history_search();
                    }
                }
            }
            InputAction::HistorySearchChar { panel_id, ch } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.terminals.get_mut(&panel_id) {
                        ts.autocomplete.history_search_char(ch, &tm.history);
                    }
                }
            }
            InputAction::HistorySearchBackspace { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.terminals.get_mut(&panel_id) {
                        ts.autocomplete.history_search_backspace(&tm.history);
                    }
                }
            }
            InputAction::HistorySearchNext { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.autocomplete.history_search_next();
                    }
                }
            }
            InputAction::HistorySearchPrev { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        ts.autocomplete.history_search_prev();
                    }
                }
            }
            InputAction::HistorySearchAccept { panel_id } => {
                if let Some(tm) = &mut self.terminal_manager {
                    if let Some(ts) = tm.get_mut(&panel_id) {
                        if let Some(cmd) = ts.autocomplete.history_search_accept() {
                            ts.write_to_pty(b"\x15");
                            ts.write_to_pty(cmd.as_bytes());
                            ts.reset_cursor_blink();
                        }
                    }
                }
            }

            // === Canvas actions (Phase 3) ===
            InputAction::CreateCanvas => {
                if let Some(focused_id) = self.focused_panel {
                    if let Some(grid) = self.grid.as_mut() {
                        if let Some(new_id) =
                            operations::split_panel(grid, focused_id, SplitDirection::Horizontal)
                        {
                            let canvas_id = format!(
                                "canvas-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs()
                            );
                            let panel = Panel::new_canvas(new_id, canvas_id.clone());
                            self.panels.push(panel);
                            self.focused_panel = Some(new_id);
                            self.recompute_layout();

                            // Create canvas webview
                            if let Some(bounds) = self.panel_content_bounds(new_id) {
                                if let (Some(cm), Some(window), Some(proxy)) =
                                    (&mut self.canvas_manager, &self.window, &self.proxy)
                                {
                                    if let Err(e) = cm.create_canvas(
                                        new_id,
                                        &canvas_id,
                                        window,
                                        bounds,
                                        proxy.clone(),
                                    ) {
                                        warn!("Failed to create canvas: {}", e);
                                    }
                                }
                            }
                            self.auto_save.mark_dirty();
                        }
                    }
                }
            }
            InputAction::CanvasIpcMessage { panel_id, message } => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&message) {
                    match parsed.get("type").and_then(|t| t.as_str()) {
                        Some("shortcut") => {
                            // D-14: Forward app-level shortcuts from webview back to Myco
                            let key = parsed.get("key").and_then(|k| k.as_str()).unwrap_or("");
                            let shift = parsed.get("shift").and_then(|s| s.as_bool()).unwrap_or(false);
                            // T-03-05: Only translate known shortcut keys
                            let action = match (key, shift) {
                                ("w", _) => Some(InputAction::PanelClose { panel_id }),
                                ("d", false) => Some(InputAction::PanelSplitHorizontal { panel_id }),
                                ("D", _) | ("d", true) => Some(InputAction::PanelSplitVertical { panel_id }),
                                ("t", false) => Some(InputAction::CreateTerminal),
                                ("b", _) => Some(InputAction::ToggleSidebar),
                                ("]", _) => Some(InputAction::FocusNextPanel),
                                ("[", _) => Some(InputAction::FocusPrevPanel),
                                _ => None,
                            };
                            if let Some(a) = action {
                                self.pending_actions.push(a);
                            }
                            return;
                        }
                        Some("save") => {
                            if let Some(cm) = &mut self.canvas_manager {
                                cm.handle_ipc_message(&panel_id, &message);
                            }
                        }
                        _ => {
                            tracing::warn!("Unknown canvas IPC type from {:?}", panel_id);
                        }
                    }
                }
            }

            // === Markdown actions ===
            InputAction::OpenMarkdown { path } => {
                // D-12: Smart placement -- reuse existing markdown panel or split focused
                let existing_md_panel = self
                    .panels
                    .iter()
                    .find(|p| p.panel_type == PanelType::Markdown)
                    .map(|p| p.id);

                if let Some(md_id) = existing_md_panel {
                    // Replace content in existing markdown panel
                    if let Some(mm) = &mut self.markdown_manager {
                        mm.destroy_markdown(&md_id);
                        let _ = mm.create_markdown(md_id, path.clone());
                    }
                    self.markdown_renderer.invalidate_panel_cache(&md_id);
                    // Update panel title
                    if let Some(panel) = self.panels.iter_mut().find(|p| p.id == md_id) {
                        panel.title = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Markdown".into());
                        panel.file_path = Some(path);
                    }
                    self.focused_panel = Some(md_id);
                } else {
                    // Split focused panel to create new markdown panel
                    if let Some(focused_id) = self.focused_panel {
                        if let Some(grid) = self.grid.as_mut() {
                            if let Some(new_id) = operations::split_panel(
                                grid,
                                focused_id,
                                SplitDirection::Horizontal,
                            ) {
                                let panel = Panel::new_markdown(new_id, path.clone());
                                self.panels.push(panel);
                                self.focused_panel = Some(new_id);
                                self.recompute_layout();
                                if let Some(mm) = &mut self.markdown_manager {
                                    let _ = mm.create_markdown(new_id, path);
                                }
                            }
                        }
                    }
                }
            }
            InputAction::MarkdownScroll { panel_id, delta } => {
                // Compute bounds before borrowing markdown_manager mutably
                let viewport_h = self.panel_content_bounds(panel_id).map(|b| b.3).unwrap_or(300.0);
                if let Some(mm) = &mut self.markdown_manager {
                    if let Some(state) = mm.get_mut(&panel_id) {
                        state.scroll(delta, viewport_h);
                    }
                }
            }
            InputAction::CanvasZoom { panel_id, delta } => {
                if let Some(cm) = &self.canvas_manager {
                    if let Some(webview) = cm.get_webview(&panel_id) {
                        let zoom_factor = if delta > 0.0 { 1.05 } else { 0.95 };
                        let _ = webview.evaluate_script(&format!(
                            "if(window.editor){{var z=window.editor.getCamera().z;window.editor.setCamera({{...window.editor.getCamera(),z:z*{zoom_factor}}});}}"
                        ));
                    }
                }
            }
            InputAction::MarkdownFileChanged { path } => {
                if let Some(mm) = &mut self.markdown_manager {
                    mm.handle_file_changed(&[path]);
                }
            }

            // === Sidebar actions (Phase 3, Plan 03) ===
            InputAction::ToggleSidebar => {
                if let Some(sidebar) = &mut self.sidebar {
                    sidebar.toggle();
                    self.recompute_layout();
                }
                #[cfg(target_os = "macos")]
                self.update_menu_toggles();
            }
            InputAction::SidebarSelect { path } => {
                // T-03-09: Validate path is within project directory
                let is_valid = self
                    .sidebar
                    .as_ref()
                    .map(|s| path.starts_with(s.project_dir()))
                    .unwrap_or(false);
                if !is_valid {
                    warn!("Sidebar: rejected path outside project directory: {:?}", path);
                    return;
                }

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                match ext {
                    "md" | "markdown" => {
                        self.process_action(InputAction::OpenMarkdown { path });
                    }
                    "tldr" => {
                        // Extract canvas_id from filename
                        let canvas_id = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        self.create_canvas_with_id(&canvas_id);
                    }
                    _ => {} // Other file types ignored in Phase 3
                }
            }
            InputAction::SidebarNewCanvas => {
                if let Some(sidebar) = &self.sidebar {
                    if let Some(action) = sidebar.new_canvas() {
                        match action {
                            SidebarAction::CreateCanvas(canvas_id, _path) => {
                                self.create_canvas_with_id(&canvas_id);
                            }
                            _ => {}
                        }
                    }
                }
                // Refresh sidebar to show new file
                if let Some(sidebar) = &mut self.sidebar {
                    sidebar.refresh_file_tree();
                }
            }

            // === Focus cycling (Phase 3) ===
            InputAction::FocusNextPanel => {
                if let Some(current) = self.focused_panel {
                    let panel_ids: Vec<PanelId> = self.panels.iter().map(|p| p.id).collect();
                    if let Some(idx) = panel_ids.iter().position(|&id| id == current) {
                        let next_idx = (idx + 1) % panel_ids.len();
                        let next_id = panel_ids[next_idx];
                        self.pending_actions.push(InputAction::FocusPanel { panel_id: next_id });
                    }
                }
            }
            InputAction::FocusPrevPanel => {
                if let Some(current) = self.focused_panel {
                    let panel_ids: Vec<PanelId> = self.panels.iter().map(|p| p.id).collect();
                    if let Some(idx) = panel_ids.iter().position(|&id| id == current) {
                        let prev_idx = if idx == 0 { panel_ids.len() - 1 } else { idx - 1 };
                        let prev_id = panel_ids[prev_idx];
                        self.pending_actions.push(InputAction::FocusPanel { panel_id: prev_id });
                    }
                }
            }
            InputAction::OpenSettings => {
                let theme_names: Vec<String> = self
                    .theme_registry
                    .available_themes()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                let active_name = self.theme_registry.active().name.clone();

                // Get project info for the Project section
                let project_name = self
                    .project_dir
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string());
                let project_path = self
                    .project_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                let project_description = self
                    .project_dir
                    .as_ref()
                    .and_then(|p| {
                        let config = crate::config::load_project_config(p)?;
                        config.metadata.description
                    })
                    .unwrap_or_default();
                let project_theme = self
                    .project_dir
                    .as_ref()
                    .and_then(|p| {
                        let config = crate::config::load_project_config(p)?;
                        config.theme
                    });

                self.settings.open_with_project(
                    theme_names,
                    &active_name,
                    project_name,
                    project_path,
                    project_description,
                    project_theme.as_deref(),
                );
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                info!("Settings overlay opened");
            }
            InputAction::CloseSettings => {
                self.settings.close();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                info!("Settings overlay closed");
            }
            InputAction::ThemeSwitch { theme_name } => {
                if self.theme_registry.set_active(&theme_name) {
                    let definition = self.theme_registry.active();
                    // 1. Replace app theme
                    self.theme = Theme::from_definition(definition);
                    // 2. Replace terminal ANSI palette (per D-12)
                    self.terminal_renderer.palette = definition.to_ansi_palette();
                    // 3. Invalidate terminal buffer caches (colors changed, hashes stale)
                    self.terminal_renderer.invalidate_all_caches();
                    // 4. Request full redraw
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    info!("Theme switched to: {}", theme_name);
                } else {
                    warn!("Theme not found: {}", theme_name);
                }
            }
            InputAction::InitPromptAccept => {
                if let Some(project_dir) = &self.project_dir {
                    let myco_dir = project_dir.join(".myco");
                    if let Err(e) = std::fs::create_dir_all(myco_dir.join("canvas")) {
                        warn!("Failed to create .myco/canvas: {}", e);
                    }
                    if let Err(e) = crate::context::ensure_context_files(project_dir) {
                        warn!("Failed to write context files: {}", e);
                    }
                    info!("Initialized .myco project folder");
                    // Refresh sidebar to show the new directory
                    if let Some(sidebar) = &mut self.sidebar {
                        sidebar.refresh_file_tree();
                    }
                }
                self.init_prompt = InitPrompt::None;
            }
            InputAction::InitPromptDismiss => {
                info!("Project initialization skipped by user");
                self.init_prompt = InitPrompt::None;
            }
            InputAction::ProjectSwitch { path } => {
                info!("Switching project to: {:?}", path);
                // Save current layout before switching
                if let (Some(grid), Some(project_dir)) = (&self.grid, &self.project_dir) {
                    let config = crate::config::ProjectConfig::from_current_state(
                        grid,
                        &self.panels,
                        self.terminal_manager.as_ref(),
                        project_dir,
                        Some(&self.theme_registry.active().name),
                    );
                    crate::config::save_project_config(project_dir, &config);
                }
                // Destroy all terminals
                if let Some(tm) = &mut self.terminal_manager {
                    let panel_ids: Vec<PanelId> = tm.terminals().keys().copied().collect();
                    for pid in panel_ids {
                        tm.destroy_terminal(&pid);
                    }
                }
                // Destroy all canvases
                if let Some(cm) = &mut self.canvas_manager {
                    let panel_ids: Vec<PanelId> = self.panels.iter()
                        .filter(|p| p.panel_type == PanelType::Canvas)
                        .map(|p| p.id)
                        .collect();
                    for pid in panel_ids {
                        cm.destroy_canvas(&pid);
                    }
                }
                // Destroy all markdown viewers
                if let Some(mm) = &mut self.markdown_manager {
                    let panel_ids: Vec<PanelId> = self.panels.iter()
                        .filter(|p| p.panel_type == PanelType::Markdown)
                        .map(|p| p.id)
                        .collect();
                    for pid in panel_ids {
                        mm.destroy_markdown(&pid);
                    }
                }
                // Clear caches and panels
                self.terminal_renderer.invalidate_all_caches();
                for panel in &self.panels {
                    self.markdown_renderer.invalidate_panel_cache(&panel.id);
                }
                self.panels.clear();
                // Open new project
                self.open_project(path);
            }
            InputAction::Quit => {
                // Handled in window_event before reaching process_action.
                // This arm exists only for exhaustive match coverage.
            }
        }
    }

    /// Handle a menu bar action by tag, resolving to InputAction.
    #[cfg(target_os = "macos")]
    fn handle_menu_action(&mut self, tag: u32) {
        #[cfg(target_os = "macos")]
        {
            use crate::platform::context_menu::*;
            if let Some(path) = self.context_menu_target.take() {
                let action = match tag {
                    CTX_TAG_OPEN_IN_PANE => Some(InputAction::SidebarOpenInPane { path }),
                    CTX_TAG_REVEAL_IN_FINDER => {
                        Some(InputAction::SidebarRevealInFinder { path })
                    }
                    CTX_TAG_RENAME => Some(InputAction::SidebarRename { path }),
                    CTX_TAG_DELETE => Some(InputAction::SidebarDelete { path }),
                    CTX_TAG_COPY_PATH => Some(InputAction::SidebarCopyPath { path }),
                    CTX_TAG_COPY_RELATIVE_PATH => {
                        Some(InputAction::SidebarCopyRelativePath { path })
                    }
                    _ => {
                        self.context_menu_target = Some(path);
                        None
                    }
                };
                if let Some(action) = action {
                    self.process_action(action);
                    return;
                }
            }
        }

        let action_name = {
            let menu_state = match &self.menu_state {
                Some(s) => s,
                None => return,
            };
            match menu_state.action_map.get(&tag) {
                Some(name) => name.clone(),
                None => return,
            }
        };
        let panel_id = self.focused_panel.unwrap_or(PanelId(0));
        let input_action = match action_name.as_str() {
            "create_terminal" => Some(InputAction::CreateTerminal),
            "create_canvas" => Some(InputAction::CreateCanvas),
            "close_panel" => Some(InputAction::PanelClose { panel_id }),
            "toggle_sidebar" => Some(InputAction::ToggleSidebar),
            "split_horizontal" => Some(InputAction::PanelSplitHorizontal { panel_id }),
            "split_vertical" => Some(InputAction::PanelSplitVertical { panel_id }),
            "focus_next" => Some(InputAction::FocusNextPanel),
            "focus_prev" => Some(InputAction::FocusPrevPanel),
            "toggle_fullscreen" => Some(InputAction::PanelToggleFullscreen { panel_id }),
            "copy" => Some(InputAction::TerminalCopy { panel_id }),
            "paste" => Some(InputAction::TerminalPaste { panel_id }),
            "find" => Some(InputAction::TerminalSearchOpen { panel_id }),
            "font_size_up" => Some(InputAction::TerminalFontSizeChange { panel_id, delta: 1.0 }),
            "font_size_down" => Some(InputAction::TerminalFontSizeChange { panel_id, delta: -1.0 }),
            "init_project" => Some(InputAction::InitPromptAccept),
            _ => None,
        };
        if let Some(action) = input_action {
            self.process_action(action);
        }
    }

    /// Update menu bar toggle labels to reflect current app state.
    #[cfg(target_os = "macos")]
    fn update_menu_toggles(&self) {
        let menu_state = match &self.menu_state {
            Some(s) => s,
            None => return,
        };
        let mut state_map = std::collections::HashMap::new();
        let sidebar_visible = self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false);
        state_map.insert("sidebar_visible".to_string(), sidebar_visible);
        crate::platform::menu::update_toggle_labels(menu_state, &state_map);
    }

    /// Get the content bounds (below panel title) for a panel in logical pixels.
    /// When sidebar is visible, panel x positions are offset by SIDEBAR_WIDTH.
    fn panel_content_bounds(&self, panel_id: PanelId) -> Option<(f32, f32, f32, f32)> {
        let grid = self.grid.as_ref()?;
        let node_id = grid.find_node(panel_id)?;
        let (x, y, w, h) = grid.get_panel_rect(node_id);
        let sidebar_offset = self.sidebar_offset();
        let content_x = x + sidebar_offset + PANEL_CONTENT_PADDING;
        let content_y = y + TOP_CHROME_HEIGHT + PANEL_TITLE_HEIGHT;
        let content_w = w - PANEL_CONTENT_PADDING * 2.0;
        let content_h = h - PANEL_TITLE_HEIGHT;
        Some((content_x, content_y, content_w, content_h))
    }

    /// Save shortcut overrides to disk.
    ///
    /// Computes the sparse set of overrides by comparing current registry
    /// bindings against defaults, and writes only changed bindings to
    /// ~/.myco/shortcuts.json (D-18).
    fn save_shortcut_overrides(&self) {
        let defaults = crate::shortcuts::defaults::default_shortcuts();
        let mut overrides = Vec::new();
        for (action_id, keys) in self.shortcut_registry.all_bindings() {
            let keys_str: Vec<String> = keys
                .iter()
                .map(|k| crate::shortcuts::chord::key_combo_to_string(k))
                .collect();
            let default_binding = defaults.iter().find(|d| d.action == action_id);
            let is_default = default_binding
                .map(|d| d.keys == keys_str)
                .unwrap_or(false);
            if !is_default {
                overrides.push(crate::shortcuts::ShortcutEntry {
                    action: action_id.to_string(),
                    keys: keys_str,
                });
            }
        }
        crate::shortcuts::serialization::save_user_shortcuts(&overrides);
    }

    /// Get the sidebar x offset (SIDEBAR_WIDTH when visible, 0 when hidden).
    fn sidebar_offset(&self) -> f32 {
        if self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false) {
            SIDEBAR_WIDTH
        } else {
            0.0
        }
    }

    /// Transition from Picker to Workspace: open a project and initialize workspace.
    ///
    /// Per D-09: selecting a project in the picker opens it.
    /// Per D-11: projects auto-register on first open.
    fn open_project(&mut self, project_path: std::path::PathBuf) {
        info!("Opening project: {:?}", project_path);

        // D-11: Auto-register project
        self.project_registry.register(&project_path);

        self.app_state = AppState::Workspace;
        self.project_dir = Some(project_path.clone());
        self.picker_state = None;

        // Load saved project config (CFG-04)
        let project_config = crate::config::load_project_config(&project_path);

        // Apply saved theme or fall back to global preferences (D-01)
        if let Some(ref config) = project_config {
            if let Some(ref theme_name) = config.theme {
                if self.theme_registry.set_active(theme_name) {
                    self.theme = Theme::from_definition(self.theme_registry.active());
                    info!("Restored project theme: {}", theme_name);
                }
            }
        }
        if project_config.as_ref().and_then(|c| c.theme.as_ref()).is_none() {
            let global_prefs = crate::config::global::load_global_preferences();
            if self.theme_registry.set_active(&global_prefs.default_theme) {
                self.theme = Theme::from_definition(self.theme_registry.active());
            }
        }

        // Initialize grid and panels from saved config or defaults
        let cell_width = self.terminal_renderer.cell_width;
        let cell_height = self.terminal_renderer.cell_height;

        let (mut grid, panels_from_config) = if let Some(ref config) = project_config {
            if crate::config::persistence::validate_config(config) {
                let grid = GridLayout::from_config(&config.layout);
                let mut panels = Vec::new();
                let mut panel_id_counter: u64 = 0;

                for col in &config.layout.columns {
                    let caps = match col {
                        crate::config::ColumnConfig::Single(cap) => vec![cap],
                        crate::config::ColumnConfig::Stack { caps } => caps.iter().collect(),
                    };
                    for cap in caps {
                        let pid = PanelId(panel_id_counter);
                        panel_id_counter += 1;
                        let panel = match cap.cap_type {
                            crate::config::CapType::Terminal => Panel::new_terminal(pid),
                            crate::config::CapType::Canvas => {
                                let canvas_id = cap
                                    .file
                                    .as_ref()
                                    .and_then(|f| {
                                        std::path::Path::new(f)
                                            .file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                    })
                                    .unwrap_or_else(|| format!("canvas-{}", panel_id_counter));
                                Panel::new_canvas(pid, canvas_id)
                            }
                            crate::config::CapType::Markdown => {
                                let file_path = cap
                                    .file
                                    .as_ref()
                                    .map(|f| project_path.join(f));
                                if let Some(path) = file_path {
                                    Panel::new_markdown(pid, path)
                                } else {
                                    Panel::new_terminal(pid)
                                }
                            }
                        };
                        panels.push(panel);
                    }
                }

                if panels.is_empty() {
                    (GridLayout::new_single_panel(), vec![Panel::new_terminal(PanelId(0))])
                } else {
                    info!("Restored {} panels from saved config", panels.len());
                    (grid, panels)
                }
            } else {
                warn!("Saved config failed validation, using default layout");
                (GridLayout::new_single_panel(), vec![Panel::new_terminal(PanelId(0))])
            }
        } else {
            (GridLayout::new_single_panel(), vec![Panel::new_terminal(PanelId(0))])
        };

        // Compute grid layout
        if let Some(window) = &self.window {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                let w = size.width as f32 / self.scale_factor;
                let h = size.height as f32 / self.scale_factor;
                let grid_height = h - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
                grid.compute(w, grid_height.max(1.0));
                self.dividers = compute_dividers(&grid, w, grid_height.max(1.0));
            }
        }

        self.panels = panels_from_config;
        self.focused_panel = self.panels.first().map(|p| p.id);

        // Initialize bottom bar
        self.bottom_bar = Some(BottomBar::new(project_path.clone()));

        // Check if .myco folder exists
        let myco_dir = project_path.join(".myco");
        if !myco_dir.exists() {
            self.init_prompt = InitPrompt::Showing;
        }

        let mut tm = TerminalManager::new(project_path.clone());

        // Create canvas manager
        self.canvas_manager = Some(CanvasManager::new(project_path.clone()));

        // Create file sidebar state
        let mut sidebar = SidebarState::new(project_path.clone());
        sidebar.set_projects(self.project_registry.projects.clone());
        self.sidebar = Some(sidebar);

        // Start file watcher
        if let Some(proxy) = &self.proxy {
            match FileWatcher::new(&project_path, proxy.clone()) {
                Ok(watcher) => {
                    self.file_watcher = Some(watcher);
                }
                Err(e) => {
                    warn!("Failed to start file watcher: {}", e);
                }
            }
        }

        // Create terminals, canvases, and markdown viewers for restored panels
        for panel in &self.panels {
            match panel.panel_type {
                PanelType::Terminal => {
                    if let Some(node_id) = grid.find_node(panel.id) {
                        let (_, _, pw, ph) = grid.get_panel_rect(node_id);
                        let cols = ((pw - PANEL_CONTENT_PADDING * 2.0) / cell_width).max(2.0) as usize;
                        let rows = ((ph - PANEL_TITLE_HEIGHT) / cell_height).max(1.0) as usize;

                        // Use saved CWD from config if available
                        let terminal_cwd = project_config.as_ref().and_then(|config| {
                            let mut idx = 0u64;
                            for col in &config.layout.columns {
                                let caps = match col {
                                    crate::config::ColumnConfig::Single(cap) => vec![cap],
                                    crate::config::ColumnConfig::Stack { caps } => caps.iter().collect(),
                                };
                                for cap in caps {
                                    if PanelId(idx) == panel.id {
                                        return cap.cwd.as_ref().map(|cwd| project_path.join(cwd));
                                    }
                                    idx += 1;
                                }
                            }
                            None
                        });

                        if let Some(cwd) = terminal_cwd {
                            let terminal = crate::terminal::TerminalState::new(cols, rows, &cwd);
                            match terminal {
                                Ok(mut ts) => {
                                    ts.cell_width = cell_width;
                                    ts.cell_height = cell_height;
                                    tm.terminals.insert(panel.id, ts);
                                }
                                Err(e) => warn!("Failed to create terminal with saved CWD: {}", e),
                            }
                        } else {
                            if let Err(e) = tm.create_terminal(panel.id, cols, rows) {
                                warn!("Failed to create terminal: {}", e);
                            } else if let Some(ts) = tm.get_mut(&panel.id) {
                                ts.cell_width = cell_width;
                                ts.cell_height = cell_height;
                            }
                        }
                    }
                }
                PanelType::Markdown => {
                    if let Some(ref path) = panel.file_path {
                        if let Some(mm) = &mut self.markdown_manager {
                            if let Err(e) = mm.create_markdown(panel.id, path.clone()) {
                                warn!("Failed to create markdown viewer: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        self.terminal_manager = Some(tm);
        self.grid = Some(grid);

        // Initialize resource monitor for process health tracking (D-01, D-03)
        if self.resource_monitor.is_none() {
            if let Some(proxy) = &self.proxy {
                self.resource_monitor = Some(crate::monitor::ResourceMonitor::new(proxy.clone()));
                trace!("Resource monitor started");
            }
        }
        self.sync_child_pids();

        info!("Workspace initialized for project: {:?}", project_path);
    }

    /// Create a canvas panel with the given canvas_id.
    /// Shared between CreateCanvas action and sidebar-triggered canvas creation.
    fn create_canvas_with_id(&mut self, canvas_id: &str) {
        if let Some(focused_id) = self.focused_panel {
            if let Some(grid) = self.grid.as_mut() {
                if let Some(new_id) =
                    operations::split_panel(grid, focused_id, SplitDirection::Horizontal)
                {
                    let panel = Panel::new_canvas(new_id, canvas_id.to_string());
                    self.panels.push(panel);
                    self.focused_panel = Some(new_id);
                    self.recompute_layout();

                    // Create canvas webview
                    if let Some(bounds) = self.panel_content_bounds(new_id) {
                        if let (Some(cm), Some(window), Some(proxy)) =
                            (&mut self.canvas_manager, &self.window, &self.proxy)
                        {
                            if let Err(e) = cm.create_canvas(
                                new_id,
                                canvas_id,
                                window,
                                bounds,
                                proxy.clone(),
                            ) {
                                warn!("Failed to create canvas: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Recompute grid layout and divider positions.
    /// D-11: Subtracts sidebar width from available grid width when sidebar is visible.
    fn recompute_layout(&mut self) {
        if let (Some(grid), Some(window)) = (self.grid.as_mut(), self.window.as_ref()) {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                let w = size.width as f32 / self.scale_factor;
                let h = size.height as f32 / self.scale_factor;
                // Deduct title bar + stats bar from top, bottom bar from bottom
                let grid_height = h - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;

                // D-11: Subtract sidebar width when visible
                let sidebar_w = if self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false) {
                    SIDEBAR_WIDTH
                } else {
                    0.0
                };
                let grid_width = w - sidebar_w;

                grid.compute(grid_width, grid_height.max(1.0));
                self.dividers = compute_dividers(grid, grid_width, grid_height.max(1.0));
            }
        }

        // Resize canvas webviews to match new layout
        if let Some(cm) = &self.canvas_manager {
            for panel in &self.panels {
                if panel.panel_type == PanelType::Canvas {
                    if let Some(bounds) = self.panel_content_bounds(panel.id) {
                        cm.resize(&panel.id, bounds);
                    }
                }
            }
        }
    }

    /// Resize all terminals to match their panel dimensions and notify PTY.
    fn resize_terminals(&mut self) {
        if let (Some(grid), Some(tm)) = (&self.grid, &mut self.terminal_manager) {
            for &(node, panel_id) in grid.panel_nodes() {
                if let Some(ts) = tm.get_mut(&panel_id) {
                    let (_, _, pw, ph) = grid.get_panel_rect(node);
                    let usable_w = pw - PANEL_CONTENT_PADDING * 2.0;
                    let cols = (usable_w / ts.cell_width).max(2.0) as usize;
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

    /// Sync child PIDs from terminal states to panels and update resource monitor tracking.
    fn sync_child_pids(&mut self) {
        if let Some(tm) = &self.terminal_manager {
            let mut all_pids = Vec::new();
            for panel in &mut self.panels {
                if panel.panel_type == PanelType::Terminal {
                    if let Some(ts) = tm.get(&panel.id) {
                        panel.child_pid = ts.child_pid;
                        if let Some(pid) = ts.child_pid {
                            all_pids.push(pid);
                        }
                    }
                }
            }
            if let Some(monitor) = &self.resource_monitor {
                monitor.update_tracked_pids(all_pids);
            }
        }
    }

    /// Update resource dot tooltip state based on cursor position.
    fn update_tooltip_state(&mut self, lx: f32, ly: f32) {
        let grid = match &self.grid {
            Some(g) => g,
            None => {
                self.tooltip_state = None;
                return;
            }
        };
        let sidebar_offset = self.sidebar_offset();

        for &(node, panel_id) in grid.panel_nodes() {
            let (px, py, pw, _ph) = grid.get_panel_rect(node);
            let px = px + sidebar_offset;
            let py_offset = py + TOP_CHROME_HEIGHT;

            // Resource dot position: same as build_quads
            let close_x = px + pw - 40.0;
            let dot_x = close_x - 24.0;
            let dot_y = py_offset + 10.0;

            // Hit test: 8x8 dot with 4px margin
            if lx >= dot_x - 4.0
                && lx <= dot_x + 12.0
                && ly >= dot_y - 4.0
                && ly <= dot_y + 12.0
            {
                if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                    if let Some(pid) = panel.child_pid {
                        if let Some(state) = self.resource_states.get(&pid) {
                            // Keep existing tooltip if same panel (preserve hover_start)
                            if self
                                .tooltip_state
                                .as_ref()
                                .map(|t| t.panel_id == panel_id)
                                .unwrap_or(false)
                            {
                                // Update position and values
                                if let Some(ref mut tooltip) = self.tooltip_state {
                                    tooltip.cpu_percent = state.cpu_percent;
                                    tooltip.memory_bytes = state.memory_bytes;
                                    tooltip.x = dot_x - 80.0;
                                    tooltip.y = dot_y + 16.0;
                                }
                            } else {
                                self.tooltip_state = Some(TooltipState {
                                    panel_id,
                                    cpu_percent: state.cpu_percent,
                                    memory_bytes: state.memory_bytes,
                                    x: dot_x - 80.0,
                                    y: dot_y + 16.0,
                                    hover_start: Instant::now(),
                                });
                            }
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            return;
                        }
                    }
                }
            }
        }

        // No dot hovered -- clear tooltip
        if self.tooltip_state.is_some() {
            self.tooltip_state = None;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    /// Build quad instances for the current frame.
    ///
    /// Accepts pre-computed terminal snapshots to avoid re-snapshotting during text prep.
    #[tracing::instrument(skip_all, level = "trace")]
    fn build_quads(
        &self,
        width: f32,
        height: f32,
        snapshots: &HashMap<PanelId, TerminalSnapshot>,
        pill_data: &HashMap<PanelId, (String, Option<(String, Option<(usize, usize, usize)>)>)>,
    ) -> (Vec<QuadInstance>, Vec<TextLabel>) {
        let mut quads = Vec::new();
        let mut pill_label_buf: Vec<TextLabel> = Vec::new();
        let grid = match &self.grid {
            Some(g) => g,
            None => return (quads, pill_label_buf),
        };

        quads.push(QuadInstance {
            position: [0.0, 0.0],
            size: [width, TITLE_BAR_HEIGHT],
            color: self.theme.background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        let sidebar_offset = self.sidebar_offset();

        // Stats bar quads (below title bar, full width minus sidebar)
        {
            let stats_bar_x = sidebar_offset;
            let stats_bar_w = width - sidebar_offset;
            let stats_quads = self.stats_bar.build_quads(
                TITLE_BAR_HEIGHT,
                stats_bar_x,
                stats_bar_w,
                &self.theme,
            );
            quads.extend(stats_quads);
        }

        // Bottom bar quads (full width, pinned to bottom)
        if let Some(bottom_bar) = &self.bottom_bar {
            let bottom_bar_y = height - BOTTOM_BAR_HEIGHT;
            let bottom_quads = bottom_bar.build_quads(bottom_bar_y, width, &self.theme);
            quads.extend(bottom_quads);
        }

        // Render sidebar quads
        if let Some(sidebar) = &self.sidebar {
            if sidebar.visible {
                let sidebar_viewport_y = TOP_CHROME_HEIGHT;
                let sidebar_viewport_h = height - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
                let sidebar_quads = SidebarRenderer::build_quads(
                    sidebar,
                    sidebar_viewport_y,
                    sidebar_viewport_h,
                    &self.theme,
                );
                quads.extend(sidebar_quads);
            }
        }

        // Panel quads
        for &(node, panel_id) in grid.panel_nodes() {
            let (px, py, pw, ph) = grid.get_panel_rect(node);
            // Offset panel x position by sidebar width
            let px = px + sidebar_offset;
            let py_offset = py + TOP_CHROME_HEIGHT;

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
                color: [0.214, 0.024, 0.024, 0.6],
                corner_radius: 2.0,
                _padding: 0.0,
            });

            // Fullscreen button quad
            let fs_x = px + pw - 20.0;
            let fs_y = py_offset + 6.0;
            quads.push(QuadInstance {
                position: [fs_x, fs_y],
                size: [16.0, 16.0],
                color: [0.068, 0.043, 0.126, 0.6],
                corner_radius: 2.0,
                _padding: 0.0,
            });

            // Resource health dot (D-01): 8x8 circle in panel header
            if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                let dot_color = if let Some(pid) = panel.child_pid {
                    if let Some(state) = self.resource_states.get(&pid) {
                        crate::monitor::dot_color(state.cpu_percent, &self.theme)
                    } else {
                        self.theme.fg_secondary
                    }
                } else {
                    self.theme.fg_secondary
                };
                let dot_x = close_x - 24.0;
                let dot_y = py_offset + 10.0;
                quads.push(QuadInstance {
                    position: [dot_x, dot_y],
                    size: [8.0, 8.0],
                    color: dot_color,
                    corner_radius: 4.0,
                    _padding: 0.0,
                });
            }

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

            // Terminal-specific quads (cell backgrounds, cursor, context pills)
            if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                if panel.panel_type == PanelType::Terminal {
                    if let Some(tm) = &self.terminal_manager {
                        if let Some(ts) = tm.get(&panel_id) {
                            let content_y = py_offset + PANEL_TITLE_HEIGHT;
                            let content_h = ph - PANEL_TITLE_HEIGHT;
                            if let Some(snapshot) = snapshots.get(&panel_id) {
                                // Bottom-align offset (same calc as text cache)
                                let bottom_offset = if ts.scroll_offset == 0 {
                                    snapshot.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                                } else {
                                    0.0
                                };

                                let term_quads =
                                    self.terminal_renderer.build_terminal_quads(
                                        snapshot,
                                        px + PANEL_CONTENT_PADDING,
                                        content_y + bottom_offset,
                                        pw - PANEL_CONTENT_PADDING * 2.0,
                                        content_h,
                                        self.theme.panel_background,
                                        ts.cursor_blink_visible,
                                        ts.cell_width,
                                        ts.cell_height,
                                    );
                                quads.extend(term_quads);

                                // Context pills below the last content row
                                if let Some((display_cwd, git)) = pill_data.get(&panel_id) {
                                    let last_row = snapshot.last_content_row();
                                    let pill_y = content_y + bottom_offset
                                        + ((last_row + 1) as f32 * ts.cell_height);
                                    let panel_bottom = content_y + content_h;
                                    if pill_y + TerminalRenderer::PILL_ROW_HEIGHT <= panel_bottom {
                                        let content_w = pw - PANEL_CONTENT_PADDING * 2.0;
                                        let (pill_quads, pill_labels) = self.terminal_renderer
                                            .build_context_pills(
                                                display_cwd,
                                                git.as_ref(),
                                                px + PANEL_CONTENT_PADDING,
                                                pill_y,
                                                content_w,
                                            );
                                        quads.extend(pill_quads);
                                        pill_label_buf.extend(pill_labels);
                                    }
                                }
                            }

                            // Selection highlight and copy flash quads
                            {
                                let term = ts.term.lock();
                                let flash_opacity = ts.copy_flash_opacity();
                                let sel_quads =
                                    self.terminal_renderer.build_selection_quads(
                                        &term,
                                        px + PANEL_CONTENT_PADDING,
                                        content_y + if let Some(snapshot) = snapshots.get(&panel_id) {
                                            if ts.scroll_offset == 0 {
                                                snapshot.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                                            } else { 0.0 }
                                        } else { 0.0 },
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
                                    color: [0.509, 0.291, 0.946, 0.7],
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

                                let search_bottom_off = if let Some(snap) = snapshots.get(&panel_id) {
                                    if ts.scroll_offset == 0 {
                                        snap.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                                    } else { 0.0 }
                                } else { 0.0 };
                                let search_quads = self
                                    .terminal_renderer
                                    .build_search_quads(
                                        ts.search.match_positions(),
                                        ts.search.current_match_index(),
                                        px + PANEL_CONTENT_PADDING,
                                        content_y + search_bottom_off,
                                        ts.cell_width,
                                        ts.cell_height,
                                        display_offset,
                                        screen_lines,
                                    );
                                quads.extend(search_quads);
                            }

                            // History search overlay quads (Ctrl+R)
                            if ts.autocomplete.history_search_is_open() {
                                let results = ts.autocomplete.history_search_results();
                                let visible_count = results.len().min(10);
                                let overlay_h = 32.0 + (visible_count as f32 * 28.0);
                                let overlay_w = 400.0_f32.min(pw - 20.0).max(200.0);
                                let overlay_x = px + (pw - overlay_w) / 2.0;
                                let overlay_y = content_y + 10.0;

                                // Background
                                quads.push(QuadInstance {
                                    position: [overlay_x, overlay_y],
                                    size: [overlay_w, overlay_h],
                                    color: [0.015, 0.016, 0.025, 0.97],
                                    corner_radius: 6.0,
                                    _padding: 0.0,
                                });

                                // Selected result highlight
                                let selected = ts.autocomplete.history_search_selected();
                                if selected < visible_count {
                                    quads.push(QuadInstance {
                                        position: [
                                            overlay_x + 4.0,
                                            overlay_y + 32.0 + (selected as f32 * 28.0),
                                        ],
                                        size: [overlay_w - 8.0, 26.0],
                                        color: [0.100, 0.059, 0.187, 0.8],
                                        corner_radius: 3.0,
                                        _padding: 0.0,
                                    });
                                }
                            }
                        }
                    }
                }

                // Markdown-specific quads (code block backgrounds, blockquote borders, HRs)
                if panel.panel_type == PanelType::Markdown {
                    if let Some(mm) = &self.markdown_manager {
                        if let Some(state) = mm.get(&panel_id) {
                            let (vx, vy, vw, vh) = (
                                px,
                                py_offset + PANEL_TITLE_HEIGHT,
                                pw,
                                ph - PANEL_TITLE_HEIGHT,
                            );
                            let md_quads = MarkdownRenderer::build_quads(
                                &state.blocks,
                                &state.block_heights,
                                state.scroll_offset,
                                vx,
                                vy,
                                vw,
                                vh,
                                &self.theme,
                            );
                            quads.extend(md_quads);
                        }
                    }
                }
            }
        }

        // D-16: Unfocused panel overlay (semi-transparent black on unfocused GPU panels)
        for &(node, panel_id) in grid.panel_nodes() {
            if Some(panel_id) == self.focused_panel {
                continue; // Skip focused panel
            }
            // Canvas desaturation handled via CSS (already in Plan 01)
            let is_canvas = self
                .panels
                .iter()
                .any(|p| p.id == panel_id && p.panel_type == PanelType::Canvas);
            if is_canvas {
                continue;
            }
            let (px, py, pw, ph) = grid.get_panel_rect(node);
            quads.push(QuadInstance {
                position: [px + sidebar_offset, py + TOP_CHROME_HEIGHT],
                size: [pw, ph],
                color: self.theme.unfocused_overlay,
                corner_radius: 0.0,
                _padding: 0.0,
            });
        }

        // Divider quads (offset by sidebar width)
        for (i, div) in self.dividers.dividers.iter().enumerate() {
            let is_hovered = self.mouse_state.hovered_divider == Some(i);
            let color = if is_hovered {
                self.theme.divider_hover
            } else {
                self.theme.divider
            };

            match div.orientation {
                Orientation::Vertical => {
                    let grid_height = height - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
                    quads.push(QuadInstance {
                        position: [
                            div.position - DIVIDER_VISUAL_WIDTH / 2.0 + sidebar_offset,
                            TOP_CHROME_HEIGHT,
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
                            sidebar_offset,
                            div.position + TOP_CHROME_HEIGHT
                                - DIVIDER_VISUAL_WIDTH / 2.0,
                        ],
                        size: [width - sidebar_offset, DIVIDER_VISUAL_WIDTH],
                        color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }

        // Init prompt overlay
        if self.init_prompt == InitPrompt::Showing {
            // Semi-transparent backdrop
            quads.push(QuadInstance {
                position: [0.0, 0.0],
                size: [width, height],
                color: [0.0, 0.0, 0.0, 0.5],
                corner_radius: 0.0,
                _padding: 0.0,
            });
            // Dialog box
            let dialog_w = 420.0;
            let dialog_h = 140.0;
            let dialog_x = (width - dialog_w) / 2.0;
            let dialog_y = (height - dialog_h) / 2.0;
            quads.push(QuadInstance {
                position: [dialog_x, dialog_y],
                size: [dialog_w, dialog_h],
                color: [0.058, 0.063, 0.102, 1.0],
                corner_radius: 8.0,
                _padding: 0.0,
            });
            // Border accent
            quads.push(QuadInstance {
                position: [dialog_x, dialog_y],
                size: [dialog_w, 3.0],
                color: [0.509, 0.291, 0.946, 1.0],
                corner_radius: 0.0,
                _padding: 0.0,
            });
        }

        // Settings overlay (renders on top of everything except title bar/bottom bar)
        if self.settings.visible {
            let viewport_y = TOP_CHROME_HEIGHT;
            let viewport_h = height - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
            let settings_quads = SettingsRenderer::build_quads(
                &self.settings,
                viewport_y,
                viewport_h,
                width,
                &self.theme,
            );
            quads.extend(settings_quads);
        }

        // Toast notification quads (bottom-right stack, renders on top of panels)
        crate::toast::renderer::build_toast_quads(
            &self.toast_manager,
            width,
            height,
            &self.theme,
            &mut quads,
        );

        // Tooltip quad (resource dot hover)
        if let Some(ref tooltip) = self.tooltip_state {
            if tooltip.hover_start.elapsed() >= Duration::from_millis(300) {
                // Tooltip background
                quads.push(QuadInstance {
                    position: [tooltip.x, tooltip.y],
                    size: [160.0, 52.0],
                    color: self.theme.bg_secondary,
                    corner_radius: 4.0,
                    _padding: 0.0,
                });
                // Tooltip border
                quads.push(QuadInstance {
                    position: [tooltip.x, tooltip.y],
                    size: [160.0, 1.0],
                    color: self.theme.border,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }
        }

        (quads, pill_label_buf)
    }

    /// Build text labels for the current frame.
    #[tracing::instrument(skip_all, level = "trace")]
    #[allow(clippy::unused_self)]
    fn build_labels(&self, width: f32, height: f32, snapshots: &HashMap<PanelId, TerminalSnapshot>) -> Vec<TextLabel> {
        let mut labels = Vec::new();
        let grid = match &self.grid {
            Some(g) => g,
            None => return labels,
        };

        // Title bar breadcrumb (D-14): "Myco > Untitled Project"
        labels.push(TextLabel {
            text: "Myco > Untitled Project".to_string(),
            x: 100.0,
            y: 10.0,
            width: 300.0,
            height: TITLE_BAR_HEIGHT,
            font_size: 13.0,
            color: glyphon::Color::rgba(
                linear_to_srgb_u8(self.theme.title_bar_text[0]),
                linear_to_srgb_u8(self.theme.title_bar_text[1]),
                linear_to_srgb_u8(self.theme.title_bar_text[2]),
                linear_to_srgb_u8(self.theme.title_bar_text[3]),
            ),
        });

        let sidebar_offset = self.sidebar_offset();

        // Stats bar labels
        {
            let stats_bar_x = sidebar_offset;
            let stats_bar_w = width - sidebar_offset;
            let stats_labels = self.stats_bar.build_labels(
                TITLE_BAR_HEIGHT,
                stats_bar_x,
                stats_bar_w,
                &self.theme,
            );
            labels.extend(stats_labels);
        }

        // Bottom bar labels
        if let Some(bottom_bar) = &self.bottom_bar {
            let bottom_bar_y = height - BOTTOM_BAR_HEIGHT;
            let bottom_labels = bottom_bar.build_labels(bottom_bar_y, width, &self.theme);
            labels.extend(bottom_labels);
        }

        // Panel labels
        for &(node, panel_id) in grid.panel_nodes() {
            let (px, py, pw, ph) = grid.get_panel_rect(node);
            let px = px + sidebar_offset;
            let py_offset = py + TOP_CHROME_HEIGHT;

            if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
                // Panel title bar label (show title for markdown, type for others)
                let title_text = match panel.panel_type {
                    PanelType::Markdown | PanelType::Canvas => panel.title.clone(),
                    _ => panel.panel_type.to_string(),
                };
                labels.push(TextLabel {
                    text: title_text,
                    x: px + 8.0,
                    y: py_offset + 4.0,
                    width: pw - 60.0,
                    height: 20.0,
                    font_size: 12.0,
                    color: glyphon::Color::rgba(
                        linear_to_srgb_u8(self.theme.title_bar_text[0]),
                        linear_to_srgb_u8(self.theme.title_bar_text[1]),
                        linear_to_srgb_u8(self.theme.title_bar_text[2]),
                        linear_to_srgb_u8(self.theme.title_bar_text[3]),
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
                    color: glyphon::Color::rgba(248, 248, 242, 255),
                });

                // Fullscreen button label
                labels.push(TextLabel {
                    text: "\u{25A1}".to_string(),
                    x: px + pw - 17.0,
                    y: py_offset + 6.0,
                    width: 16.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: glyphon::Color::rgba(248, 248, 242, 255),
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
                                        linear_to_srgb_u8(self.theme.panel_label_text[0]),
                                        linear_to_srgb_u8(self.theme.panel_label_text[1]),
                                        linear_to_srgb_u8(self.theme.panel_label_text[2]),
                                        linear_to_srgb_u8(self.theme.panel_label_text[3]),
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
                                    color: glyphon::Color::rgba(248, 248, 242, 255),
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
                                    color: glyphon::Color::rgba(248, 248, 242, 255),
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
                                            139, 147, 164, 255,
                                        ),
                                    });
                                }
                            }

                            // Ghost text autocomplete label
                            if let Some(ghost) = ts.autocomplete.ghost_text() {
                                let in_alt = ts.term.lock().mode().contains(
                                    alacritty_terminal::term::TermMode::ALT_SCREEN,
                                );
                                if !in_alt && !ghost.is_empty() {
                                    let term = ts.term.lock();
                                    let cursor = term.renderable_content().cursor.point;
                                    drop(term);
                                    let content_y = py_offset + PANEL_TITLE_HEIGHT;
                                    let content_h = ph - PANEL_TITLE_HEIGHT;
                                    let ghost_offset = if let Some(snap) = snapshots.get(&panel_id) {
                                        if ts.scroll_offset == 0 {
                                            snap.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                                        } else { 0.0 }
                                    } else { 0.0 };
                                    let ghost_x = px + PANEL_CONTENT_PADDING
                                        + (cursor.column.0 as f32) * ts.cell_width;
                                    let ghost_y =
                                        content_y + ghost_offset + (cursor.line.0 as f32) * ts.cell_height;
                                    labels.push(TextLabel {
                                        text: ghost.to_string(),
                                        x: ghost_x,
                                        y: ghost_y,
                                        width: pw - (ghost_x - px),
                                        height: ts.cell_height,
                                        font_size: ts.font_size,
                                        color: glyphon::Color::rgba(98, 114, 164, 140),
                                    });
                                }
                            }

                            // History search overlay labels (Ctrl+R)
                            if ts.autocomplete.history_search_is_open() {
                                let content_y = py_offset + PANEL_TITLE_HEIGHT;
                                let results = ts.autocomplete.history_search_results();
                                let visible_count = results.len().min(10);
                                let overlay_w = 400.0_f32.min(pw - 20.0).max(200.0);
                                let overlay_x = px + (pw - overlay_w) / 2.0;
                                let overlay_y = content_y + 10.0;

                                // Search input label
                                let query = ts.autocomplete.history_search_query();
                                let display_query = if query.is_empty() {
                                    "Search history...".to_string()
                                } else {
                                    query.to_string()
                                };
                                labels.push(TextLabel {
                                    text: format!("  {}", display_query),
                                    x: overlay_x + 8.0,
                                    y: overlay_y + 8.0,
                                    width: overlay_w - 16.0,
                                    height: 16.0,
                                    font_size: 13.0,
                                    color: if query.is_empty() {
                                        glyphon::Color::rgba(98, 114, 164, 255)
                                    } else {
                                        glyphon::Color::rgba(248, 248, 242, 255)
                                    },
                                });

                                // Result entries
                                for (i, result) in
                                    results.iter().take(visible_count).enumerate()
                                {
                                    let entry_y =
                                        overlay_y + 32.0 + (i as f32 * 28.0) + 5.0;
                                    let truncated: String =
                                        result.chars().take(60).collect();
                                    labels.push(TextLabel {
                                        text: truncated,
                                        x: overlay_x + 12.0,
                                        y: entry_y,
                                        width: overlay_w - 24.0,
                                        height: 16.0,
                                        font_size: 12.0,
                                        color: glyphon::Color::rgba(
                                            248, 248, 242, 255,
                                        ),
                                    });
                                }
                            }
                        }
                    }
                } else if panel.panel_type != PanelType::Markdown {
                    // Centered type label in panel body (D-03) for non-terminal, non-markdown panels
                    // Markdown panels render their own content via markdown_renderer
                    let center_y = py_offset + ph / 2.0 - 7.0;
                    labels.push(TextLabel {
                        text: panel.title.clone(),
                        x: px,
                        y: center_y,
                        width: pw,
                        height: 28.0,
                        font_size: 14.0,
                        color: glyphon::Color::rgba(
                            linear_to_srgb_u8(self.theme.panel_label_text[0]),
                            linear_to_srgb_u8(self.theme.panel_label_text[1]),
                            linear_to_srgb_u8(self.theme.panel_label_text[2]),
                            linear_to_srgb_u8(self.theme.panel_label_text[3]),
                        ),
                    });
                }
            }
        }

        // Init prompt labels
        if self.init_prompt == InitPrompt::Showing {
            let dialog_w = 420.0;
            let dialog_h = 140.0;
            let dialog_x = (width - dialog_w) / 2.0;
            let dialog_y = (height - dialog_h) / 2.0;
            let text_color = glyphon::Color::rgba(248, 248, 242, 255);
            let dim_color = glyphon::Color::rgba(98, 114, 164, 255);

            labels.push(TextLabel {
                text: "Initialize project?".to_string(),
                x: dialog_x + 20.0,
                y: dialog_y + 16.0,
                width: dialog_w - 40.0,
                height: 24.0,
                font_size: 16.0,
                color: text_color,
            });
            labels.push(TextLabel {
                text: "Create .myco folder with canvas and AI context files.".to_string(),
                x: dialog_x + 20.0,
                y: dialog_y + 48.0,
                width: dialog_w - 40.0,
                height: 20.0,
                font_size: 13.0,
                color: dim_color,
            });
            labels.push(TextLabel {
                text: "[Y / Enter] Initialize    [N / Esc] Skip".to_string(),
                x: dialog_x + 20.0,
                y: dialog_y + 100.0,
                width: dialog_w - 40.0,
                height: 20.0,
                font_size: 12.0,
                color: dim_color,
            });
        }

        // Toast notification labels (bottom-right stack)
        crate::toast::renderer::build_toast_labels(
            &self.toast_manager,
            width,
            height,
            &self.theme,
            &mut labels,
        );

        // Tooltip labels (resource dot hover)
        if let Some(ref tooltip) = self.tooltip_state {
            if tooltip.hover_start.elapsed() >= Duration::from_millis(300) {
                let fg_primary = glyphon::Color::rgba(
                    linear_to_srgb_u8(self.theme.fg_primary[0]),
                    linear_to_srgb_u8(self.theme.fg_primary[1]),
                    linear_to_srgb_u8(self.theme.fg_primary[2]),
                    255,
                );
                labels.push(TextLabel {
                    text: format!("CPU: {:.1}%", tooltip.cpu_percent),
                    x: tooltip.x + 12.0,
                    y: tooltip.y + 8.0,
                    width: 136.0,
                    height: 18.0,
                    font_size: 13.0,
                    color: fg_primary,
                });
                let mem_mb = tooltip.memory_bytes as f64 / 1_048_576.0;
                labels.push(TextLabel {
                    text: format!("RAM: {:.1} MB", mem_mb),
                    x: tooltip.x + 12.0,
                    y: tooltip.y + 28.0,
                    width: 136.0,
                    height: 18.0,
                    font_size: 13.0,
                    color: fg_primary,
                });
            }
        }

        // Settings overlay labels (renders on top)
        if self.settings.visible {
            let viewport_y = TOP_CHROME_HEIGHT;
            let viewport_h = height - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
            let settings_labels = SettingsRenderer::build_labels(
                &self.settings,
                viewport_y,
                viewport_h,
                width,
                &self.theme,
            );
            labels.extend(settings_labels);

            // Shortcut badge labels (with actual binding data from registry)
            let badge_labels = SettingsRenderer::build_shortcuts_badge_labels(
                &self.settings,
                viewport_y,
                width,
                &self.theme,
                &self.shortcut_registry,
            );
            labels.extend(badge_labels);
        }

        labels
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::TerminalEvent => {}
            UserEvent::CanvasMessage(panel_id, msg) => {
                self.process_action(InputAction::CanvasIpcMessage { panel_id, message: msg });
            }
            UserEvent::ResourceUpdate(updates) => {
                for update in updates {
                    self.resource_states.insert(
                        update.pid,
                        crate::monitor::ResourceState {
                            cpu_percent: update.cpu_percent,
                            memory_bytes: update.memory_bytes,
                            last_updated: Instant::now(),
                        },
                    );
                }
            }
            UserEvent::FileChanged(paths) => {
                if let Some(mm) = &mut self.markdown_manager {
                    mm.handle_file_changed(&paths);
                }
                if let Some(sidebar) = &mut self.sidebar {
                    sidebar.refresh_file_tree();
                }
            }
            #[cfg(target_os = "macos")]
            UserEvent::MenuAction(tag) => {
                self.handle_menu_action(tag);
            }
        }
        // Drain pending actions (from IPC shortcut forwarding)
        while let Some(action) = self.pending_actions.pop() {
            self.process_action(action);
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only initialize once
        if self.window.is_some() {
            return;
        }

        info!("Application resumed -- creating window and GPU state");

        let window = create_window(event_loop);
        self.scale_factor = window.scale_factor() as f32;

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

        // D-10: Check for CLI argument (myco /path/to/project)
        let cli_project_dir = std::env::args().nth(1).map(std::path::PathBuf::from);

        // Determine project directory: CLI arg or CWD
        let project_dir = if let Some(ref cli_path) = cli_project_dir {
            if cli_path.exists() {
                cli_path.clone()
            } else {
                warn!("CLI path does not exist: {:?}, falling back to CWD", cli_path);
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
        };

        // If no CLI argument, show picker instead of workspace (D-09)
        if cli_project_dir.is_none() {
            info!("No CLI argument — showing project picker");

            // Apply global theme preferences for picker view
            let global_prefs = crate::config::global::load_global_preferences();
            if self.theme_registry.set_active(&global_prefs.default_theme) {
                self.theme = Theme::from_definition(self.theme_registry.active());
            }

            self.app_state = AppState::Picker;
            self.picker_state = Some(PickerState::new(self.project_registry.projects.clone()));

            self.window = Some(window);
            self.renderer = Some(renderer);

            // Set up native menu bar
            #[cfg(target_os = "macos")]
            {
                if let Some(proxy) = &self.proxy {
                    self.menu_state = Some(crate::platform::menu::setup_menu_bar(proxy.clone()));
                }
            }

            info!("Picker mode initialized with {} projects", self.project_registry.projects.len());
            return;
        }

        // D-10: CLI argument present — open workspace directly, skip picker
        info!("CLI argument present — opening workspace directly");
        self.app_state = AppState::Workspace;
        self.project_dir = Some(project_dir.clone());

        // D-11: Auto-register project
        self.project_registry.register(&project_dir);

        // Load saved project config (CFG-04)
        let project_config = crate::config::load_project_config(&project_dir);

        // Apply saved theme or fall back to global preferences (D-01)
        if let Some(ref config) = project_config {
            if let Some(ref theme_name) = config.theme {
                if self.theme_registry.set_active(theme_name) {
                    self.theme = Theme::from_definition(self.theme_registry.active());
                    info!("Restored project theme: {}", theme_name);
                }
            }
        }
        if project_config.as_ref().and_then(|c| c.theme.as_ref()).is_none() {
            let global_prefs = crate::config::global::load_global_preferences();
            if self.theme_registry.set_active(&global_prefs.default_theme) {
                self.theme = Theme::from_definition(self.theme_registry.active());
                info!("Applied global default theme: {}", global_prefs.default_theme);
            }
        }

        // Initialize grid and panels from saved config or defaults
        let (mut grid, panels_from_config) = if let Some(ref config) = project_config {
            if crate::config::persistence::validate_config(config) {
                let grid = GridLayout::from_config(&config.layout);
                let mut panels = Vec::new();
                let mut panel_id_counter: u64 = 0;

                for col in &config.layout.columns {
                    let caps = match col {
                        crate::config::ColumnConfig::Single(cap) => vec![cap],
                        crate::config::ColumnConfig::Stack { caps } => caps.iter().collect(),
                    };
                    for cap in caps {
                        let pid = PanelId(panel_id_counter);
                        panel_id_counter += 1;
                        let panel = match cap.cap_type {
                            crate::config::CapType::Terminal => Panel::new_terminal(pid),
                            crate::config::CapType::Canvas => {
                                let canvas_id = cap
                                    .file
                                    .as_ref()
                                    .and_then(|f| {
                                        std::path::Path::new(f)
                                            .file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                    })
                                    .unwrap_or_else(|| format!("canvas-{}", panel_id_counter));
                                Panel::new_canvas(pid, canvas_id)
                            }
                            crate::config::CapType::Markdown => {
                                let file_path = cap
                                    .file
                                    .as_ref()
                                    .map(|f| project_dir.join(f));
                                if let Some(path) = file_path {
                                    Panel::new_markdown(pid, path)
                                } else {
                                    Panel::new_terminal(pid)
                                }
                            }
                        };
                        panels.push(panel);
                    }
                }

                if panels.is_empty() {
                    (GridLayout::new_single_panel(), vec![Panel::new_terminal(PanelId(0))])
                } else {
                    info!("Restored {} panels from saved config", panels.len());
                    (grid, panels)
                }
            } else {
                warn!("Saved config failed validation, using default layout");
                (GridLayout::new_single_panel(), vec![Panel::new_terminal(PanelId(0))])
            }
        } else {
            (GridLayout::new_single_panel(), vec![Panel::new_terminal(PanelId(0))])
        };

        let size = window.inner_size();
        if size.width > 0 && size.height > 0 {
            let w = size.width as f32 / self.scale_factor;
            let h = size.height as f32 / self.scale_factor;
            let grid_height = h - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
            grid.compute(w, grid_height.max(1.0));
            self.dividers = compute_dividers(&grid, w, grid_height.max(1.0));
        }

        self.panels = panels_from_config;
        self.focused_panel = self.panels.first().map(|p| p.id);

        // Initialize bottom bar with project directory (D-07)
        self.bottom_bar = Some(BottomBar::new(project_dir.clone()));

        // Check if .myco folder exists — if not, prompt user to initialize
        let myco_dir = project_dir.join(".myco");
        if !myco_dir.exists() {
            self.init_prompt = InitPrompt::Showing;
            info!("No .myco folder found — showing initialization prompt");
        }

        let mut tm = TerminalManager::new(project_dir.clone());

        // Create canvas manager for TLDraw webview panels
        self.canvas_manager = Some(CanvasManager::new(project_dir.clone()));

        // Create file sidebar state with project registry data
        let mut sidebar = SidebarState::new(project_dir.clone());
        sidebar.set_projects(self.project_registry.projects.clone());
        self.sidebar = Some(sidebar);

        // Start file watcher for live markdown updates (CAP-04)
        if let Some(proxy) = &self.proxy {
            match FileWatcher::new(&project_dir, proxy.clone()) {
                Ok(watcher) => {
                    self.file_watcher = Some(watcher);
                }
                Err(e) => {
                    // No user-visible error per UI-SPEC: log via tracing::warn
                    warn!("Failed to start file watcher: {}", e);
                }
            }
        }

        // Create terminals, canvases, and markdown viewers for restored panels
        for panel in &self.panels {
            match panel.panel_type {
                PanelType::Terminal => {
                    if let Some(node_id) = grid.find_node(panel.id) {
                        let (_, _, pw, ph) = grid.get_panel_rect(node_id);
                        let cols = ((pw - PANEL_CONTENT_PADDING * 2.0) / cell_width).max(2.0) as usize;
                        let rows = ((ph - PANEL_TITLE_HEIGHT) / cell_height).max(1.0) as usize;

                        // Use saved CWD from config if available, otherwise project dir
                        let terminal_cwd = project_config.as_ref().and_then(|config| {
                            let mut idx = 0u64;
                            for col in &config.layout.columns {
                                let caps = match col {
                                    crate::config::ColumnConfig::Single(cap) => vec![cap],
                                    crate::config::ColumnConfig::Stack { caps } => caps.iter().collect(),
                                };
                                for cap in caps {
                                    if PanelId(idx) == panel.id {
                                        return cap.cwd.as_ref().map(|cwd| project_dir.join(cwd));
                                    }
                                    idx += 1;
                                }
                            }
                            None
                        });

                        // Create terminal with saved or default CWD
                        if let Some(cwd) = terminal_cwd {
                            // We need a temporary terminal manager with different CWD
                            let terminal = crate::terminal::TerminalState::new(
                                cols, rows, &cwd,
                            );
                            match terminal {
                                Ok(mut ts) => {
                                    ts.cell_width = cell_width;
                                    ts.cell_height = cell_height;
                                    tm.terminals.insert(panel.id, ts);
                                }
                                Err(e) => warn!("Failed to create terminal with saved CWD: {}", e),
                            }
                        } else {
                            if let Err(e) = tm.create_terminal(panel.id, cols, rows) {
                                warn!("Failed to create terminal: {}", e);
                            } else if let Some(ts) = tm.get_mut(&panel.id) {
                                ts.cell_width = cell_width;
                                ts.cell_height = cell_height;
                            }
                        }
                    }
                }
                PanelType::Markdown => {
                    if let Some(ref path) = panel.file_path {
                        if let Some(mm) = &mut self.markdown_manager {
                            if let Err(e) = mm.create_markdown(panel.id, path.clone()) {
                                warn!("Failed to create markdown viewer: {}", e);
                            }
                        }
                    }
                }
                _ => {
                    // Canvas and Placeholder panels are handled after window is stored
                }
            }
        }

        self.terminal_manager = Some(tm);
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.grid = Some(grid);

        // Initialize resource monitor for process health tracking (D-01, D-03)
        if self.resource_monitor.is_none() {
            if let Some(proxy) = &self.proxy {
                self.resource_monitor = Some(crate::monitor::ResourceMonitor::new(proxy.clone()));
                trace!("Resource monitor started");
            }
        }
        self.sync_child_pids();

        // Set up native menu bar
        #[cfg(target_os = "macos")]
        {
            if let Some(proxy) = &self.proxy {
                self.menu_state = Some(crate::platform::menu::setup_menu_bar(proxy.clone()));
            }
        }

        info!("Application initialization complete with terminal");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match &event {
            WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::Resized(_)
            | WindowEvent::Focused(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
        match event {
            WindowEvent::CloseRequested => {
                // Save config on exit (D-07: persist layout before closing)
                if let (Some(grid), Some(project_dir)) = (&self.grid, &self.project_dir) {
                    let config = crate::config::ProjectConfig::from_current_state(
                        grid,
                        &self.panels,
                        self.terminal_manager.as_ref(),
                        project_dir,
                        Some(&self.theme_registry.active().name),
                    );
                    crate::config::save_project_config(project_dir, &config);
                    info!("Saved project config on exit");
                }
                info!("Close requested -- exiting");
                event_loop.exit();
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::CursorMoved { position, .. } => {
                let lx = position.x / self.scale_factor as f64;
                let ly = position.y / self.scale_factor as f64;

                // Route cursor to picker when in picker mode
                if self.app_state == AppState::Picker {
                    self.mouse_state.cursor_x = lx;
                    self.mouse_state.cursor_y = ly;
                    if let Some(picker) = &mut self.picker_state {
                        if let Some(window) = &self.window {
                            let size = window.inner_size();
                            let vw = size.width as f32 / self.scale_factor;
                            let vh = size.height as f32 / self.scale_factor;
                            let prev_hovered = picker.hovered;
                            picker.hovered = picker.entry_at(lx as f32, ly as f32, vw, vh);
                            if picker.hovered != prev_hovered {
                                window.request_redraw();
                            }
                        }
                    }
                    return;
                }

                // Route cursor to settings overlay when visible
                if self.settings.visible {
                    let viewport_y = TOP_CHROME_HEIGHT;
                    if self.settings.update_hover(lx as f32, ly as f32, viewport_y) {
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    // Also update mouse state position for click handling
                    self.mouse_state.cursor_x = lx;
                    self.mouse_state.cursor_y = ly;
                    return;
                }

                // Update sidebar hover state
                let sidebar_visible = self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false);
                if sidebar_visible && (lx as f32) < SIDEBAR_WIDTH && (ly as f32) > TOP_CHROME_HEIGHT {
                    let sidebar_y = ly as f32 - TOP_CHROME_HEIGHT;
                    if let Some(sidebar) = &mut self.sidebar {
                        let prev = sidebar.hovered;
                        sidebar.hovered = sidebar.entry_at_y(sidebar_y);
                        if sidebar.hovered != prev {
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                } else if sidebar_visible {
                    if let Some(sidebar) = &mut self.sidebar {
                        if sidebar.hovered.is_some() {
                            sidebar.hovered = None;
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }

                if let Some(grid) = &self.grid {
                    let actions = self.mouse_state.on_cursor_moved(
                        lx,
                        ly,
                        &self.dividers,
                        grid,
                        TOP_CHROME_HEIGHT,
                    );
                    let actions: Vec<_> = actions;
                    for action in actions {
                        self.process_action(action);
                    }
                }

                // Resource dot tooltip hover tracking (D-02)
                self.update_tooltip_state(lx as f32, ly as f32);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Route mouse clicks to picker when in picker mode (D-09)
                if self.app_state == AppState::Picker
                    && state == ElementState::Pressed
                    && button == MouseButton::Left
                {
                    let lx = self.mouse_state.cursor_x as f32;
                    let ly = self.mouse_state.cursor_y as f32;
                    // Get viewport size before mutable operations
                    let (vw, vh) = self.window.as_ref()
                        .map(|w| {
                            let s = w.inner_size();
                            (s.width as f32 / self.scale_factor, s.height as f32 / self.scale_factor)
                        })
                        .unwrap_or((800.0, 600.0));
                    let picker_action = self.picker_state.as_mut()
                        .map(|picker| picker.handle_click(lx, ly, vw, vh));
                    if let Some(action) = picker_action {
                        match action {
                            PickerAction::OpenProject(path) => {
                                self.open_project(path);
                            }
                            PickerAction::OpenFolderDialog => {
                                info!("Open folder dialog requested (deferred)");
                            }
                            PickerAction::LocateProject(_idx) => {
                                info!("Locate project requested (deferred)");
                            }
                            PickerAction::None => {}
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }

                // Block mouse input while init prompt is showing
                if self.init_prompt == InitPrompt::Showing {
                    return;
                }

                // Route mouse clicks to settings overlay when visible
                if self.settings.visible
                    && state == ElementState::Pressed
                    && button == MouseButton::Left
                {
                    let lx = self.mouse_state.cursor_x as f32;
                    let ly = self.mouse_state.cursor_y as f32;
                    let viewport_y = TOP_CHROME_HEIGHT;

                    // Check toast Undo click first (overlays on top)
                    if let Some(window) = &self.window {
                        let size = window.inner_size();
                        let width = size.width as f32 / self.scale_factor;
                        let height = size.height as f32 / self.scale_factor;
                        let viewport_h = height - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
                        if self.settings.toast_undo_at(lx, ly, viewport_y, viewport_h, width) {
                            self.settings.handle_undo(&mut self.shortcut_registry);
                            self.save_shortcut_overrides();
                            window.request_redraw();
                            return;
                        }
                    }

                    let result = self.settings.handle_click(lx, ly, viewport_y);
                    match result {
                        SettingsClickResult::ThemeSelected(name) => {
                            self.pending_actions.push(InputAction::ThemeSwitch { theme_name: name });
                        }
                        SettingsClickResult::ProjectThemeChanged(theme_opt) => {
                            // Apply project theme override and mark for auto-save
                            if let Some(ref name) = theme_opt {
                                self.pending_actions.push(InputAction::ThemeSwitch { theme_name: name.clone() });
                            } else {
                                // Revert to global default theme
                                let global_prefs = crate::config::global::load_global_preferences();
                                self.pending_actions.push(InputAction::ThemeSwitch { theme_name: global_prefs.default_theme });
                            }
                            self.auto_save.mark_dirty();
                        }
                        SettingsClickResult::ShortcutRecordingStarted
                        | SettingsClickResult::SectionChanged
                        | SettingsClickResult::Consumed => {}
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }

                let lx = self.mouse_state.cursor_x as f32;
                let ly = self.mouse_state.cursor_y as f32;

                // Check if click is in the sidebar region
                let sidebar_visible = self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false);
                if sidebar_visible
                    && lx < SIDEBAR_WIDTH
                    && ly > TOP_CHROME_HEIGHT
                    && state == ElementState::Pressed
                    && button == MouseButton::Left
                {
                    let sidebar_y = ly - TOP_CHROME_HEIGHT;
                    // Handle sidebar click
                    if let Some(sidebar) = &mut self.sidebar {
                        if let Some(index) = sidebar.entry_at_y(sidebar_y) {
                            if let Some(action) = sidebar.click_entry(index) {
                                match action {
                                    SidebarAction::OpenMarkdown(path) => {
                                        self.process_action(InputAction::OpenMarkdown { path });
                                    }
                                    SidebarAction::OpenCanvas(path) => {
                                        let canvas_id = path
                                            .file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "unknown".to_string());
                                        self.create_canvas_with_id(&canvas_id);
                                    }
                                    SidebarAction::CreateCanvas(canvas_id, _path) => {
                                        self.create_canvas_with_id(&canvas_id);
                                    }
                                }
                            }
                        } else {
                            // Check if clicked on "New Canvas" button area
                            let header_offset = 16.0 + 15.6 + 8.0;
                            let entries_end = header_offset
                                + (sidebar.entries.len() as f32 * crate::sidebar::ENTRY_HEIGHT_PX)
                                + 8.0
                                - sidebar.scroll_offset;
                            if sidebar_y >= entries_end
                                && sidebar_y <= entries_end + crate::sidebar::ENTRY_HEIGHT_PX
                            {
                                self.process_action(InputAction::SidebarNewCanvas);
                            }
                        }
                    }
                } else if sidebar_visible
                    && lx < SIDEBAR_WIDTH
                    && ly > TOP_CHROME_HEIGHT
                    && state == ElementState::Pressed
                    && button == MouseButton::Right
                {
                    let sidebar_y = ly - TOP_CHROME_HEIGHT;
                    if let Some(sidebar) = &mut self.sidebar {
                        if let Some(index) = sidebar.entry_at_y(sidebar_y) {
                            sidebar.selected = Some(index);
                            let entry = &sidebar.entries[index];
                            let is_dir = entry.is_dir;
                            self.context_menu_target = Some(entry.path.clone());
                            #[cfg(target_os = "macos")]
                            if let Some(window) = &self.window {
                                crate::platform::context_menu::show_sidebar_context_menu(
                                    window,
                                    lx,
                                    ly,
                                    is_dir,
                                );
                            }
                        }
                    }
                } else if let Some(grid) = &self.grid {
                    let panels = &self.panels;
                    let panel_types = |pid: PanelId| -> Option<PanelType> {
                        panels.iter().find(|p| p.id == pid).map(|p| p.panel_type)
                    };
                    let actions = match state {
                        ElementState::Pressed => self.mouse_state.on_mouse_press(
                            button,
                            &self.dividers,
                            grid,
                            TOP_CHROME_HEIGHT,
                            &panel_types,
                            &self.modifiers,
                        ),
                        ElementState::Released => self.mouse_state.on_mouse_release(
                            button,
                            grid,
                            TOP_CHROME_HEIGHT,
                        ),
                    };
                    let actions: Vec<_> = actions;
                    for action in actions {
                        self.process_action(action);
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let lx = self.mouse_state.cursor_x as f32;

                // If mouse is over sidebar, scroll sidebar instead of panels
                let sidebar_visible = self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false);
                if sidebar_visible && lx < SIDEBAR_WIDTH {
                    let pixel_delta = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 21.0,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    if let (Some(sidebar), Some(window)) = (&mut self.sidebar, &self.window) {
                        let size = window.inner_size();
                        let viewport_h = size.height as f32 / self.scale_factor - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
                        sidebar.scroll(-pixel_delta, viewport_h);
                    }
                } else {
                    // Negate for natural scrolling convention: on macOS with natural
                    // scrolling, positive y means "scroll down" (content moves up),
                    // but positive delta to TerminalScroll means "scroll up/back".
                    let lines = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => {
                            self.scroll_pixel_accumulator = 0.0;
                            -(y * 3.0) as i32
                        }
                        winit::event::MouseScrollDelta::PixelDelta(pos) => {
                            self.scroll_pixel_accumulator += -pos.y;
                            let line_height = 20.0;
                            let accumulated_lines = (self.scroll_pixel_accumulator / line_height) as i32;
                            if accumulated_lines != 0 {
                                self.scroll_pixel_accumulator -= accumulated_lines as f64 * line_height;
                            }
                            accumulated_lines
                        }
                    };
                    if lines != 0 {
                        if let Some(grid) = &self.grid {
                            let sidebar_off = self.sidebar_offset();
                            let panels = &self.panels;
                            let panel_types = |pid: PanelId| -> Option<PanelType> {
                                panels.iter().find(|p| p.id == pid).map(|p| p.panel_type)
                            };
                            let actions = self.mouse_state.on_mouse_wheel(
                                lines as f32,
                                grid,
                                TOP_CHROME_HEIGHT,
                                sidebar_off,
                                &panel_types,
                            );
                            for action in actions {
                                self.process_action(action);
                            }
                        }
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // Intercept keys when in picker mode (D-09)
                if self.app_state == AppState::Picker && event.state == ElementState::Pressed {
                    use winit::keyboard::{Key, NamedKey};
                    let picker_action = match &event.logical_key {
                        Key::Named(NamedKey::ArrowDown) => {
                            if let Some(picker) = &mut self.picker_state {
                                picker.select_next();
                            }
                            None
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if let Some(picker) = &mut self.picker_state {
                                picker.select_prev();
                            }
                            None
                        }
                        Key::Named(NamedKey::Enter) => {
                            self.picker_state.as_ref().map(|p| p.handle_key_enter())
                        }
                        Key::Named(NamedKey::Escape) => {
                            self.picker_state.as_ref().map(|p| p.handle_key_escape())
                        }
                        Key::Character(c) if self.modifiers.super_key() && c.as_str() == "q" => {
                            event_loop.exit();
                            None
                        }
                        _ => None,
                    };
                    if let Some(action) = picker_action {
                        match action {
                            PickerAction::OpenProject(path) => {
                                self.open_project(path);
                            }
                            PickerAction::OpenFolderDialog => {
                                // Deferred: folder dialog requires platform-specific code
                                info!("Open folder dialog requested (deferred)");
                            }
                            PickerAction::LocateProject(_idx) => {
                                info!("Locate project requested (deferred)");
                            }
                            PickerAction::None => {}
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }

                // Intercept keys when settings overlay is visible
                if self.settings.visible && event.state == ElementState::Pressed {
                    use winit::keyboard::{Key, NamedKey};

                    // If recording mode is active, route key to recording state machine
                    if self.settings.recording.is_recording() {
                        if let Some(combo) = crate::shortcuts::chord::key_combo_from_event(&event, &self.modifiers) {
                            let result = self.settings.feed_recording_key(combo, &mut self.shortcut_registry);
                            if let Some(ref r) = result {
                                match r {
                                    crate::settings::SettingsShortcutResult::Bound { .. }
                                    | crate::settings::SettingsShortcutResult::Cleared => {
                                        self.save_shortcut_overrides();
                                        // Mirror settings conflict toast to shared toast system
                                        if let Some(toast) = self.settings.toasts.last() {
                                            self.toast_manager.add(
                                                crate::toast::ToastType::Conflict,
                                                toast.message.clone(),
                                                None,
                                                None,
                                                None,
                                                Some("Undo".to_string()),
                                                crate::toast::INFO_TOAST_DURATION,
                                            );
                                        }
                                    }
                                    crate::settings::SettingsShortcutResult::Cancelled => {}
                                }
                            }
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }

                    // Normal settings key handling (not recording)
                    match &event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.process_action(InputAction::CloseSettings);
                        }
                        Key::Character(c) if self.modifiers.super_key() && c.as_str() == "," => {
                            // Cmd+, toggles settings closed when already open
                            self.process_action(InputAction::CloseSettings);
                        }
                        _ => {}
                    }
                    return;
                }

                // Intercept keys when the init prompt is showing
                if self.init_prompt == InitPrompt::Showing && event.state == ElementState::Pressed {
                    use winit::keyboard::{Key, NamedKey};
                    let accepted = matches!(&event.logical_key, Key::Named(NamedKey::Enter))
                        || matches!(&event.logical_key, Key::Character(c) if c.as_str() == "y" || c.as_str() == "Y");
                    let dismissed = matches!(&event.logical_key, Key::Named(NamedKey::Escape))
                        || matches!(&event.logical_key, Key::Character(c) if c.as_str() == "n" || c.as_str() == "N");
                    if accepted {
                        self.process_action(InputAction::InitPromptAccept);
                    } else if dismissed {
                        self.process_action(InputAction::InitPromptDismiss);
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }

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
                let history_search_open = self
                    .focused_panel
                    .and_then(|pid| {
                        self.terminal_manager
                            .as_ref()?
                            .get(&pid)
                            .map(|ts| ts.autocomplete.history_search_is_open())
                    })
                    .unwrap_or(false);
                let has_ghost_text = self
                    .focused_panel
                    .and_then(|pid| {
                        self.terminal_manager
                            .as_ref()?
                            .get(&pid)
                            .map(|ts| ts.autocomplete.ghost_text().is_some())
                    })
                    .unwrap_or(false);
                let term_mode = self
                    .focused_panel
                    .and_then(|pid| self.terminal_manager.as_ref()?.get(&pid))
                    .map(|ts| *ts.term.lock().mode())
                    .unwrap_or(alacritty_terminal::term::TermMode::empty());
                let actions = keyboard::handle_key_event(
                    &event,
                    &self.modifiers,
                    self.focused_panel,
                    panel_type,
                    search_open,
                    history_search_open,
                    has_ghost_text,
                    term_mode,
                    &self.shortcut_registry,
                    &mut self.chord_state,
                );
                for action in actions {
                    if matches!(action, InputAction::Quit) {
                        // Save config on quit (same logic as CloseRequested)
                        if let (Some(grid), Some(project_dir)) =
                            (&self.grid, &self.project_dir)
                        {
                            let config =
                                crate::config::ProjectConfig::from_current_state(
                                    grid,
                                    &self.panels,
                                    self.terminal_manager.as_ref(),
                                    project_dir,
                                    Some(&self.theme_registry.active().name),
                                );
                            crate::config::save_project_config(
                                project_dir,
                                &config,
                            );
                            info!("Saved project config on quit");
                        }
                        info!("Quit requested via shortcut -- exiting");
                        event_loop.exit();
                        return;
                    }
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
                // Picker mode rendering: simplified path (no grid, no terminals)
                if self.app_state == AppState::Picker {
                    if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
                        let size = window.inner_size();
                        let s = self.scale_factor;
                        let logical_w = size.width as f32 / s;
                        let logical_h = size.height as f32 / s;
                        let physical_w = size.width as f32;
                        let physical_h = size.height as f32;

                        let mut quads = Vec::new();
                        let mut labels = Vec::new();

                        if let Some(picker) = &self.picker_state {
                            quads = crate::picker::renderer::build_quads(
                                picker, logical_w, logical_h, &self.theme,
                            );
                            labels = crate::picker::renderer::build_labels(
                                picker, logical_w, logical_h, &self.theme,
                            );
                        }

                        // Scale to physical
                        let phys_quads: Vec<QuadInstance> = quads
                            .into_iter()
                            .map(|mut q| {
                                q.position[0] *= s;
                                q.position[1] *= s;
                                q.size[0] *= s;
                                q.size[1] *= s;
                                q.corner_radius *= s;
                                q
                            })
                            .collect();
                        let phys_labels: Vec<TextLabel> = labels
                            .into_iter()
                            .map(|mut l| {
                                l.x *= s;
                                l.y *= s;
                                l.width *= s;
                                l.height *= s;
                                l
                            })
                            .collect();

                        match renderer.render(
                            self.theme.background,
                            &phys_quads,
                            &phys_labels,
                            physical_w,
                            physical_h,
                            s,
                            vec![],
                        ) {
                            crate::renderer::RenderResult::Ok => {}
                            crate::renderer::RenderResult::SkipFrame => {}
                            crate::renderer::RenderResult::SurfaceLost => {
                                warn!("Surface lost in picker mode");
                            }
                        }
                    }
                    // Skip workspace rendering
                } else
                // Workspace mode rendering (existing code)
                if let Some(window) = &self.window {
                    let _frame_span = tracing::trace_span!("frame").entered();
                    let frame_start = Instant::now();
                    let size = window.inner_size();
                    let s = self.scale_factor;
                    let logical_w = size.width as f32 / s;
                    let logical_h = size.height as f32 / s;
                    let physical_w = size.width as f32;
                    let physical_h = size.height as f32;

                    // Pre-compute terminal snapshots once per frame (WR-01: avoid double snapshot).
                    let _snap_span = tracing::trace_span!("snapshot_terminals").entered();
                    let mut snapshots: HashMap<PanelId, TerminalSnapshot> = HashMap::new();
                    if let Some(tm) = &self.terminal_manager {
                        for (&panel_id, ts) in tm.terminals().iter() {
                            let is_terminal = self
                                .panels
                                .iter()
                                .any(|p| p.id == panel_id && p.panel_type == PanelType::Terminal);
                            if is_terminal && !ts.exited {
                                snapshots.insert(panel_id, TerminalRenderer::snapshot(&ts.term));
                            }
                        }
                    }
                    drop(_snap_span);

                    // Pre-compute context pill data (CWD + git info) while we have &mut access
                    let mut pill_data: HashMap<PanelId, (String, Option<(String, Option<(usize, usize, usize)>)>)> = HashMap::new();
                    if let Some(tm) = &mut self.terminal_manager {
                        for (&panel_id, ts) in tm.terminals_mut().iter_mut() {
                            if !ts.exited {
                                let cwd = ts.display_cwd();
                                let git = ts.git_info();
                                pill_data.insert(panel_id, (cwd, git));
                            }
                        }
                    }

                    let cell_count: usize = snapshots.values().map(|s| s.cols * s.rows.len()).sum();

                    // Update stats bar slots before rendering
                    self.stats_bar.update_panel_count(self.panels.len());
                    self.stats_bar.update_uptime();

                    // Refresh bottom bar git info cache (5s interval)
                    if let Some(bottom_bar) = &mut self.bottom_bar {
                        bottom_bar.refresh();
                    }

                    // Build frame data in logical coordinates
                    let (logical_quads, pill_labels) = self.build_quads(logical_w, logical_h, &snapshots, &pill_data);
                    let mut logical_labels = self.build_labels(logical_w, logical_h, &snapshots);
                    logical_labels.extend(pill_labels);

                    // Scale quads from logical to physical at the GPU render boundary
                    let quads: Vec<QuadInstance> = logical_quads
                        .into_iter()
                        .map(|mut q| {
                            q.position[0] *= s;
                            q.position[1] *= s;
                            q.size[0] *= s;
                            q.size[1] *= s;
                            q.corner_radius *= s;
                            q
                        })
                        .collect();

                    // Scale label positions/sizes to physical (font_size stays logical;
                    // glyphon's TextArea.scale handles DPI scaling for text)
                    let labels: Vec<TextLabel> = logical_labels
                        .into_iter()
                        .map(|mut l| {
                            l.x *= s;
                            l.y *= s;
                            l.width *= s;
                            l.height *= s;
                            l
                        })
                        .collect();

                    let sidebar_off = self.sidebar_offset();

                    // Phase 1: Update terminal and markdown buffer caches
                    if let Some(renderer) = &mut self.renderer {
                        let font_system = renderer.text_engine_mut().font_system_mut();

                        // Terminal buffer cache update (only reshapes changed rows)
                        if let Some(tm) = &self.terminal_manager {
                            if let Some(grid) = &self.grid {
                                let _prep_span = tracing::trace_span!("prepare_terminal_text").entered();
                                for &(node, panel_id) in grid.panel_nodes() {
                                    if let Some(ts) = tm.get(&panel_id) {
                                        if let Some(snapshot) = snapshots.get(&panel_id) {
                                            if !ts.exited {
                                                let (px, py, pw, ph) =
                                                    grid.get_panel_rect(node);
                                                let content_y =
                                                    py + TOP_CHROME_HEIGHT + PANEL_TITLE_HEIGHT;
                                                let content_h = ph - PANEL_TITLE_HEIGHT;

                                                // Bottom-align: push content down when it doesn't fill the viewport
                                                let bottom_offset = if ts.scroll_offset == 0 {
                                                    snapshot.bottom_align_offset(content_h, ts.cell_height, TerminalRenderer::PILL_RESERVE)
                                                } else {
                                                    0.0
                                                };

                                                self.terminal_renderer.update_cache(
                                                    panel_id,
                                                    font_system,
                                                    snapshot,
                                                    px + sidebar_off + PANEL_CONTENT_PADDING,
                                                    content_y + bottom_offset,
                                                    pw - PANEL_CONTENT_PADDING * 2.0,
                                                    content_h,
                                                    ts.font_size,
                                                    ts.cell_width,
                                                    ts.cell_height,
                                                );
                                            }
                                        }
                                    }
                                }
                                drop(_prep_span);
                            }
                        }

                        // Markdown buffer cache update
                        if let (Some(mm), Some(grid)) = (&mut self.markdown_manager, &self.grid) {
                            let _md_span = tracing::trace_span!("prepare_markdown_text").entered();
                            for &(node, panel_id) in grid.panel_nodes() {
                                let is_markdown = self
                                    .panels
                                    .iter()
                                    .any(|p| p.id == panel_id && p.panel_type == PanelType::Markdown);
                                if is_markdown {
                                    if let Some(state) = mm.get_mut(&panel_id) {
                                        let (px, py, pw, ph) = grid.get_panel_rect(node);
                                        let content_y = py + TOP_CHROME_HEIGHT + PANEL_TITLE_HEIGHT;
                                        let content_h = ph - PANEL_TITLE_HEIGHT;

                                        let dirty = state.dirty;
                                        self.markdown_renderer.update_cache(
                                            panel_id,
                                            font_system,
                                            &state.blocks,
                                            &state.block_heights,
                                            state.scroll_offset,
                                            px + sidebar_off,
                                            content_y,
                                            pw,
                                            content_h,
                                            dirty,
                                        );
                                        state.dirty = false;
                                    }
                                }
                            }
                            drop(_md_span);
                        }

                        // Sidebar buffer preparation
                        if let Some(sidebar) = &self.sidebar {
                            if sidebar.visible {
                                let sidebar_viewport_y = TOP_CHROME_HEIGHT;
                                let sidebar_viewport_h = logical_h - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT;
                                let (bufs, metas) = SidebarRenderer::prepare_buffers(
                                    font_system,
                                    sidebar,
                                    sidebar_viewport_y,
                                    sidebar_viewport_h,
                                    &self.theme,
                                );
                                self.sidebar_buffers = bufs;
                                self.sidebar_metas = metas;
                            } else {
                                self.sidebar_buffers.clear();
                                self.sidebar_metas.clear();
                            }
                        }
                    }

                    // Phase 2: Collect cached TextAreas and render
                    let mut terminal_text_areas = self.terminal_renderer.collect_text_areas(s);
                    // Append markdown text areas to the same vec
                    terminal_text_areas.extend(self.markdown_renderer.collect_text_areas(s));

                    // Append sidebar text areas
                    {
                        use glyphon::{TextArea, TextBounds};
                        let default_color = glyphon::Color::rgba(248, 248, 242, 255);
                        for (buf, meta) in self.sidebar_buffers.iter().zip(self.sidebar_metas.iter()) {
                            terminal_text_areas.push(TextArea {
                                buffer: buf,
                                left: meta.left * s,
                                top: meta.top * s,
                                scale: s,
                                bounds: TextBounds {
                                    left: 0,
                                    top: (TOP_CHROME_HEIGHT * s) as i32,
                                    right: (SIDEBAR_WIDTH * s) as i32,
                                    bottom: ((logical_h - BOTTOM_BAR_HEIGHT) * s) as i32,
                                },
                                default_color,
                                custom_glyphs: &[],
                            });
                        }
                    }

                    if let Some(renderer) = &mut self.renderer {
                        match renderer.render(
                            self.theme.background,
                            &quads,
                            &labels,
                            physical_w,
                            physical_h,
                            s,
                            terminal_text_areas,
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

                    self.frame_stats.record(frame_start.elapsed(), quads.len(), cell_count);
                    if self.frame_stats.should_log() {
                        self.frame_stats.log_and_reset();
                    }
                }
            }

            _ => {}
        }

        // Drain pending actions (from IPC shortcut forwarding, focus cycling, etc.)
        while let Some(action) = self.pending_actions.pop() {
            self.process_action(action);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        const ACTIVE_INTERVAL: Duration = Duration::from_millis(16);
        const IDLE_INTERVAL: Duration = Duration::from_millis(500);

        let mut needs_render = false;
        if let Some(tm) = &mut self.terminal_manager {
            if tm.drain_all_events() {
                needs_render = true;
            }
            if tm.update_all_cursor_blinks() {
                needs_render = true;
            }
            for ts in tm.terminals_mut().values_mut() {
                ts.clear_expired_flash();
            }
        }

        // Reset stale chord state (D-15: 500ms timeout between chord keys)
        self.chord_state.check_timeout();

        // Settings recording timeout and toast expiry (D-14, D-16)
        if self.settings.visible {
            if let Some(ref result) = self.settings.check_recording_timeout(&mut self.shortcut_registry) {
                match result {
                    crate::settings::SettingsShortcutResult::Bound { .. }
                    | crate::settings::SettingsShortcutResult::Cleared => {
                        self.save_shortcut_overrides();
                    }
                    crate::settings::SettingsShortcutResult::Cancelled => {}
                }
                needs_render = true;
            }
            let prev_toast_count = self.settings.toasts.len();
            self.settings.tick_toasts();
            if self.settings.toasts.len() != prev_toast_count {
                needs_render = true;
            }
            // Recording mode needs periodic redraws for visual pulsing
            if self.settings.recording.is_recording() {
                needs_render = true;
            }
        }

        // Unified toast manager tick (expire old toasts)
        {
            let prev_count = self.toast_manager.count();
            self.toast_manager.tick();
            if self.toast_manager.count() != prev_count {
                needs_render = true;
            }
        }

        // Tooltip redraw (for delayed appearance)
        if let Some(ref tooltip) = self.tooltip_state {
            if tooltip.hover_start.elapsed() >= Duration::from_millis(300)
                && tooltip.hover_start.elapsed() < Duration::from_millis(350)
            {
                needs_render = true;
            }
        }

        if needs_render {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        // Auto-save config if dirty and debounce elapsed (D-07, D-08)
        if self.auto_save.should_save() {
            if let (Some(grid), Some(project_dir)) = (&self.grid, &self.project_dir) {
                let config = crate::config::ProjectConfig::from_current_state(
                    grid,
                    &self.panels,
                    self.terminal_manager.as_ref(),
                    project_dir,
                    Some(&self.theme_registry.active().name),
                );
                crate::config::save_project_config(project_dir, &config);
                self.auto_save.mark_saved();
            }
        }

        let has_terminals = self.terminal_manager.as_ref()
            .map_or(false, |tm| !tm.terminals().is_empty());
        if has_terminals {
            let interval = if needs_render { ACTIVE_INTERVAL } else { IDLE_INTERVAL };
            event_loop.set_control_flow(
                winit::event_loop::ControlFlow::WaitUntil(Instant::now() + interval)
            );
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }
}
