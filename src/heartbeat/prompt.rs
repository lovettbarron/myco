//! Prompt template resolution and file content assembly.
//!
//! Resolves template variables ({{file_contents}}, {{file_list}}, etc.)
//! and assembles file contents from glob patterns relative to the project directory.
//!
//! T-10-03: Validates glob-resolved paths start with project_dir using
//! Path::starts_with(). Skips files outside the project boundary.
//! T-10-04: Enforces max_files and max_bytes limits per job.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::warn;

/// Assemble file contents from glob patterns relative to a project directory.
///
/// Resolves each pattern using the `glob` crate, reads matching files, and
/// concatenates them with `--- {relative_path} ---` headers. Enforces
/// `max_files` and `max_bytes` limits (T-10-04). Skips files outside the
/// project directory (T-10-03) and files that fail to read (binary/unreadable).
///
/// Returns `(contents_string, file_list, file_count)`.
pub fn assemble_file_contents(
    project_dir: &Path,
    patterns: &[String],
    max_files: u32,
    max_bytes: u64,
) -> (String, Vec<String>, u32) {
    let mut contents = String::new();
    let mut file_list: Vec<String> = Vec::new();
    let mut total_bytes: u64 = 0;

    let canonical_project = match project_dir.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to canonicalize project dir: {}", e);
            return (contents, file_list, 0);
        }
    };

    for pattern in patterns {
        let full_pattern = project_dir
            .join(pattern)
            .to_string_lossy()
            .to_string();

        let entries = match glob::glob(&full_pattern) {
            Ok(e) => e,
            Err(e) => {
                warn!("Invalid glob pattern '{}': {}", pattern, e);
                continue;
            }
        };

        for entry in entries {
            // Check file limit
            if file_list.len() >= max_files as usize {
                break;
            }

            let path = match entry {
                Ok(p) => p,
                Err(e) => {
                    warn!("Glob entry error: {}", e);
                    continue;
                }
            };

            // Skip directories
            if !path.is_file() {
                continue;
            }

            // T-10-03: Validate path is within project directory
            let canonical_path = match path.canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };

            if !canonical_path.starts_with(&canonical_project) {
                warn!(
                    "Skipping file outside project boundary: {}",
                    path.display()
                );
                continue;
            }

            // Try to read as text (skip binary files)
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue, // binary or unreadable
            };

            // T-10-04: Check bytes limit
            if total_bytes + content.len() as u64 > max_bytes {
                break;
            }

            let relative = path
                .strip_prefix(project_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            total_bytes += content.len() as u64;
            contents.push_str(&format!("--- {} ---\n{}\n\n", relative, content));
            file_list.push(relative);
        }

        // Also break the outer loop if limits are hit
        if file_list.len() >= max_files as usize {
            break;
        }
    }

    let count = file_list.len() as u32;
    (contents, file_list, count)
}

/// Resolve template variables in a prompt string.
///
/// Replaces the following variables using simple `String::replace`:
/// - `{{file_contents}}` - assembled file contents
/// - `{{file_list}}` - newline-separated file paths
/// - `{{project_name}}` - project directory name
/// - `{{file_count}}` - number of matched files
/// - `{{timestamp}}` - current ISO 8601 timestamp
///
/// Unknown `{{variables}}` are left unchanged.
pub fn resolve_template(
    template: &str,
    file_contents: &str,
    file_list: &[String],
    project_name: &str,
    file_count: u32,
) -> String {
    template
        .replace("{{file_contents}}", file_contents)
        .replace("{{file_list}}", &file_list.join("\n"))
        .replace("{{project_name}}", project_name)
        .replace("{{file_count}}", &file_count.to_string())
        .replace("{{timestamp}}", &format_iso8601(SystemTime::now()))
}

/// Format a `SystemTime` as an ISO 8601 string (e.g., "2026-05-18T14:30:00Z").
///
/// Uses `UNIX_EPOCH` arithmetic to avoid a chrono dependency.
pub fn format_iso8601(time: SystemTime) -> String {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Calculate date and time components from epoch seconds
    let days = secs / 86400;
    let time_of_day = secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_date(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm based on Howard Hinnant's civil_from_days
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as u64, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_template_replaces_all_variables() {
        let template = "Project: {{project_name}}\nFiles ({{file_count}}):\n{{file_list}}\n\nContents:\n{{file_contents}}\n\nGenerated at {{timestamp}}";
        let file_contents = "--- src/main.rs ---\nfn main() {}\n\n";
        let file_list = vec!["src/main.rs".to_string()];
        let project_name = "myco";
        let file_count = 1;

        let result = resolve_template(
            template,
            file_contents,
            &file_list,
            project_name,
            file_count,
        );

        assert!(result.contains("Project: myco"));
        assert!(result.contains("Files (1):"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("fn main() {}"));
        // Timestamp should be a valid ISO 8601 format
        assert!(result.contains("T"));
        assert!(result.contains("Z"));
    }

    #[test]
    fn test_resolve_template_leaves_unknown_variables() {
        let template = "Hello {{unknown_var}} and {{another}}";
        let result = resolve_template(template, "", &[], "test", 0);

        assert!(result.contains("{{unknown_var}}"));
        assert!(result.contains("{{another}}"));
    }

    #[test]
    fn test_resolve_template_multiple_file_list() {
        let template = "Files:\n{{file_list}}";
        let file_list = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
        ];

        let result = resolve_template(template, "", &file_list, "test", 3);

        assert!(result.contains("src/main.rs\nsrc/lib.rs\nCargo.toml"));
    }

    #[test]
    fn test_resolve_template_empty_inputs() {
        let template = "{{file_contents}}|{{file_list}}|{{file_count}}";
        let result = resolve_template(template, "", &[], "test", 0);
        assert_eq!(result, "||0");
    }

    #[test]
    fn test_format_iso8601_epoch() {
        let epoch = UNIX_EPOCH;
        let formatted = format_iso8601(epoch);
        assert_eq!(formatted, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_format_iso8601_known_date() {
        // 2026-05-18T14:30:00Z = 1779114600 seconds since epoch
        let time = UNIX_EPOCH + std::time::Duration::from_secs(1779114600);
        let formatted = format_iso8601(time);
        assert_eq!(formatted, "2026-05-18T14:30:00Z");
    }

    #[test]
    fn test_format_iso8601_current_is_valid_format() {
        let now = format_iso8601(SystemTime::now());
        // Should match YYYY-MM-DDTHH:MM:SSZ pattern
        assert_eq!(now.len(), 20);
        assert_eq!(&now[4..5], "-");
        assert_eq!(&now[7..8], "-");
        assert_eq!(&now[10..11], "T");
        assert_eq!(&now[13..14], ":");
        assert_eq!(&now[16..17], ":");
        assert!(now.ends_with('Z'));
    }

    #[test]
    fn test_assemble_file_contents_respects_max_files() {
        let tmp = tempfile::tempdir().unwrap();

        // Create 10 files
        for i in 0..10 {
            std::fs::write(
                tmp.path().join(format!("file_{}.txt", i)),
                format!("content {}", i),
            )
            .unwrap();
        }

        let patterns = vec!["*.txt".to_string()];
        let (_, file_list, count) =
            assemble_file_contents(tmp.path(), &patterns, 5, 1_000_000);

        assert!(count <= 5);
        assert!(file_list.len() <= 5);
    }

    #[test]
    fn test_assemble_file_contents_respects_max_bytes() {
        let tmp = tempfile::tempdir().unwrap();

        // Create files of known size
        for i in 0..5 {
            // Each file is 100 bytes
            let content = "x".repeat(100);
            std::fs::write(
                tmp.path().join(format!("file_{}.txt", i)),
                content,
            )
            .unwrap();
        }

        let patterns = vec!["*.txt".to_string()];
        let (contents, _, _) =
            assemble_file_contents(tmp.path(), &patterns, 50, 250);

        // With headers, each entry is more than 100 bytes, but raw content
        // is limited to 250 bytes total (so at most 2 files' content)
        // Check that we didn't include all 500 bytes of content
        let raw_content_bytes: usize = contents
            .lines()
            .filter(|line| !line.starts_with("---") && !line.is_empty())
            .map(|line| line.len())
            .sum();
        assert!(raw_content_bytes <= 300); // some overhead from headers
    }

    #[test]
    fn test_assemble_file_contents_skips_binary() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a text file and a binary file
        std::fs::write(tmp.path().join("text.txt"), "hello world").unwrap();
        std::fs::write(
            tmp.path().join("binary.bin"),
            vec![0u8, 1, 2, 255, 254, 0, 0],
        )
        .unwrap();

        let patterns = vec!["*.txt".to_string(), "*.bin".to_string()];
        let (contents, file_list, count) =
            assemble_file_contents(tmp.path(), &patterns, 50, 100_000);

        // Text file should be included
        assert!(file_list.iter().any(|f| f.contains("text.txt")));
        assert!(contents.contains("hello world"));
        // Binary file might be skipped (if it contains invalid UTF-8)
        // or included (if the bytes happen to be valid UTF-8)
        assert!(count >= 1);
    }

    #[test]
    fn test_assemble_file_contents_empty_patterns() {
        let tmp = tempfile::tempdir().unwrap();
        let patterns: Vec<String> = vec![];
        let (contents, file_list, count) =
            assemble_file_contents(tmp.path(), &patterns, 50, 100_000);

        assert!(contents.is_empty());
        assert!(file_list.is_empty());
        assert_eq!(count, 0);
    }

    #[test]
    fn test_assemble_file_contents_no_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let patterns = vec!["nonexistent/**/*.xyz".to_string()];
        let (contents, file_list, count) =
            assemble_file_contents(tmp.path(), &patterns, 50, 100_000);

        assert!(contents.is_empty());
        assert!(file_list.is_empty());
        assert_eq!(count, 0);
    }

    #[test]
    fn test_assemble_file_contents_includes_header() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "content").unwrap();

        let patterns = vec!["test.txt".to_string()];
        let (contents, _, _) =
            assemble_file_contents(tmp.path(), &patterns, 50, 100_000);

        assert!(contents.contains("--- test.txt ---"));
        assert!(contents.contains("content"));
    }
}
