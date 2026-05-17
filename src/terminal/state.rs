//! Terminal state wrapping alacritty_terminal's Term in an Arc<FairMutex>.
//!
//! Manages the PTY lifecycle, event draining, and cursor blink state.

use std::borrow::Cow;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config as TermConfig, Term, TermDamage, TermMode};
use alacritty_terminal::tty;
use tracing::{debug, warn};

use super::event_listener::MycoEventListener;

/// Dimensions struct implementing alacritty_terminal's Dimensions trait.
///
/// Used for Term creation and resize operations.
#[derive(Debug, Clone, Copy)]
pub struct TermDimensions {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

/// State for a single terminal instance.
///
/// Wraps the alacritty_terminal Term in an Arc<FairMutex> for thread-safe access,
/// and manages the PTY event loop, event channel, and cursor blink state.
pub struct TerminalState {
    /// Thread-safe terminal grid state.
    pub term: Arc<FairMutex<Term<MycoEventListener>>>,
    /// Channel to write data to the PTY via the background event loop.
    pub event_loop_sender: EventLoopSender,
    /// Receiver for events from the background thread (Wakeup, Exit, etc.)
    event_rx: mpsc::Receiver<alacritty_terminal::event::Event>,
    /// Handle to the background event loop thread.
    _event_loop_handle: JoinHandle<(EventLoop<tty::Pty, MycoEventListener>, alacritty_terminal::event_loop::State)>,
    /// Whether the shell process has exited.
    pub exited: bool,
    /// Exit code if the process has exited.
    pub exit_code: Option<i32>,
    /// Cell width in pixels (computed from font metrics).
    pub cell_width: f32,
    /// Cell height in pixels (computed from font metrics).
    pub cell_height: f32,
    /// Current font size in points.
    pub font_size: f32,
    /// Whether the cursor is currently visible in the blink cycle.
    pub cursor_blink_visible: bool,
    /// Timestamp of the last cursor blink toggle.
    cursor_blink_last_toggle: Instant,
    /// Whether cursor blinking is enabled (programs can toggle via escape sequences).
    pub cursor_blink_enabled: bool,

    // --- Scrollback state (D-10, D-11, D-12) ---
    /// Current display offset (how far scrolled back; 0 = at bottom).
    pub scroll_offset: usize,
    /// Whether new output arrived while the terminal is scrolled up (for D-10 indicator).
    pub has_new_output_while_scrolled: bool,

    // --- Copy flash state (D-15) ---
    /// Start time of the copy flash animation (None = no flash active).
    pub copy_flash_start: Option<Instant>,

    // --- Search state (D-09) ---
    /// Search overlay state machine.
    pub search: crate::terminal::search::SearchState,

    // --- Autocomplete state ---
    /// Ghost text autocomplete and Ctrl+R history search.
    pub autocomplete: crate::terminal::autocomplete::AutocompleteState,

    /// Whether the terminal grid has visual damage since last render.
    content_dirty: bool,

    #[allow(dead_code)]
    event_listener: MycoEventListener,

    /// Initial working directory (project dir).
    pub working_dir: std::path::PathBuf,
    /// Current working directory as reported by shell title (OSC 2).
    pub current_title: Option<String>,

    /// Child process PID (captured at creation time for resource monitoring, T-06-02).
    pub child_pid: Option<u32>,

    /// Cached git info (branch, optional stats). Refreshed periodically.
    cached_git_info: Option<(String, Option<(usize, usize, usize)>)>,
    /// Last time git info was refreshed.
    git_info_last_refresh: Instant,
}

impl TerminalState {
    /// Create a new terminal with the given dimensions and working directory.
    ///
    /// Spawns the user's $SHELL (fallback /bin/zsh on macOS) in the given directory.
    /// The background event loop thread handles all PTY I/O.
    pub fn new(
        cols: usize,
        rows: usize,
        working_dir: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Per D-12: 50K line scrollback
        let config = TermConfig {
            scrolling_history: 50_000,
            ..TermConfig::default()
        };

        // Create event channel for background -> main thread communication
        let (event_tx, event_rx) = mpsc::channel();
        let event_listener = MycoEventListener::new(event_tx);

        // Create terminal grid state
        let dims = TermDimensions { cols, rows };
        let term = Term::new(config, &dims, event_listener.clone());
        let term = Arc::new(FairMutex::new(term));

        // Default font metrics (will be updated after font system is available)
        let font_size = 14.0_f32;
        let cell_width = font_size * 0.6;
        let cell_height = font_size * 1.3;

        // Create PTY with user's shell
        // Per D-01: detect $SHELL, fallback to /bin/zsh
        // Per D-02: working directory is the project folder
        // Per D-04: inherits full parent environment automatically
        let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let pty_config = tty::Options {
            shell: Some(tty::Shell::new(shell_path, vec![])),
            working_directory: Some(working_dir.to_path_buf()),
            ..Default::default()
        };

        let window_size = WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width: cell_width.round() as u16,
            cell_height: cell_height.round() as u16,
        };

        let pty = tty::new(&pty_config, window_size, 0)?;

        // Capture child PID before pty is consumed by EventLoop (T-06-02).
        // Only signal PIDs we ourselves spawned.
        let child_pid = Some(pty.child().id());

        // Create and spawn the background event loop
        let listener_handle = event_listener.clone();
        let event_loop = EventLoop::new(
            term.clone(),
            event_listener,
            pty,
            false, // drain_on_exit
            false, // ref_test
        )?;
        let event_loop_sender = event_loop.channel();
        let event_loop_handle = event_loop.spawn();

        debug!("Terminal created: {}x{} in {:?}", cols, rows, working_dir);

        Ok(Self {
            term,
            event_loop_sender,
            event_rx,
            _event_loop_handle: event_loop_handle,
            exited: false,
            exit_code: None,
            cell_width,
            cell_height,
            font_size,
            cursor_blink_visible: true,
            cursor_blink_last_toggle: Instant::now(),
            cursor_blink_enabled: true, // Per D-08: blink by default
            scroll_offset: 0,
            has_new_output_while_scrolled: false,
            copy_flash_start: None,
            search: crate::terminal::search::SearchState::new(),
            autocomplete: crate::terminal::autocomplete::AutocompleteState::new(),
            content_dirty: true,
            event_listener: listener_handle,
            working_dir: working_dir.to_path_buf(),
            current_title: None,
            child_pid,
            cached_git_info: None,
            git_info_last_refresh: Instant::now() - Duration::from_secs(60),
        })
    }

    /// Drain all pending events from the background thread.
    ///
    /// Called in the main thread's about_to_wait handler.
    /// Handles terminal events like exit, cursor blink changes, etc.
    pub fn drain_events(&mut self) -> bool {
        let mut had_wakeup = false;
        let mut had_meaningful_event = false;
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                alacritty_terminal::event::Event::Exit => {
                    debug!("Terminal: Exit event received");
                    self.exited = true;
                    had_meaningful_event = true;
                }
                alacritty_terminal::event::Event::ChildExit(status) => {
                    debug!("Terminal: ChildExit event received: {:?}", status);
                    self.exited = true;
                    self.exit_code = status.code();
                    had_meaningful_event = true;
                }
                alacritty_terminal::event::Event::CursorBlinkingChange => {
                    let term = self.term.lock();
                    self.cursor_blink_enabled = term.cursor_style().blinking;
                    debug!(
                        "Terminal: CursorBlinkingChange, enabled={}",
                        self.cursor_blink_enabled
                    );
                    had_meaningful_event = true;
                }
                alacritty_terminal::event::Event::Wakeup => {
                    had_wakeup = true;
                }
                alacritty_terminal::event::Event::Title(title) => {
                    debug!("Terminal: Title changed to {:?}", title);
                    self.current_title = Some(title);
                }
                other => {
                    debug!("Terminal: Unhandled event: {:?}", other);
                }
            }
        }

        if had_wakeup {
            let mut term = self.term.lock();
            let damaged = match term.damage() {
                TermDamage::Full => true,
                TermDamage::Partial(iter) => iter.count() > 0,
            };
            term.reset_damage();
            if damaged {
                self.content_dirty = true;
            }
        }

        let needs_render = std::mem::take(&mut self.content_dirty) || had_meaningful_event;
        if needs_render {
            self.on_new_output();
        }
        needs_render
    }

    /// Update cursor blink state based on elapsed time.
    ///
    /// Returns true if the visibility state changed (needs redraw).
    /// Toggles every 500ms when blinking is enabled.
    pub fn update_cursor_blink(&mut self) -> bool {
        if !self.cursor_blink_enabled {
            if !self.cursor_blink_visible {
                self.cursor_blink_visible = true;
                return true;
            }
            return false;
        }

        if self.cursor_blink_last_toggle.elapsed() >= Duration::from_millis(500) {
            self.cursor_blink_visible = !self.cursor_blink_visible;
            self.cursor_blink_last_toggle = Instant::now();
            return true;
        }

        false
    }

    /// Reset cursor blink to visible state (called when user types).
    pub fn reset_cursor_blink(&mut self) {
        self.cursor_blink_visible = true;
        self.cursor_blink_last_toggle = Instant::now();
    }

    /// Write bytes to the PTY via the event loop sender.
    pub fn write_to_pty(&self, bytes: &[u8]) {
        if let Err(e) = self
            .event_loop_sender
            .send(Msg::Input(Cow::Owned(bytes.to_vec())))
        {
            warn!("Failed to write to PTY: {}", e);
        }
    }

    // --- Scrollback methods (D-10, D-11) ---

    /// Scroll the terminal display by delta lines (positive = scroll up/back in history).
    ///
    /// Per D-11: if in ALT_SCREEN mode, send arrow key sequences to the app instead.
    pub fn scroll(&mut self, delta: i32) {
        let term = self.term.lock();
        let mode = *term.mode();
        drop(term);

        if mode.contains(TermMode::ALT_SCREEN) {
            // D-11: In alternate screen (vim, htop, less), send arrow keys to the app
            let key = if delta > 0 { b"\x1b[A" } else { b"\x1b[B" };
            for _ in 0..delta.unsigned_abs() {
                self.write_to_pty(key);
            }
        } else {
            let mut term = self.term.lock();
            term.scroll_display(Scroll::Delta(delta));
            self.scroll_offset = term.grid().display_offset();
            if self.scroll_offset == 0 {
                self.has_new_output_while_scrolled = false;
            }
        }
    }

    /// Jump to the latest output (scroll to bottom). Clears new output indicator.
    pub fn scroll_to_bottom(&mut self) {
        let mut term = self.term.lock();
        term.scroll_display(Scroll::Bottom);
        self.scroll_offset = 0;
        self.has_new_output_while_scrolled = false;
    }

    /// Called when new PTY output arrives. Updates the new-output indicator per D-10.
    fn on_new_output(&mut self) {
        let term = self.term.lock();
        let offset = term.grid().display_offset();
        drop(term);
        if offset > 0 {
            self.has_new_output_while_scrolled = true;
        }
        self.scroll_offset = offset;
    }

    // --- Copy flash methods (D-15) ---

    /// Trigger copy flash animation (brief highlight ~200ms fade).
    pub fn trigger_copy_flash(&mut self) {
        self.copy_flash_start = Some(Instant::now());
    }

    /// Check if copy flash is active and return opacity (0.0-1.0).
    /// Returns None if flash has expired (>200ms).
    pub fn copy_flash_opacity(&self) -> Option<f32> {
        if let Some(start) = self.copy_flash_start {
            let elapsed = start.elapsed().as_millis() as f32;
            if elapsed < 200.0 {
                Some(1.0 - (elapsed / 200.0))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Clear expired flash state.
    pub fn clear_expired_flash(&mut self) {
        if let Some(start) = self.copy_flash_start {
            if start.elapsed().as_millis() > 200 {
                self.copy_flash_start = None;
            }
        }
    }

    /// Get the effective CWD for display in the context pill.
    /// Parses from shell title (zsh sets title to "user@host: /path" or just "/path"),
    /// falls back to the initial working directory.
    pub fn effective_cwd(&self) -> std::path::PathBuf {
        if let Some(title) = &self.current_title {
            // zsh default: "user@host: /path" or just the path
            let path_str = if let Some(after_colon) = title.split(": ").nth(1) {
                after_colon.trim()
            } else if title.starts_with('/') {
                title.trim()
            } else {
                return self.working_dir.clone();
            };
            let path = std::path::PathBuf::from(path_str);
            if path.is_absolute() {
                return path;
            }
        }
        self.working_dir.clone()
    }

    /// Get a shortened display path for the CWD (replace $HOME with ~).
    pub fn display_cwd(&self) -> String {
        let cwd = self.effective_cwd();
        let cwd_str = cwd.to_string_lossy();
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();
            if let Some(rest) = cwd_str.strip_prefix(home_str.as_ref()) {
                return format!("~{rest}");
            }
        }
        cwd_str.into_owned()
    }

    /// Get git branch and status info for the effective CWD.
    /// Cached for 5 seconds to avoid hitting the filesystem on every frame.
    pub fn git_info(&mut self) -> Option<(String, Option<(usize, usize, usize)>)> {
        if self.git_info_last_refresh.elapsed() > Duration::from_secs(5) {
            self.git_info_last_refresh = Instant::now();
            self.cached_git_info = Self::fetch_git_info(&self.effective_cwd());
        }
        self.cached_git_info.clone()
    }

    fn fetch_git_info(cwd: &std::path::Path) -> Option<(String, Option<(usize, usize, usize)>)> {
        let repo = git2::Repository::discover(cwd).ok()?;
        let head = repo.head().ok()?;
        let branch = head.shorthand().unwrap_or("HEAD").to_string();

        let stats = repo.diff_index_to_workdir(None, None).ok().and_then(|diff| {
            let stats = diff.stats().ok()?;
            let changed = stats.files_changed();
            let ins = stats.insertions();
            let del = stats.deletions();
            if changed > 0 { Some((changed, ins, del)) } else { None }
        });

        Some((branch, stats))
    }
}
