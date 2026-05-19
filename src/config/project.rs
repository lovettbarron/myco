//! Project configuration data model.
//!
//! Defines the JSON schema for `.myco/config.json` per D-03, D-04, D-05, D-06.
//! All file paths are stored relative to the project root (no absolute paths).

use serde::{Deserialize, Serialize};

/// Top-level project configuration, serialized to `.myco/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Project metadata (name, description).
    pub metadata: ProjectMetadata,
    /// Grid layout configuration (v1 columns format).
    pub layout: LayoutConfig,
    /// Tree-based layout configuration (v2 tree format).
    /// Present when version >= 2. Used instead of layout.columns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_layout: Option<TreeLayoutConfig>,
    /// Active theme name (falls back to global preference if None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

/// Project metadata stored in the config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name (derived from folder name).
    pub name: String,
    /// Optional project description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Grid layout configuration: a list of columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Columns in the grid. Each column is either a single cap or a vertical stack.
    pub columns: Vec<ColumnConfig>,
}

/// A column in the grid layout.
///
/// Uses `#[serde(untagged)]` so JSON is either a single object (Single)
/// or an object with a `caps` array (Stack).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColumnConfig {
    /// A single cap filling the column.
    Single(CapConfig),
    /// A vertical stack of caps in one column.
    Stack {
        /// The caps stacked vertically.
        caps: Vec<CapConfig>,
    },
}

/// Configuration for a single cap (panel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapConfig {
    /// The type of cap.
    #[serde(rename = "type")]
    pub cap_type: CapType,
    /// File path relative to project root (for canvas/markdown caps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Working directory relative to project root (for terminal caps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// The type of cap (panel content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapType {
    /// Terminal emulator cap.
    Terminal,
    /// TLDraw canvas cap.
    Canvas,
    /// Markdown viewer/editor cap.
    Markdown,
    /// Agent monitor cap.
    #[serde(rename = "agent_monitor")]
    AgentMonitor,
    /// Heartbeat output cap.
    Heartbeat,
}

impl ProjectConfig {
    /// Create a ProjectConfig from the current application state.
    ///
    /// Walks the grid tree to reconstruct the column layout, converting
    /// absolute paths to project-relative paths (D-04).
    pub fn from_current_state(
        grid: &crate::grid::layout::GridLayout,
        panels: &[crate::grid::Panel],
        terminal_manager: Option<&crate::terminal::TerminalManager>,
        project_dir: &std::path::Path,
        theme_name: Option<&str>,
    ) -> Self {
        let root = grid.root();
        let children = grid.tree().children(root).unwrap_or_default();

        let mut columns = Vec::new();

        for child in children {
            if grid.is_column_container(child) {
                // This is a vertical stack (column container)
                let stack_children = grid.tree().children(child).unwrap_or_default();
                let caps: Vec<CapConfig> = stack_children
                    .iter()
                    .filter_map(|&node| {
                        let panel_id = grid
                            .panel_nodes()
                            .iter()
                            .find(|(n, _)| *n == node)
                            .map(|(_, id)| *id)?;
                        let panel = panels.iter().find(|p| p.id == panel_id)?;
                        Some(Self::cap_config_from_panel(
                            panel,
                            terminal_manager,
                            project_dir,
                        ))
                    })
                    .collect();
                if !caps.is_empty() {
                    columns.push(ColumnConfig::Stack { caps });
                }
            } else {
                // This is a single cap (leaf node)
                if let Some((_, panel_id)) =
                    grid.panel_nodes().iter().find(|(n, _)| *n == child)
                {
                    if let Some(panel) = panels.iter().find(|p| p.id == *panel_id) {
                        columns.push(ColumnConfig::Single(Self::cap_config_from_panel(
                            panel,
                            terminal_manager,
                            project_dir,
                        )));
                    }
                }
            }
        }

        // Fallback: if we ended up with empty columns, add a single terminal
        if columns.is_empty() {
            columns.push(ColumnConfig::Single(CapConfig {
                cap_type: CapType::Terminal,
                file: None,
                cwd: None,
            }));
        }

        let name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name,
                description: None,
            },
            layout: LayoutConfig { columns },
            tree_layout: None,
            theme: theme_name.map(|s| s.to_string()),
        }
    }

    /// Convert a Panel to a CapConfig, making paths relative to project_dir.
    /// Public version for use by layout.rs to_tree_config.
    pub fn cap_config_from_panel_public(
        panel: &crate::grid::Panel,
        terminal_manager: Option<&crate::terminal::TerminalManager>,
        project_dir: &std::path::Path,
    ) -> CapConfig {
        Self::cap_config_from_panel(panel, terminal_manager, project_dir)
    }

    /// Convert a Panel to a CapConfig, making paths relative to project_dir.
    fn cap_config_from_panel(
        panel: &crate::grid::Panel,
        terminal_manager: Option<&crate::terminal::TerminalManager>,
        project_dir: &std::path::Path,
    ) -> CapConfig {
        use crate::grid::PanelType;

        match panel.panel_type {
            PanelType::Terminal => {
                let cwd = terminal_manager
                    .and_then(|tm| tm.get(&panel.id))
                    .map(|ts| {
                        let effective = ts.effective_cwd();
                        make_relative(&effective, project_dir)
                    });
                CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd,
                }
            }
            PanelType::Canvas => {
                let file = panel
                    .canvas_id
                    .as_ref()
                    .map(|id| format!(".myco/canvas/{}.excalidraw", id));
                CapConfig {
                    cap_type: CapType::Canvas,
                    file,
                    cwd: None,
                }
            }
            PanelType::Markdown => {
                let file = panel
                    .file_path
                    .as_ref()
                    .map(|p| make_relative(p, project_dir));
                CapConfig {
                    cap_type: CapType::Markdown,
                    file,
                    cwd: None,
                }
            }
            PanelType::AgentMonitor => CapConfig {
                cap_type: CapType::AgentMonitor,
                file: None,
                cwd: None,
            },
            PanelType::Heartbeat => CapConfig {
                cap_type: CapType::Heartbeat,
                file: None,
                cwd: None,
            },
            PanelType::Placeholder => CapConfig {
                cap_type: CapType::Terminal,
                file: None,
                cwd: None,
            },
        }
    }
}

/// Convert an absolute path to a path relative to the project directory.
/// Returns the path as a String with forward slashes.
/// If the path is not under project_dir, returns it as-is (should not happen
/// for well-formed configs per D-04).
fn make_relative(path: &std::path::Path, project_dir: &std::path::Path) -> String {
    path.strip_prefix(project_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

/// Tree-based layout config (version 2). Replaces LayoutConfig for new configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeLayoutConfig {
    pub tree: TreeNodeConfig,
}

/// A node in the recursive layout tree config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_type")]
pub enum TreeNodeConfig {
    #[serde(rename = "leaf")]
    Leaf {
        cap: CapConfig,
        #[serde(default = "default_weight")]
        weight: f32,
    },
    #[serde(rename = "branch")]
    Branch {
        direction: String, // "horizontal" or "vertical"
        children: Vec<TreeNodeConfig>,
        #[serde(default)]
        weights: Vec<f32>,
    },
}

fn default_weight() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_config_serialization_roundtrip() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test-project".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![
                    ColumnConfig::Single(CapConfig {
                        cap_type: CapType::Terminal,
                        file: None,
                        cwd: Some(".".to_string()),
                    }),
                    ColumnConfig::Single(CapConfig {
                        cap_type: CapType::Markdown,
                        file: Some("docs/README.md".to_string()),
                        cwd: None,
                    }),
                ],
            },
            tree_layout: None,
            theme: Some("Dracula".to_string()),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: ProjectConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.metadata.name, "test-project");
        assert_eq!(deserialized.theme, Some("Dracula".to_string()));
        assert_eq!(deserialized.layout.columns.len(), 2);
    }

    #[test]
    fn test_project_config_deserialization_from_known_json() {
        let json = r#"{
            "version": 1,
            "metadata": { "name": "my-app", "description": "A cool app" },
            "layout": {
                "columns": [
                    { "type": "terminal", "cwd": "." },
                    { "type": "markdown", "file": "README.md" }
                ]
            },
            "theme": "Solarized"
        }"#;

        let config: ProjectConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.metadata.name, "my-app");
        assert_eq!(
            config.metadata.description,
            Some("A cool app".to_string())
        );
        assert_eq!(config.layout.columns.len(), 2);
        assert_eq!(config.theme, Some("Solarized".to_string()));
    }

    #[test]
    fn test_column_config_single_serde() {
        let single = ColumnConfig::Single(CapConfig {
            cap_type: CapType::Terminal,
            file: None,
            cwd: Some("src".to_string()),
        });

        let json = serde_json::to_string(&single).unwrap();
        let deserialized: ColumnConfig = serde_json::from_str(&json).unwrap();

        match deserialized {
            ColumnConfig::Single(cap) => {
                assert_eq!(cap.cap_type, CapType::Terminal);
                assert_eq!(cap.cwd, Some("src".to_string()));
            }
            ColumnConfig::Stack { .. } => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_column_config_stack_serde() {
        let stack = ColumnConfig::Stack {
            caps: vec![
                CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: None,
                },
                CapConfig {
                    cap_type: CapType::Markdown,
                    file: Some("notes.md".to_string()),
                    cwd: None,
                },
            ],
        };

        let json = serde_json::to_string(&stack).unwrap();
        let deserialized: ColumnConfig = serde_json::from_str(&json).unwrap();

        match deserialized {
            ColumnConfig::Stack { caps } => {
                assert_eq!(caps.len(), 2);
                assert_eq!(caps[0].cap_type, CapType::Terminal);
                assert_eq!(caps[1].cap_type, CapType::Markdown);
            }
            ColumnConfig::Single(_) => panic!("Expected Stack variant"),
        }
    }

    #[test]
    fn test_cap_type_lowercase_serialization() {
        assert_eq!(
            serde_json::to_string(&CapType::Terminal).unwrap(),
            "\"terminal\""
        );
        assert_eq!(
            serde_json::to_string(&CapType::Canvas).unwrap(),
            "\"canvas\""
        );
        assert_eq!(
            serde_json::to_string(&CapType::Markdown).unwrap(),
            "\"markdown\""
        );
    }

    #[test]
    fn test_no_absolute_paths_in_serialized_config() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Markdown,
                    file: Some("docs/plan.md".to_string()),
                    cwd: None,
                })],
            },
            tree_layout: None,
            theme: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        // No absolute path markers should appear
        assert!(!json.contains("\"/"), "JSON contains absolute path");
        // Specifically, no path starting with /Users or /home
        assert!(!json.contains("/Users/"), "JSON contains /Users/ path");
        assert!(!json.contains("/home/"), "JSON contains /home/ path");
    }

    #[test]
    fn test_make_relative_strips_prefix() {
        let project = std::path::Path::new("/Users/dev/my-project");
        let absolute = std::path::Path::new("/Users/dev/my-project/src/main.rs");
        assert_eq!(make_relative(absolute, project), "src/main.rs");
    }

    #[test]
    fn test_make_relative_outside_project() {
        let project = std::path::Path::new("/Users/dev/my-project");
        let outside = std::path::Path::new("/tmp/other/file.txt");
        // Should return path as-is since it's outside project
        assert_eq!(make_relative(outside, project), "/tmp/other/file.txt");
    }

    #[test]
    fn test_description_skip_serializing_if_none() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: None,
                })],
            },
            tree_layout: None,
            theme: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("description"));
        assert!(!json.contains("theme"));
    }

    // =========================================================================
    // TreeNodeConfig serde tests
    // =========================================================================

    #[test]
    fn test_tree_node_config_leaf_serde() {
        let leaf = TreeNodeConfig::Leaf {
            cap: CapConfig {
                cap_type: CapType::Terminal,
                file: None,
                cwd: Some(".".to_string()),
            },
            weight: 1.0,
        };

        let json = serde_json::to_string_pretty(&leaf).unwrap();
        assert!(json.contains("\"node_type\": \"leaf\""));
        assert!(json.contains("\"weight\": 1.0"));

        let roundtrip: TreeNodeConfig = serde_json::from_str(&json).unwrap();
        match roundtrip {
            TreeNodeConfig::Leaf { cap, weight } => {
                assert_eq!(cap.cap_type, CapType::Terminal);
                assert_eq!(weight, 1.0);
            }
            _ => panic!("Expected Leaf"),
        }
    }

    #[test]
    fn test_tree_node_config_branch_serde() {
        let branch = TreeNodeConfig::Branch {
            direction: "horizontal".to_string(),
            children: vec![
                TreeNodeConfig::Leaf {
                    cap: CapConfig {
                        cap_type: CapType::Terminal,
                        file: None,
                        cwd: None,
                    },
                    weight: 0.5,
                },
                TreeNodeConfig::Leaf {
                    cap: CapConfig {
                        cap_type: CapType::Markdown,
                        file: Some("README.md".to_string()),
                        cwd: None,
                    },
                    weight: 0.5,
                },
            ],
            weights: vec![0.5, 0.5],
        };

        let json = serde_json::to_string_pretty(&branch).unwrap();
        assert!(json.contains("\"node_type\": \"branch\""));
        assert!(json.contains("\"direction\": \"horizontal\""));

        let roundtrip: TreeNodeConfig = serde_json::from_str(&json).unwrap();
        match roundtrip {
            TreeNodeConfig::Branch { direction, children, weights } => {
                assert_eq!(direction, "horizontal");
                assert_eq!(children.len(), 2);
                assert_eq!(weights.len(), 2);
            }
            _ => panic!("Expected Branch"),
        }
    }

    #[test]
    fn test_tree_node_config_nested_serde() {
        let nested = TreeNodeConfig::Branch {
            direction: "horizontal".to_string(),
            children: vec![
                TreeNodeConfig::Leaf {
                    cap: CapConfig {
                        cap_type: CapType::Terminal,
                        file: None,
                        cwd: None,
                    },
                    weight: 1.0,
                },
                TreeNodeConfig::Branch {
                    direction: "vertical".to_string(),
                    children: vec![
                        TreeNodeConfig::Leaf {
                            cap: CapConfig {
                                cap_type: CapType::Terminal,
                                file: None,
                                cwd: None,
                            },
                            weight: 0.5,
                        },
                        TreeNodeConfig::Leaf {
                            cap: CapConfig {
                                cap_type: CapType::Canvas,
                                file: Some(".myco/canvas/sketch.excalidraw".to_string()),
                                cwd: None,
                            },
                            weight: 0.5,
                        },
                    ],
                    weights: vec![0.5, 0.5],
                },
            ],
            weights: vec![0.5, 0.5],
        };

        let json = serde_json::to_string_pretty(&nested).unwrap();
        let roundtrip: TreeNodeConfig = serde_json::from_str(&json).unwrap();

        match roundtrip {
            TreeNodeConfig::Branch { children, .. } => {
                assert_eq!(children.len(), 2);
                assert!(matches!(&children[0], TreeNodeConfig::Leaf { .. }));
                match &children[1] {
                    TreeNodeConfig::Branch { children: inner, direction, .. } => {
                        assert_eq!(direction, "vertical");
                        assert_eq!(inner.len(), 2);
                    }
                    _ => panic!("Expected nested Branch"),
                }
            }
            _ => panic!("Expected Branch"),
        }
    }
}
