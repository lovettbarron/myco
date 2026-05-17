# Phase 5: Configuration and Persistence - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase makes the workspace state survive application restarts. A .myco/ directory per project stores layout and theme; ~/.myco/ stores global preferences, project registry, and keyboard shortcuts. On launch, a project picker shows registered projects; selecting one restores the saved grid layout with correct cap types and file references. Keyboard shortcuts become fully customizable with chord support.

</domain>

<decisions>
## Implementation Decisions

### Config File Split
- **D-01:** Theme preference is per-project with global fallback. `.myco/config.json` can set a theme override; if absent, `~/.myco/preferences.json` theme applies. New projects inherit the global default (Dracula).
- **D-02:** Keyboard shortcuts are global only. Stored in `~/.myco/shortcuts.json`. No per-project override.
- **D-03:** Project config (`.myco/config.json`) contains: grid layout, cap types, active theme name, project metadata (name, description). Minimal and git-safe.
- **D-04:** No machine-specific paths in .myco/config.json. File references are relative to project root (e.g., `"file": "sketches/plan.tldr"` not absolute paths).

### Layout Save/Restore
- **D-05:** Layout restore includes grid structure + cap types + file references. Terminal caps restore with saved CWD (fresh shell at that directory). Canvas caps reopen their .tldr file. Markdown caps reopen their .md file.
- **D-06:** Layout does NOT include exact pixel split ratios in v1. Grid restores with equal splits. Sizes are a future enhancement.
- **D-07:** Auto-save is debounced (2-3 seconds after last structural change). Panel split, close, resize, cap type change, or file navigation triggers the debounce timer.
- **D-08:** Save writes to `.myco/config.json`. Single file, full overwrite (not patch).

### Startup Flow
- **D-09:** Launch without CLI argument shows a project picker (list from ~/.myco/projects.json). User selects which project to open.
- **D-10:** Launch with CLI argument (`myco /path/to/project`) opens that project directly, skipping the picker.
- **D-11:** Projects are auto-registered in `~/.myco/projects.json` on first open. User can manually remove stale entries from the sidebar project switcher.
- **D-12:** If a registered project's folder no longer exists, show it grayed-out with a "Locate" option to re-point to the new path. No auto-removal.
- **D-13:** Project picker is GPU-rendered (same rendering pipeline as everything else). Not a webview, not a system dialog.

### Keyboard Shortcuts
- **D-14:** Full rebinding — any action can be rebound to any key combo via the settings UI (Phase 4 settings overlay, Shortcuts section).
- **D-15:** Chord sequences supported from the start (e.g., Cmd+K then Cmd+S). Internal shortcut system handles timeout and partial match state.
- **D-16:** Conflict handling: override + notify. New binding replaces old silently, with a notification showing what was unbound ("Cmd+D removed from Panel Split"). User can undo.
- **D-17:** Shortcuts stored as an array of `{ "action": "panel_split_h", "keys": ["cmd+d"] }` objects in `~/.myco/shortcuts.json`. Chords represented as arrays: `"keys": ["cmd+k", "cmd+s"]`.
- **D-18:** Default shortcuts ship as a built-in fallback. User's `shortcuts.json` contains only overrides (sparse format). Missing actions use built-in defaults.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints (JSON config, folder-first), key decisions
- `.planning/REQUIREMENTS.md` — CFG-01 through CFG-05 and KEY-01 through KEY-03 requirements
- `.planning/ROADMAP.md` — Phase 5 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack, serde_json for config, dirs crate for home paths

### Prior Phase Context
- `.planning/phases/04-application-frame-and-theming/04-CONTEXT.md` — Settings overlay (D-08 to D-10), theme system (D-11 to D-15), project switcher (D-01 to D-04)
- `.planning/phases/03-webview-caps/03-CONTEXT.md` — Sidebar file tree design (extends with project picker)
- `.planning/phases/01-window-grid-and-build-pipeline/01-CONTEXT.md` — Grid layout architecture, panel chrome, fullscreen mechanism

### Key Implementation References
- `src/context.rs` — Existing .myco/ directory creation pattern (ensure_context_files)
- `src/terminal/history.rs` — JSON file read/write pattern with dirs::home_dir()
- `src/theme/loader.rs` — File scanning pattern (~/.myco/themes/)
- `src/settings.rs` — Settings overlay UI (shortcuts section will need rebinding UI)
- `src/input/keyboard.rs` — Current hardcoded shortcut dispatch (to be replaced with configurable system)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/terminal/history.rs` — JSON load/save with dirs::home_dir(), create_dir_all, serde_json. Direct pattern for config persistence.
- `src/theme/loader.rs` — File scanning in ~/.myco/ subdirectory. Pattern for loading shortcuts.json.
- `src/context.rs` — .myco/ directory bootstrapping per project. Extend for config.json.
- `src/settings.rs` — SettingsState with SettingsSection::Shortcuts. Already has KeyValueRow display; needs upgrade to interactive rebinding.
- `src/app.rs` — `project_dir: Option<PathBuf>` already tracked. Grid layout via taffy. Layout serialization needs taffy tree → JSON conversion.

### Established Patterns
- JSON config via serde_json (used throughout)
- dirs::home_dir() for ~/.myco/ paths
- Debounced operations (notify-debouncer-full in file watcher, 5-second git cache in status_bar)
- InputAction enum for all user actions (natural fit for shortcut action registry)

### Integration Points
- `App::resumed()` — Currently creates terminal/canvas/sidebar from scratch. Needs to read .myco/config.json and restore layout instead.
- `App::recompute_layout()` — Taffy tree mutations. Save trigger point.
- `src/input/keyboard.rs` — `handle_terminal_key` and `handle_generic_key` currently have hardcoded match arms. Replace with lookup into shortcut registry.
- `src/sidebar/mod.rs` — SidebarState.projects Vec already exists (Phase 4). Connect to ~/.myco/projects.json.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 05-configuration-and-persistence*
*Context gathered: 2026-05-17*
