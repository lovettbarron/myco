pub mod global;
pub mod persistence;
pub mod project;
pub mod registry;

pub use global::GlobalPreferences;
pub use persistence::{load_project_config, save_project_config, AutoSaveState};
pub use project::{CapConfig, CapType, ColumnConfig, LayoutConfig, ProjectConfig, ProjectMetadata};
pub use registry::{ProjectEntry, ProjectRegistry};
