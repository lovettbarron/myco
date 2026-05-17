# Phase 4: Application Frame and Theming - Research

**Researched:** 2026-05-16
**Domain:** GPU-rendered application chrome (status bars, navigation, settings overlay, color theme system)
**Confidence:** HIGH

## Summary

Phase 4 transforms Myco from a bare workspace into a visually complete desktop application. The work spans five areas: (1) refactoring the Theme struct to support base colors + derivation + ANSI palette + JSON serialization, (2) adding top stats bar and bottom git/path bar as fixed-height regions outside the grid, (3) extending the sidebar with a project switcher mini-list, (4) building a GPU-rendered settings overlay using the existing fullscreen mechanism, and (5) loading custom themes from ~/.myco/themes/ JSON files.

The codebase is well-prepared for this phase. Fixed-height regions (title bar at 38px, sidebar at 240px) already deducted from grid space establish the pattern for the new stats bar (24px) and bottom bar (24px). The existing Theme struct has 17 color fields hard-coded to Dracula linear-light values -- the refactor restructures this to base slots + derived fields while keeping the flat `[f32; 4]` access pattern renderers already use. The TerminalRenderer owns an `AnsiPalette` (16 ANSI colors + fg/bg) that currently defaults to Dracula -- theme switching will replace this palette instance. git2 is already a dependency and the `fetch_git_info` pattern in terminal/state.rs provides the exact branch + dirty detection needed for the bottom bar.

**Primary recommendation:** Implement as three vertical slices: (1) Theme system refactor + four built-in themes + custom JSON loading, (2) Status bars + bottom bar with git integration + sidebar project switcher, (3) Settings overlay with interactive theme switching. Slice 1 is foundational -- the other two depend on it.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Cross-project navigation is integrated into the existing file sidebar (combined element). No separate nav rail. One toggle (Cmd+B) controls the whole sidebar.
- **D-02:** A fixed-height mini-list (3-5 project icons/names) sits at the very top of the sidebar, always visible. The file tree scrolls independently below a separator.
- **D-03:** Project status indicators are minimal: a colored dot per project (green = running processes, gray = idle).
- **D-04:** Clicking a different project does a full window switch -- new file tree, new grid layout, new terminal contexts. All workspace state changes in-place.
- **D-05:** Top stats bar is a separate strip below the custom title bar (not merged into it). Title bar retains traffic lights + breadcrumb.
- **D-06:** Top stats bar uses a configurable slots architecture (3-4 slots). For v1, slots show panel count and uptime. Architecture supports future token usage, LLM status, etc.
- **D-07:** Bottom bar displays git branch name, dirty/clean indicator, and the project folder path. Git integration uses the git2 crate already in the dependency stack.
- **D-08:** Settings is GPU-rendered (not a webview). Same rendering pipeline as terminal and markdown.
- **D-09:** Settings has interactive forms: dropdown for theme selection, font picker, keybinding editor, toggles. Full interactive UI built with the GPU renderer.
- **D-10:** Cmd+, opens settings as a fullscreen overlay (fills entire workspace area, other panels hidden underneath). Esc returns to the workspace. Uses the same mechanism as panel fullscreen (D-11 from Phase 1).
- **D-11:** Themes use a layered definition: ~8-10 base colors that auto-derive the full palette, with optional per-field overrides for power users. Simple themes are easy to create; full control is available.
- **D-12:** Each theme includes a 16-color ANSI terminal palette. Switching themes changes both UI and terminal colors together for a consistent aesthetic.
- **D-13:** Three built-in themes: Dracula (currently implemented in src/theme.rs -- remains the default), Solarized (exact Ethan Schoonover colors, both light and dark variants), and Obsidian minimalist (true dark gray/charcoal with low-saturation accents inspired by Obsidian's default dark theme).
- **D-14:** Dracula is the default theme on fresh install. The existing color values in Theme::dark() are preserved as the Dracula theme definition.
- **D-15:** Custom themes are JSON files in ~/.myco/themes/. App scans this directory on startup. Theme names derived from filenames. Share themes by copying files.

### Claude's Discretion
- Internal struct naming and module organization for the theme system
- Implementation details of the derived color computation
- How to structure the settings overlay rendering (single render function vs modular components)
- How to handle project registry for the project switcher (data structure, persistence format)

### Deferred Ideas (OUT OF SCOPE)
- Obsidian-style extensions to markdown (callouts, wikilinks, math)
- Token usage / LLM status in top bar slots (Phase 6)
- Per-project theme overrides (theme in .myco file)
- Settings keybinding editor (Cmd+K style chord recording) -- may simplify in v1
- Theme hot-reload (auto-update when theme JSON changes on disk)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FRAME-01 | Application has a left navigation bar for cross-project switching with project status indicators | Sidebar already renders at 240px; extend with 80px fixed project switcher section at top. Project registry stored in ~/.myco/projects.json. Green/gray dot via QuadInstance circles. |
| FRAME-02 | Application has a top bar displaying macro-level information (placeholder stats surface) | New 24px fixed region below title bar. StatsSlot struct with label+value+visible. Deducted from grid height in recompute_layout. |
| FRAME-03 | Application has a bottom bar displaying in-project information | New 24px fixed region at window bottom. git2::Repository::discover for branch name, diff_index_to_workdir for dirty status. Existing pattern in terminal/state.rs. |
| FRAME-04 | User can open settings via Cmd+, shortcut | New InputAction::OpenSettings variant. AppState enum (Workspace/Settings) controls render path. Cmd+, handled in keyboard.rs. |
| FRAME-05 | Settings view allows configuration of theme, fonts, keyboard shortcuts, and project preferences | GPU-rendered overlay with left nav + content area. Theme dropdown triggers immediate theme swap. Font picker lists cosmic-text FontSystem families. |
| THEME-01 | Application ships with Solarized and Obsidian minimalist themes as defaults | Four built-in ThemeDefinitions (Dracula, Solarized Dark, Solarized Light, Obsidian). All color values specified in UI-SPEC with exact hex. |
| THEME-02 | User can switch themes from settings and the change applies immediately across all panels | Replace self.theme, replace terminal_renderer.palette, request_redraw. Webviews NOT re-themed (independent styling). |
| THEME-03 | Theme system is configurable enough for users to create custom color schemes | JSON files in ~/.myco/themes/ with base + ansi + overrides schema. serde_json deserialization into ThemeDefinition. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Theme definition & derivation | Data model (src/theme.rs) | -- | Pure data transformation: base colors -> derived palette. No rendering involved. |
| Theme JSON loading | Data model + filesystem | -- | serde_json deserialization + dirs::home_dir() for ~/.myco/themes/ |
| Top stats bar rendering | GPU renderer (app.rs build_quads) | -- | Fixed-height quad region + text labels, same as title bar pattern |
| Bottom bar rendering | GPU renderer (app.rs build_quads) | -- | Fixed-height quad region + text labels |
| Bottom bar git status | Background/async | GPU renderer | git2 queries on timer/trigger, results cached, rendered by GPU |
| Project switcher | Sidebar module (sidebar/) | App state | Extends existing SidebarState + SidebarRenderer with project list section |
| Project switch logic | App state (app.rs) | Terminal/Canvas managers | Tears down current workspace, loads new config, rebuilds everything |
| Settings overlay | GPU renderer | Input system | New render path when AppState::Settings, captures all input |
| Settings form controls | GPU renderer | -- | Dropdown, toggle, text display -- all custom QuadInstance + TextLabel |
| Theme switching | App state (app.rs) | Terminal renderer | Replace Theme + AnsiPalette, request redraw |
| Custom theme scanning | Filesystem (startup) | -- | Read ~/.myco/themes/*.json, validate, add to available themes list |

## Standard Stack

### Core (already in Cargo.toml -- no new dependencies needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wgpu | 29.0.3 | GPU rendering (quads, text) | Already rendering all UI. Stats bars, bottom bar, settings overlay all use existing QuadRenderer + TextEngine. [VERIFIED: cargo tree] |
| glyphon | 0.11.0 | GPU text rendering | Already rendering sidebar, terminal, markdown text. Settings forms use same pattern. [VERIFIED: cargo tree] |
| serde + serde_json | 1.0.228 / 1.0.149 | Theme JSON parsing | Already in dependencies. Used for menu config and terminal history. Theme JSON is a natural fit. [VERIFIED: cargo tree] |
| git2 | 0.20.4 | Git branch + dirty status for bottom bar | Already a dependency. Existing fetch_git_info() in terminal/state.rs provides the exact pattern. [VERIFIED: cargo tree / codebase grep] |
| taffy | 0.10.1 | Layout computation | Grid layout already deducts fixed regions (title bar, sidebar). Stats bar and bottom bar follow same pattern. [VERIFIED: cargo tree] |
| dirs | 6.0.0 | Home directory resolution (~/.myco/) | Already used for terminal history path. [VERIFIED: cargo tree] |

### Supporting (no new additions)

This phase requires zero new crate dependencies. All rendering, serialization, git integration, and filesystem access use existing dependencies.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled color derivation | palette crate | Overkill for 10 base -> 20 derived colors. Simple lighten/darken math suffices. |
| Custom JSON theme format | TOML themes | Project decision: JSON for AI tool compatibility (CLAUDE.md constraint). |
| Webview settings panel | GPU-rendered settings | D-08 locks this to GPU. More work but consistent with "everything is GPU-rendered" philosophy. |

**Installation:**
```bash
# No new dependencies needed. Existing Cargo.toml covers everything.
cargo build
```

## Architecture Patterns

### System Architecture Diagram

```
                        Startup
                           |
                    [Load Config]
                           |
              +------------+------------+
              |                         |
    [Scan ~/.myco/themes/]    [Load Built-in Themes]
              |                         |
              +-----------+-------------+
                          |
                   ThemeRegistry
                   (available themes)
                          |
            +-------------+-------------+
            |             |             |
      [App State]   [Settings UI]  [Theme JSON]
            |             |             |
    +-------+-------+    |    +--------+--------+
    |       |       |    |    |        |        |
  Theme  AnsiPal  Build  |  Parse  Validate  Derive
    |       |     Quads  |    |        |        |
    v       v       |    v    v        v        v
  [All    [Term   [Render]  [User selects theme]
   GPU     Color           [Replace Theme + Palette]
   Panels] Resolve]        [request_redraw()]
```

### Window Layout Regions (vertical stack)

```
+----------------------------------------------------------+
| Title Bar (38px) - traffic lights + breadcrumb            |
+----------------------------------------------------------+
| Stats Bar (24px) - panel count | uptime | [empty slots]   |
+------+-------------------------------------------------+--+
|Proj  |                                                 |
|Switch|              Grid Layout                        |
|(80px)|         (panels fill remaining space)           |
|------|                                                 |
| File |                                                 |
| Tree |                                                 |
|(scroll)                                                |
|240px |                                                 |
+------+-------------------------------------------------+--+
| Bottom Bar (24px) - git branch + dirty dot | project path |
+----------------------------------------------------------+
```

### Recommended Module Structure

```
src/
├── theme.rs              # Refactored: ThemeBase, ThemeDerived, ThemeDefinition, Theme, AnsiPalette
├── theme/                # Alternative: split into submodules if theme.rs gets too large
│   ├── mod.rs            # Re-exports, ThemeRegistry
│   ├── definition.rs     # ThemeBase, ThemeDerived, ThemeDefinition structs
│   ├── builtin.rs        # Dracula, Solarized Dark/Light, Obsidian definitions
│   ├── loader.rs         # JSON file loading from ~/.myco/themes/
│   └── colors.rs         # hex_to_linear, srgb_to_linear, derivation functions
├── bars/                 # New: status bar rendering
│   ├── mod.rs            # StatsBar, BottomBar state structs
│   ├── stats.rs          # StatsSlot, stats bar quad/text generation
│   └── bottom.rs         # Bottom bar quad/text generation, git status caching
├── settings/             # New: settings overlay
│   ├── mod.rs            # SettingsState, section navigation
│   ├── renderer.rs       # GPU quad/text generation for settings UI
│   └── controls.rs       # Dropdown, Toggle, TextDisplay control logic
├── sidebar/
│   ├── mod.rs            # Extended: ProjectEntry, project switcher state
│   └── renderer.rs       # Extended: project mini-list rendering
├── app.rs                # Extended: AppState enum, layout deductions, settings dispatch
└── input/
    ├── mod.rs            # Extended: OpenSettings, ThemeSwitch, ProjectSwitch actions
    └── keyboard.rs       # Extended: Cmd+, handler
```

### Pattern 1: Fixed-Height Region (Stats Bar / Bottom Bar)

**What:** Non-grid fixed regions deducted from available space before grid layout computation.
**When to use:** Any UI element that has a fixed pixel height and spans the window width (or width minus sidebar).
**Example:**

```rust
// Source: Existing pattern in app.rs (title bar deduction at line 1325)
// Stats bar and bottom bar follow the identical pattern.

const STATS_BAR_HEIGHT: f32 = 24.0;
const BOTTOM_BAR_HEIGHT: f32 = 24.0;

fn recompute_layout(&mut self) {
    if let (Some(grid), Some(window)) = (self.grid.as_mut(), self.window.as_ref()) {
        let size = window.inner_size();
        let w = size.width as f32 / self.scale_factor;
        let h = size.height as f32 / self.scale_factor;

        // Deduct ALL fixed regions from grid height
        let grid_height = h - TITLE_BAR_HEIGHT - STATS_BAR_HEIGHT - BOTTOM_BAR_HEIGHT;

        let sidebar_w = if self.sidebar.as_ref().map(|s| s.visible).unwrap_or(false) {
            SIDEBAR_WIDTH
        } else {
            0.0
        };
        let grid_width = w - sidebar_w;
        grid.compute(grid_width, grid_height.max(1.0));
    }
}
```

### Pattern 2: Theme Struct Refactor (Layered Definition)

**What:** Base colors define the theme; derived colors are computed at load time; flat struct for renderer access.
**When to use:** When themes need both simplicity (create from 10 colors) and power (override any field).

```rust
// Source: UI-SPEC color architecture section

/// The 10 base semantic colors that define a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeBase {
    pub bg_primary: String,    // hex string e.g. "#282A36"
    pub bg_secondary: String,
    pub bg_tertiary: String,
    pub fg_primary: String,
    pub fg_secondary: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub border: String,
}

/// ANSI terminal palette (16 colors + fg/bg).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeAnsi {
    pub colors: Vec<String>,  // 16 hex strings
    pub foreground: String,
    pub background: String,
}

/// What's stored in JSON / built-in definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDefinition {
    pub name: String,
    pub base: ThemeBase,
    pub ansi: ThemeAnsi,
    #[serde(default)]
    pub overrides: HashMap<String, String>,
}

/// Fully resolved theme ready for rendering. Flat [f32; 4] fields.
/// Constructed from ThemeDefinition via derive().
pub struct Theme {
    // All existing fields preserved (background, panel_background, etc.)
    // Plus new fields for stats bar, bottom bar, settings UI
    pub background: [f32; 4],
    pub panel_background: [f32; 4],
    // ... all current fields ...
    // New: stats bar and bottom bar use existing semantic colors
}

impl Theme {
    /// Derive a full Theme from a ThemeDefinition.
    pub fn from_definition(def: &ThemeDefinition) -> Self {
        let bg_primary = hex_to_linear(&def.base.bg_primary);
        let bg_secondary = hex_to_linear(&def.base.bg_secondary);
        // ... derive all fields from base ...
        // Apply overrides last
        Self { /* ... */ }
    }
}
```

### Pattern 3: AppState for Settings Overlay

**What:** Enum controlling the render path -- workspace vs settings vs future states.
**When to use:** Any full-window overlay that replaces the normal grid rendering.

```rust
// Source: CONTEXT.md D-10, Phase 1 D-11 fullscreen mechanism

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    Workspace,
    Settings,
}

// In render loop:
match self.app_state {
    AppState::Workspace => {
        // Normal grid rendering (existing code)
    }
    AppState::Settings => {
        // Settings overlay rendering (skip grid, render settings UI)
        // Input captured by settings handler
    }
}
```

### Pattern 4: Git Status Caching for Bottom Bar

**What:** Periodic git status polling with caching to avoid per-frame git2 calls.
**When to use:** Any data that's expensive to compute but doesn't need per-frame freshness.

```rust
// Source: Existing pattern in terminal/state.rs lines 411-433

struct BottomBarState {
    branch: Option<String>,
    is_dirty: bool,
    project_path: String,
    last_refresh: Instant,
}

impl BottomBarState {
    fn refresh_if_stale(&mut self, project_dir: &Path) {
        if self.last_refresh.elapsed() > Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            if let Ok(repo) = git2::Repository::discover(project_dir) {
                if let Ok(head) = repo.head() {
                    self.branch = head.shorthand().map(|s| s.to_string());
                }
                // Check dirty: any status entries means dirty
                let mut opts = git2::StatusOptions::new();
                opts.include_untracked(true);
                self.is_dirty = repo.statuses(Some(&mut opts))
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);
            } else {
                self.branch = None;
                self.is_dirty = false;
            }
        }
    }
}
```

### Anti-Patterns to Avoid

- **Per-frame git2 calls:** git2::Repository::discover does filesystem traversal. Cache with 5-second TTL (existing pattern in terminal/state.rs). [VERIFIED: codebase pattern]
- **Storing linear-light colors in JSON:** JSON stores sRGB hex strings (human-readable). Convert to linear-light `[f32; 4]` at load time. The reverse conversion (`linear_to_srgb_u8`) already exists in theme.rs. [VERIFIED: codebase]
- **Theme struct with Option fields:** Keep the Theme struct flat with all fields resolved. The layered definition (ThemeDefinition -> Theme) is a construction pattern, not a runtime concern. Renderers should never check for missing colors. [ASSUMED]
- **Modifying the grid layout for settings overlay:** Settings is NOT a panel. It's a separate render path controlled by AppState. Don't touch the grid -- just skip grid rendering and render the settings UI instead. [VERIFIED: Phase 1 D-11 fullscreen mechanism]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| sRGB hex parsing | Custom hex parser | Standard u8::from_str_radix + sRGB-to-linear math | Two lines of code, well-known formula. The inverse (linear_to_srgb_u8) already exists. |
| Git branch detection | Shell out to `git` | git2::Repository::discover + head().shorthand() | Already a dependency. Existing pattern in terminal/state.rs. No subprocess overhead. |
| JSON theme parsing | Custom parser | serde_json + #[derive(Deserialize)] | Already a dependency with derive feature enabled. |
| Home directory | Hardcoded path | dirs::home_dir() | Already used for ~/.myco/history.json. Cross-platform. |
| Font enumeration | Filesystem scanning | cosmic-text FontSystem::db().faces() | FontSystem already loaded. Has all system fonts indexed. |
| Color lightening/darkening | Complex color space math | Simple linear interpolation on sRGB components | For 3-5% shifts, sRGB linear interpolation is perceptually adequate. No need for Lab/Oklab. |

**Key insight:** This phase adds zero new crate dependencies. Every capability (rendering, serialization, git, filesystem) is already available in the existing stack.

## Common Pitfalls

### Pitfall 1: Color Space Confusion (sRGB vs Linear-Light)

**What goes wrong:** Theme colors look washed out or too dark because sRGB hex values are used directly as wgpu colors (which expects linear-light on sRGB surfaces).
**Why it happens:** wgpu with an sRGB surface format expects linear-light values. sRGB hex (#282A36) must be converted: `channel = if srgb <= 0.04045 { srgb / 12.92 } else { ((srgb + 0.055) / 1.055).powf(2.4) }`.
**How to avoid:** All hex-to-color conversion goes through a single `hex_to_linear()` function. The existing `linear_to_srgb_u8()` in theme.rs does the inverse for glyphon. Both directions must use the same gamma curve.
**Warning signs:** Colors in the app don't match the hex values when compared visually. Especially noticeable with Solarized's precise color science. [VERIFIED: existing theme.rs uses linear-light values]

### Pitfall 2: Grid Layout Offset Cascade

**What goes wrong:** After adding stats bar and bottom bar heights, panel positions are wrong, mouse hit-testing is off, or content overflows.
**Why it happens:** Multiple places in app.rs use `TITLE_BAR_HEIGHT` as an offset. Adding two more fixed regions means every `py + TITLE_BAR_HEIGHT` must become `py + TITLE_BAR_HEIGHT + STATS_BAR_HEIGHT`, and panel content height must also deduct bottom bar.
**How to avoid:** Define a single `grid_y_offset()` method that returns `TITLE_BAR_HEIGHT + STATS_BAR_HEIGHT`. Audit every use of `TITLE_BAR_HEIGHT` in app.rs (there are 20+ occurrences). Use the method everywhere instead of raw constants.
**Warning signs:** Panels overlap the stats bar, mouse clicks on panels are off by 24px, bottom bar overlaps last panel row. [VERIFIED: grep shows 20+ TITLE_BAR_HEIGHT references in app.rs]

### Pitfall 3: Terminal Palette Not Updated on Theme Switch

**What goes wrong:** User switches theme but terminal colors remain Dracula.
**Why it happens:** `TerminalRenderer` owns its own `AnsiPalette` (line 111 of terminal/renderer.rs). Theme switch replaces `self.theme` but doesn't touch `self.terminal_renderer.palette`.
**How to avoid:** Theme switch must update both `self.theme` AND `self.terminal_renderer.palette`. Also invalidate the terminal buffer cache (row content hashes include color info).
**Warning signs:** After theme switch, terminal text colors don't match the new theme. Old terminal output stays in old colors. [VERIFIED: terminal/renderer.rs line 94 shows hash_row uses palette]

### Pitfall 4: Sidebar Project Switcher Y-Offset Breaks File Tree

**What goes wrong:** File tree entries are positioned wrong after adding the 80px project switcher section.
**Why it happens:** SidebarRenderer.prepare_buffers() calculates `header_offset = viewport_y + 16.0 + 15.6 + 8.0` (the "FILES" heading). Adding project switcher above this means the heading must shift down by ~80px, and all entry_at_y() calculations must account for it.
**How to avoid:** Extract the project switcher height as a constant (e.g., `PROJECT_SWITCHER_HEIGHT: f32 = 80.0`). Update both build_quads and prepare_buffers to deduct this from the file tree area. Update entry_at_y() similarly.
**Warning signs:** File tree overlaps project switcher, clicking on a file entry selects the wrong file, "FILES" heading appears at the top of the sidebar instead of below the project list. [VERIFIED: sidebar/renderer.rs line 42-43 shows hard-coded header offset]

### Pitfall 5: Settings Overlay Input Leak

**What goes wrong:** Keyboard shortcuts meant for settings controls trigger workspace actions (e.g., typing in a text field triggers Cmd+D split).
**Why it happens:** The keyboard handler in keyboard.rs processes shortcuts before settings forms get input.
**How to avoid:** When `AppState::Settings`, the input handler should route ALL input to the settings controller first. Only Esc and Cmd+, should be handled at the app level (to close settings).
**Warning signs:** Pressing keyboard shortcuts while settings is open affects the workspace underneath. [VERIFIED: keyboard.rs processes shortcuts globally]

## Code Examples

### hex_to_linear: Parse hex color to wgpu linear-light

```rust
// Source: Standard sRGB-to-linear conversion formula
// Inverse of existing linear_to_srgb_u8() in theme.rs

/// Parse a hex color string (e.g., "#282A36" or "282A36") to linear-light [f32; 4].
pub fn hex_to_linear(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}

/// Convert a single sRGB u8 channel to linear-light f32.
fn srgb_to_linear(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
```

### ThemeDefinition to AnsiPalette conversion

```rust
// Source: CONTEXT.md D-12 + existing AnsiPalette in terminal/colors.rs

impl ThemeDefinition {
    /// Convert theme's ANSI section to a terminal palette.
    pub fn to_ansi_palette(&self) -> AnsiPalette {
        let mut colors = [[0u8; 3]; 16];
        for (i, hex) in self.ansi.colors.iter().enumerate().take(16) {
            colors[i] = hex_to_srgb_u8(hex);
        }
        AnsiPalette {
            colors,
            foreground: hex_to_srgb_u8(&self.ansi.foreground),
            background: hex_to_srgb_u8(&self.ansi.background),
        }
    }
}

/// Parse hex to sRGB u8 triple (NOT linear-light -- AnsiPalette stores sRGB).
fn hex_to_srgb_u8(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    [
        u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
        u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
        u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
    ]
}
```

### Stats bar quad + text generation

```rust
// Source: Existing build_quads pattern in app.rs + UI-SPEC stats bar section

fn build_stats_bar_quads(
    &self,
    width: f32,
    sidebar_offset: f32,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();
    let bar_y = TITLE_BAR_HEIGHT;
    let bar_w = width - sidebar_offset;

    // Background (bg_primary -- blends with window)
    quads.push(QuadInstance {
        position: [sidebar_offset, bar_y],
        size: [bar_w, STATS_BAR_HEIGHT],
        color: self.theme.background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    // Slot separators (1px vertical lines, 12px tall, centered)
    let slot_width = bar_w / 4.0;
    for i in 1..4 {
        let sep_x = sidebar_offset + (i as f32 * slot_width);
        quads.push(QuadInstance {
            position: [sep_x, bar_y + 6.0],
            size: [1.0, 12.0],
            color: self.theme.divider,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }

    quads
}
```

### Theme switching in process_action

```rust
// Source: CONTEXT.md D-12, UI-SPEC state transitions section

InputAction::ThemeSwitch { theme_name } => {
    if let Some(definition) = self.theme_registry.get(&theme_name) {
        // 1. Replace app theme
        self.theme = Theme::from_definition(definition);

        // 2. Replace terminal ANSI palette
        self.terminal_renderer.palette = definition.to_ansi_palette();

        // 3. Invalidate terminal buffer cache (colors changed)
        self.terminal_renderer.invalidate_all_caches();

        // 4. Request full redraw
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
```

### Custom theme loading from ~/.myco/themes/

```rust
// Source: CONTEXT.md D-15, dirs crate usage pattern from terminal/mod.rs

fn load_custom_themes() -> Vec<ThemeDefinition> {
    let mut themes = Vec::new();
    let themes_dir = match dirs::home_dir() {
        Some(home) => home.join(".myco").join("themes"),
        None => return themes,
    };

    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => match serde_json::from_str::<ThemeDefinition>(&contents) {
                        Ok(def) => {
                            info!("Loaded custom theme: {} from {:?}", def.name, path);
                            themes.push(def);
                        }
                        Err(e) => warn!("Failed to parse theme {:?}: {}", path, e),
                    },
                    Err(e) => warn!("Failed to read theme {:?}: {}", path, e),
                }
            }
        }
    }
    themes
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Theme::dark() with hardcoded linear values | ThemeDefinition with hex strings + derivation | This phase | Themes become data-driven, user-configurable |
| Single fixed Dracula palette | Four built-in themes + custom JSON loading | This phase | Users can personalize appearance |
| No status bars | Top stats + bottom git/path bar | This phase | Application feels like a complete desktop app |
| No settings UI | GPU-rendered settings overlay | This phase | Users can configure without editing files |
| TerminalRenderer owns fixed AnsiPalette | AnsiPalette derived from active theme | This phase | Terminal colors match UI theme |

**Deprecated/outdated:**
- `Theme::dark()` constructor: Will be replaced by `Theme::from_definition(&DRACULA_DEFINITION)`. The method can remain as a convenience alias but should delegate to the definition system. [ASSUMED]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Theme struct should remain flat (direct field access) rather than using nested structs for renderer performance | Anti-Patterns | Low -- nested access is also fast in Rust, just less ergonomic |
| A2 | Theme::dark() should delegate to from_definition() rather than being removed | State of the Art | Low -- implementation detail, either approach works |
| A3 | sRGB linear interpolation is perceptually adequate for 3-5% lighten/darken operations | Don't Hand-Roll | Low -- at small shifts the difference from Lab is imperceptible |
| A4 | Project registry for the switcher should be a simple JSON file at ~/.myco/projects.json | Architecture | Medium -- Phase 5 (Configuration) may have different ideas about global config structure |
| A5 | Settings keybinding editor can be simplified to a read-only display in v1 (per deferred items) | Requirements FRAME-05 | Low -- CONTEXT.md explicitly defers chord recording |
| A6 | Font picker can list fonts from cosmic-text FontSystem::db().faces() | Requirements FRAME-05 | Medium -- need to verify this API exists and returns usable font family names |

## Open Questions

1. **Project registry persistence format** — RESOLVED
   - What we know: Projects need to be listed in the sidebar switcher. ~/.myco/ is the global config location. The switcher shows 3-5 entries.
   - Resolution: Use ~/.myco/projects.json for v1 (simple array of {path, name}). Phase 5 can consolidate into global config if needed. Plan 02 initializes with current project only.

2. **Settings form interactivity scope for v1** — RESOLVED
   - What we know: D-09 specifies dropdown, font picker, keybinding editor, toggles. But keybinding editor is deferred.
   - Resolution: Theme dropdown (fully interactive) + font family as read-only label showing "JetBrains Mono" (interactive font picker deferred to Phase 5) + read-only keybinding display + simple toggles. This satisfies FRAME-05 without deferred components.

3. **Project switch teardown completeness** — RESOLVED
   - What we know: D-04 specifies full window switch -- new file tree, grid layout, terminal contexts.
   - Resolution: For v1, wire the action and UI only. Full teardown/rebuild requires config persistence (Phase 5 CFG-04). Plan 02 adds the ProjectSwitch action as a stub with info log.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git2 (libgit2) | Bottom bar git status | Yes | 0.20.4 (bundled source) | Show "No repository" if git2 fails |
| ~/.myco/ directory | Custom theme loading | Created on demand | -- | Skip custom themes if dir doesn't exist |
| System fonts | Font picker in settings | Yes (macOS) | -- | Use cosmic-text defaults |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None. All dependencies are already resolved.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in Rust test framework) |
| Config file | None -- standard `#[cfg(test)]` modules |
| Quick run command | `cargo test` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| THEME-01 | Four built-in theme definitions produce valid Theme structs | unit | `cargo test theme::tests -x` | Wave 0 |
| THEME-02 | Theme switch replaces Theme + AnsiPalette | unit | `cargo test theme::tests::test_theme_switch -x` | Wave 0 |
| THEME-03 | Custom JSON theme parses to valid ThemeDefinition | unit | `cargo test theme::tests::test_json_parsing -x` | Wave 0 |
| FRAME-03 | Git status detection (branch name, dirty flag) | unit | `cargo test bars::tests::test_git_status -x` | Wave 0 |
| THEME-01 | hex_to_linear produces correct linear-light values | unit | `cargo test theme::tests::test_hex_to_linear -x` | Wave 0 |
| THEME-01 | hex_to_linear round-trips with linear_to_srgb_u8 | unit | `cargo test theme::tests::test_color_roundtrip -x` | Wave 0 |
| THEME-03 | Invalid JSON theme gracefully fails with warning | unit | `cargo test theme::tests::test_invalid_json -x` | Wave 0 |
| FRAME-02 | StatsSlot rendering produces correct quad positions | unit | `cargo test bars::tests::test_stats_slots -x` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `src/theme.rs` (or `src/theme/mod.rs`) -- test module for hex_to_linear, color roundtrip, ThemeDefinition parsing, built-in theme validation
- [ ] `src/bars/mod.rs` -- test module for git status caching, stats slot positioning

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A -- desktop app, no auth |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | Yes | serde_json validation of theme JSON files; reject malformed hex strings |
| V6 Cryptography | No | N/A |

### Known Threat Patterns for Rust + JSON file loading

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed theme JSON crashes app | Denial of Service | serde_json returns Result -- handle Err with warning log, skip file |
| Extremely large JSON theme file | Denial of Service | Check file size before parsing (reject > 1MB) |
| Theme JSON with path traversal in name field | Information Disclosure | Theme name from filename only, not from JSON content |
| Symlink in ~/.myco/themes/ pointing to sensitive file | Information Disclosure | Only read .json files, validate they parse as ThemeDefinition |

## Sources

### Primary (HIGH confidence)
- Codebase inspection: src/theme.rs (17 color fields, linear-light values, linear_to_srgb_u8 helper) [VERIFIED: direct file read]
- Codebase inspection: src/terminal/colors.rs (AnsiPalette struct, resolve_color/fg/bg functions) [VERIFIED: direct file read]
- Codebase inspection: src/terminal/renderer.rs (TerminalRenderer owns palette, hash_row uses palette) [VERIFIED: direct file read]
- Codebase inspection: src/terminal/state.rs (fetch_git_info with git2, 5-second cache TTL) [VERIFIED: direct file read]
- Codebase inspection: src/app.rs (TITLE_BAR_HEIGHT=38, build_quads pattern, sidebar_offset, recompute_layout) [VERIFIED: direct file read]
- Codebase inspection: src/sidebar/ (SidebarState, SidebarRenderer, entry_at_y, build_quads) [VERIFIED: direct file read]
- Codebase inspection: src/input/mod.rs (InputAction enum, all current variants) [VERIFIED: direct file read]
- Cargo.toml / cargo tree: All dependencies verified at listed versions [VERIFIED: cargo tree output]
- git2-rs docs (Context7 /websites/rs_git2): Repository::discover, head().shorthand(), StatusOptions [VERIFIED: Context7]

### Secondary (MEDIUM confidence)
- UI-SPEC: All four theme color definitions (Dracula, Solarized Dark/Light, Obsidian) with exact hex values [VERIFIED: 04-UI-SPEC.md]
- CONTEXT.md: All 15 locked decisions (D-01 through D-15) [VERIFIED: 04-CONTEXT.md]
- sRGB-to-linear conversion formula: Standard IEC 61966-2-1 formula [CITED: widely documented standard]

### Tertiary (LOW confidence)
- cosmic-text FontSystem font enumeration API (A6 in assumptions) -- not verified in this session

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- zero new dependencies, all verified in cargo tree
- Architecture: HIGH -- extends proven patterns (fixed regions, build_quads, AnsiPalette)
- Theme system: HIGH -- UI-SPEC provides exact hex values, sRGB math is well-established
- Settings overlay: MEDIUM -- GPU-rendered interactive forms are novel for this codebase (no prior dropdown/toggle widgets)
- Project switcher: MEDIUM -- project registry format not yet finalized (Phase 5 overlap)
- Pitfalls: HIGH -- identified from direct codebase inspection

**Research date:** 2026-05-16
**Valid until:** 2026-06-16 (stable domain -- Rust crate versions unlikely to change)
