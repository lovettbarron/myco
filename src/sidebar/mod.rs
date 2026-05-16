pub mod renderer;

use std::path::{Path, PathBuf};
use tracing::debug;

/// Width of the sidebar in logical pixels (per UI-SPEC).
pub const SIDEBAR_WIDTH: f32 = 240.0;

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
}

impl SidebarState {
    pub fn new(project_dir: PathBuf) -> Self {
        let mut expanded_dirs = std::collections::HashSet::new();
        // Auto-expand .myco directory
        expanded_dirs.insert(project_dir.join(".myco"));

        let mut state = Self {
            visible: true, // Visible by default, toggle with Cmd+B
            entries: Vec::new(),
            selected: None,
            hovered: None,
            scroll_offset: 0.0,
            project_dir,
            expanded_dirs,
        };
        state.refresh_file_tree();
        state
    }

    /// Toggle sidebar visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        debug!("Sidebar visibility: {}", self.visible);
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

            // Skip hidden files except .myco directory (which contains canvas files)
            if name.starts_with('.') && name != ".myco" {
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
                "tldr" => Some(SidebarAction::OpenCanvas(path)),
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
        let path = canvas_dir.join(format!("{}.tldr", canvas_id));
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

    /// Get the total visible height of sidebar content.
    pub fn content_height(&self) -> f32 {
        let header = 16.0 + 15.6 + 8.0; // top padding + FILES heading + gap
        let entries = self.entries.len() as f32 * ENTRY_HEIGHT;
        let footer = 8.0 + ENTRY_HEIGHT; // gap + "New Canvas" button
        header + entries + footer
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
