pub mod renderer;
pub mod search;

use std::path::{Path, PathBuf};
use tracing::debug;

use crate::config::registry::ProjectEntry;

/// Default width of the sidebar in logical pixels.
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 240.0;

/// Minimum sidebar width in logical pixels.
pub const SIDEBAR_MIN_WIDTH: f32 = 160.0;

/// Hit zone width for the sidebar resize edge (pixels from the right edge).
pub const SIDEBAR_EDGE_HIT_ZONE: f32 = 4.0;

/// Height of each file entry row (matches PANEL_TITLE_HEIGHT for visual consistency).
const ENTRY_HEIGHT: f32 = 28.0;

/// A single entry in the file tree.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// File or directory name (display name only, not full path).
    pub name: String,
    /// Full path to the file/directory.
    pub path: PathBuf,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Nesting depth (0 = root level).
    pub depth: u8,
    /// Whether this directory is expanded (only meaningful for dirs).
    pub expanded: bool,
}

/// File sidebar state.
pub struct SidebarState {
    /// Whether the sidebar is currently visible.
    pub visible: bool,
    /// Current sidebar width in logical pixels.
    pub width: f32,
    /// Flat list of visible file entries (expanded dirs show children).
    pub entries: Vec<FileEntry>,
    /// Currently selected entry index (None if nothing selected).
    pub selected: Option<usize>,
    /// Currently hovered entry index.
    pub hovered: Option<usize>,
    /// Scroll offset (for long file trees).
    pub scroll_offset: f32,
    /// Project root directory.
    project_dir: PathBuf,
    /// Tracks which directories are expanded (by path).
    expanded_dirs: std::collections::HashSet<PathBuf>,
    /// Registered projects for the project switcher section.
    pub projects: Vec<ProjectEntry>,
    /// Whether to show the .git directory in the file tree.
    pub show_git_directory: bool,
    /// Project-wide file search state.
    pub search: search::SearchState,
}

impl SidebarState {
    pub fn new(project_dir: PathBuf, show_git_directory: bool) -> Self {
        let mut expanded_dirs = std::collections::HashSet::new();
        // Auto-expand .myco directory
        expanded_dirs.insert(project_dir.join(".myco"));

        let mut state = Self {
            visible: true, // Visible by default, toggle with Cmd+B
            width: SIDEBAR_DEFAULT_WIDTH,
            entries: Vec::new(),
            selected: None,
            hovered: None,
            scroll_offset: 0.0,
            project_dir,
            expanded_dirs,
            projects: Vec::new(),
            show_git_directory,
            search: search::SearchState::new(),
        };
        state.refresh_file_tree();
        state
    }

    /// Toggle sidebar visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        debug!("Sidebar visibility: {}", self.visible);
    }

    /// Resize the sidebar by a pixel delta, clamping to min and max.
    /// `window_width` is the total window width for computing the 40% max.
    pub fn resize(&mut self, delta: f32, window_width: f32) {
        let max_width = window_width * 0.4;
        self.width = (self.width + delta).clamp(SIDEBAR_MIN_WIDTH, max_width);
    }

    /// Get the project directory.
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// Rebuild the file tree from the project directory.
    pub fn refresh_file_tree(&mut self) {
        self.entries.clear();
        let root = self.project_dir.clone();
        self.build_tree(&root, 0);
    }

    fn build_tree(&mut self, dir: &Path, depth: u8) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };

        let mut dir_entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();

        // Sort: directories first, then alphabetical
        dir_entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in dir_entries {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            // Hide .git unless show_git_directory is enabled
            if name == ".git" && !self.show_git_directory {
                continue;
            }

            // Hide .DS_Store (macOS metadata noise)
            if name == ".DS_Store" {
                continue;
            }

            // T-03-11: Skip symlinks that resolve outside project_dir
            if is_dir {
                if let Ok(canonical) = path.canonicalize() {
                    if let Ok(project_canonical) = self.project_dir.canonicalize() {
                        if !canonical.starts_with(&project_canonical) {
                            continue;
                        }
                    }
                }
            }

            let expanded = is_dir && self.expanded_dirs.contains(&path);

            self.entries.push(FileEntry {
                name: name.clone(),
                path: path.clone(),
                is_dir,
                depth,
                expanded,
            });

            // Recurse into expanded directories
            if is_dir && expanded {
                self.build_tree(&path, depth + 1);
            }
        }
    }

    /// Handle click on an entry at the given index.
    pub fn click_entry(&mut self, index: usize) -> Option<SidebarAction> {
        if index >= self.entries.len() {
            return None;
        }

        self.selected = Some(index);
        let entry = &self.entries[index];

        if entry.is_dir {
            // Toggle directory expansion
            let path = entry.path.clone();
            if self.expanded_dirs.contains(&path) {
                self.expanded_dirs.remove(&path);
            } else {
                self.expanded_dirs.insert(path.clone());
            }
            // Rebuild tree to reflect expansion change
            self.refresh_file_tree();
            // Restore selection to the toggled dir
            self.selected = self.entries.iter().position(|e| e.path == path);
            None
        } else {
            let path = entry.path.clone();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext {
                "md" | "markdown" => Some(SidebarAction::OpenMarkdown(path)),
                "excalidraw" => Some(SidebarAction::OpenCanvas(path)),
                _ => None, // Other file types not handled in Phase 3
            }
        }
    }

    /// Create a new canvas file with timestamp name.
    pub fn new_canvas(&self) -> Option<SidebarAction> {
        let canvas_dir = self.project_dir.join(".myco").join("canvas");
        let _ = std::fs::create_dir_all(&canvas_dir);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let canvas_id = format!("canvas-{}", timestamp);
        let path = canvas_dir.join(format!("{}.excalidraw", canvas_id));
        Some(SidebarAction::CreateCanvas(canvas_id, path))
    }

    /// Scroll the sidebar by delta pixels.
    pub fn scroll(&mut self, delta: f32, viewport_height: f32) {
        let total_height = self.entries.len() as f32 * ENTRY_HEIGHT;
        self.scroll_offset = (self.scroll_offset + delta)
            .max(0.0)
            .min((total_height - viewport_height).max(0.0));
    }

    /// Get entry index at a given y position within the sidebar viewport.
    pub fn entry_at_y(&self, y: f32) -> Option<usize> {
        let adjusted_y = y + self.scroll_offset;
        let header_offset = 16.0 + 15.6 + 8.0; // top padding + "FILES" heading + gap
        if adjusted_y < header_offset {
            return None;
        }
        let index = ((adjusted_y - header_offset) / ENTRY_HEIGHT) as usize;
        if index < self.entries.len() {
            Some(index)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn content_height(&self) -> f32 {
        let header = 16.0 + 15.6 + 8.0; // top padding + FILES heading + gap
        let entries = self.entries.len() as f32 * ENTRY_HEIGHT;
        let footer = 8.0 + ENTRY_HEIGHT; // gap + "New Canvas" button
        header + entries + footer
    }

    /// Set the list of registered projects for the sidebar project switcher.
    pub fn set_projects(&mut self, projects: Vec<ProjectEntry>) {
        self.projects = projects;
    }

    /// Whether the sidebar search mode is currently active.
    pub fn search_active(&self) -> bool {
        self.search.active
    }

    /// Handle a click in search mode at the given y position within the sidebar viewport.
    /// Returns a SidebarAction if a match line for an openable file was clicked.
    pub fn search_click_at_y(&mut self, y: f32) -> Option<SidebarAction> {
        if !self.search.active {
            return None;
        }
        let header_offset = 16.0 + 15.6 + 8.0; // SEARCH header
        let input_offset = header_offset + ENTRY_HEIGHT; // input box
        let count_offset = input_offset + ENTRY_HEIGHT; // results count
        let entries_start = count_offset;

        let adjusted_y = y + self.search.scroll_offset;
        if adjusted_y < entries_start {
            return None;
        }

        let entry_idx = ((adjusted_y - entries_start) / ENTRY_HEIGHT) as usize;
        let flat = self.search.flat_entries();
        if entry_idx >= flat.len() {
            return None;
        }

        match flat[entry_idx] {
            search::SearchFlatEntry::FileHeader(file_idx) => {
                self.search.toggle_file_expansion(file_idx);
                None
            }
            search::SearchFlatEntry::MatchLine(file_idx, _match_idx) => {
                let path = self.search.results[file_idx].path.clone();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                match ext {
                    "md" | "markdown" => Some(SidebarAction::OpenMarkdown(path)),
                    "excalidraw" => Some(SidebarAction::OpenCanvas(path)),
                    _ => None,
                }
            }
        }
    }
}

/// Actions produced by sidebar interactions.
#[derive(Debug)]
pub enum SidebarAction {
    OpenMarkdown(PathBuf),
    OpenCanvas(PathBuf),
    CreateCanvas(String, PathBuf), // (canvas_id, path)
}

/// Entry height constant (exported for renderer).
pub const ENTRY_HEIGHT_PX: f32 = ENTRY_HEIGHT;
