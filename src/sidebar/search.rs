use std::fs;
use std::io::{BufRead, BufReader, Read as _};
use std::path::{Path, PathBuf};

/// A single match within a file.
pub struct SearchMatch {
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

/// All matches within a single file.
pub struct SearchFileResult {
    pub path: PathBuf,
    pub rel_path: String,
    pub file_name: String,
    pub matches: Vec<SearchMatch>,
    pub expanded: bool,
}

/// A flattened entry for rendering and hit-testing.
pub enum SearchFlatEntry {
    /// Index into SearchState::results
    FileHeader(usize),
    /// (file_idx, match_idx) into results[file_idx].matches[match_idx]
    MatchLine(usize, usize),
}

/// State for the project-wide sidebar search.
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchFileResult>,
    pub total_matches: usize,
    pub active: bool,
    pub selected: Option<usize>,
    pub scroll_offset: f32,
}

/// Directories to skip during search.
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules"];

/// Files to skip during search.
const SKIP_FILES: &[&str] = &[".DS_Store"];

/// Maximum total matches before stopping.
const MAX_MATCHES: usize = 1000;

/// Maximum line length to store for display.
const MAX_LINE_LEN: usize = 200;

/// Number of bytes to sample for binary detection.
const BINARY_DETECT_SIZE: usize = 512;

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            total_matches: 0,
            active: false,
            selected: None,
            scroll_offset: 0.0,
        }
    }

    /// Toggle search mode. Clears query and results when deactivating.
    pub fn toggle(&mut self) {
        self.active = !self.active;
        if !self.active {
            self.query.clear();
            self.results.clear();
            self.total_matches = 0;
            self.selected = None;
            self.scroll_offset = 0.0;
        }
    }

    /// Append a character to the query.
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
    }

    /// Remove the last character from the query.
    pub fn backspace(&mut self) {
        self.query.pop();
    }

    /// Walk project_dir recursively and find case-insensitive substring matches.
    pub fn execute_search(&mut self, project_dir: &Path) {
        self.results.clear();
        self.total_matches = 0;
        self.scroll_offset = 0.0;

        if self.query.is_empty() {
            return;
        }

        let query_lower = self.query.to_lowercase();
        let mut all_results = Vec::new();
        self.walk_and_search(project_dir, project_dir, &query_lower, &mut all_results);

        // Sort by relative path
        all_results.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        self.total_matches = all_results.iter().map(|r| r.matches.len()).sum();
        self.results = all_results;
    }

    fn walk_and_search(
        &self,
        dir: &Path,
        project_dir: &Path,
        query_lower: &str,
        results: &mut Vec<SearchFileResult>,
    ) {
        let current_total: usize = results.iter().map(|r| r.matches.len()).sum();
        if current_total >= MAX_MATCHES {
            return;
        }

        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };

        let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let current_total: usize = results.iter().map(|r| r.matches.len()).sum();
            if current_total >= MAX_MATCHES {
                return;
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                self.walk_and_search(&path, project_dir, query_lower, results);
            } else {
                if SKIP_FILES.contains(&name.as_str()) {
                    continue;
                }

                // Binary detection: read first 512 bytes, check for null byte
                if is_binary(&path) {
                    continue;
                }

                let remaining = MAX_MATCHES - current_total;
                if let Some(file_result) =
                    search_file(&path, project_dir, query_lower, remaining)
                {
                    results.push(file_result);
                }
            }
        }
    }

    /// Toggle expansion of a file result group.
    pub fn toggle_file_expansion(&mut self, file_idx: usize) {
        if let Some(result) = self.results.get_mut(file_idx) {
            result.expanded = !result.expanded;
        }
    }

    /// Build a flat list of entries for rendering and hit-testing.
    pub fn flat_entries(&self) -> Vec<SearchFlatEntry> {
        let mut entries = Vec::new();
        for (file_idx, file_result) in self.results.iter().enumerate() {
            entries.push(SearchFlatEntry::FileHeader(file_idx));
            if file_result.expanded {
                for (match_idx, _) in file_result.matches.iter().enumerate() {
                    entries.push(SearchFlatEntry::MatchLine(file_idx, match_idx));
                }
            }
        }
        entries
    }

    /// Scroll search results, clamped to valid range.
    pub fn scroll(&mut self, delta: f32, viewport_height: f32) {
        let entry_height = 28.0_f32;
        let total_height = self.flat_entries().len() as f32 * entry_height;
        // Account for header area (SEARCH + input + count)
        let header_area = 16.0 + 15.6 + 8.0 + 28.0 + 28.0;
        let content_height = total_height + header_area;
        self.scroll_offset = (self.scroll_offset + delta)
            .max(0.0)
            .min((content_height - viewport_height).max(0.0));
    }
}

/// Check if a file appears to be binary by looking for null bytes in the first 512 bytes.
fn is_binary(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return true; // Can't open = skip
    };
    let mut buf = [0u8; BINARY_DETECT_SIZE];
    let Ok(n) = file.read(&mut buf) else {
        return true;
    };
    buf[..n].contains(&0)
}

/// Search a single file for case-insensitive matches, returning up to `max_matches` results.
fn search_file(
    path: &Path,
    project_dir: &Path,
    query_lower: &str,
    max_matches: usize,
) -> Option<SearchFileResult> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    let rel_path = path
        .strip_prefix(project_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut matches = Vec::new();

    for (line_idx, line_result) in reader.lines().enumerate() {
        if matches.len() >= max_matches {
            break;
        }
        let Ok(line) = line_result else {
            continue;
        };
        let line_lower = line.to_lowercase();
        if let Some(pos) = line_lower.find(query_lower) {
            let truncated = if line.len() > MAX_LINE_LEN {
                format!("{}...", &line[..MAX_LINE_LEN])
            } else {
                line.clone()
            };
            matches.push(SearchMatch {
                line_number: line_idx + 1,
                line_content: truncated,
                match_start: pos,
                match_end: pos + query_lower.len(),
            });
        }
    }

    if matches.is_empty() {
        None
    } else {
        Some(SearchFileResult {
            path: path.to_path_buf(),
            rel_path,
            file_name,
            matches,
            expanded: true,
        })
    }
}
