# Skill: new-cap

Create a new cap type for the Myco workspace grid.

## When to use

When the user wants to add a new panel content type (cap) to Myco. Examples: code editor, image viewer, table/CSV viewer, agent monitor, browser, log viewer, diff viewer.

## Prerequisites

Before creating a cap, clarify these with the user:

1. **Rendering mode**: GPU (custom quads + text via glyphon) or Webview (HTML/JS via wry)?
2. **File association**: Does this cap open/display a file type? Which extensions?
3. **Input model**: Read-only viewer, or does it capture keyboard input?
4. **Persistence**: What gets saved to disk and where? (`.myco/` subdir, project files, etc.)
5. **Creation trigger**: Keyboard shortcut? Sidebar file open? Menu action?

## Architecture

Caps follow a consistent structure. See `src/cap/mod.rs` for the trait definitions.

### Two rendering paradigms

| Mode | Trait | Examples | Produces |
|------|-------|----------|----------|
| GPU | `Cap + GpuCap` | Terminal, Markdown | `Vec<QuadInstance>` + `Vec<TextArea>` |
| Webview | `Cap + WebviewCap` | Canvas (TLDraw) | Positioned `wry::WebView` child window |

### Module structure

```
src/{cap_name}/
├���─ mod.rs          # Manager struct + public API
├── state.rs        # Per-instance state struct
├── renderer.rs     # (GPU only) Quad/text generation
└── assets.rs       # (Webview only) Bundled HTML/JS/CSS
```

### Integration surfaces (checklist)

Every new cap touches these files:

1. `src/grid/panel.rs` — Add variant to `PanelType` enum, `Display` impl, and `Panel::new_*()` factory
2. `src/{cap_name}/mod.rs` — Manager struct with `HashMap<PanelId, State>`
3. `src/app.rs` — Manager field, creation in `InputAction` handler, destruction in `PanelClose`
4. `src/input/mod.rs` — New `InputAction` variants for cap-specific actions
5. `src/input/keyboard.rs` — Keyboard routing (if `captures_keyboard`)
6. `src/input/mouse.rs` — Scroll dispatch in `handle_scroll_event`
7. `src/app.rs` render section — Quad/text or webview positioning

## Step-by-step procedure

### 1. Define the type

Add to `src/grid/panel.rs`:

```rust
// In PanelType enum:
/// {CapName} panel -- {one-line description}.
{CapName},

// In Display impl:
PanelType::{CapName} => write!(f, "{CapName}"),

// Factory method on Panel:
pub fn new_{cap_name}(id: PanelId, /* cap-specific args */) -> Self {
    Self {
        id,
        panel_type: PanelType::{CapName},
        title: /* derive from args */,
        file_path: /* if file-based */,
        canvas_id: None, // or use a generic metadata field
    }
}
```

### 2. Create the module

Create `src/{cap_name}/mod.rs`:

```rust
use std::collections::HashMap;
use crate::grid::PanelId;

pub mod state;
// pub mod renderer;  // GPU caps
// pub mod assets;    // Webview caps

pub use state::{CapName}State;

pub struct {CapName}Manager {
    states: HashMap<PanelId, {CapName}State>,
    // project_dir, config, etc. as needed
}

impl {CapName}Manager {
    pub fn new(/* shared deps */) -> Self { ... }

    pub fn create_{cap_name}(
        &mut self,
        panel_id: PanelId,
        /* cap-specific args */
    ) -> Result<(), Box<dyn std::error::Error>> { ... }

    pub fn destroy_{cap_name}(&mut self, panel_id: &PanelId) {
        if self.states.remove(panel_id).is_some() {
            tracing::debug!("Destroyed {cap_name} for panel {:?}", panel_id);
        }
    }

    pub fn get(&self, panel_id: &PanelId) -> Option<&{CapName}State> {
        self.states.get(panel_id)
    }

    pub fn get_mut(&mut self, panel_id: &PanelId) -> Option<&mut {CapName}State> {
        self.states.get_mut(panel_id)
    }
}
```

### 3. Wire into app.rs

```rust
// Field in App struct:
{cap_name}_manager: Option<{CapName}Manager>,

// In App::new():
{cap_name}_manager: Some({CapName}Manager::new(/* deps */)),

// In PanelClose handler (around line 313):
if let Some(mgr) = &mut self.{cap_name}_manager {
    mgr.destroy_{cap_name}(&panel_id);
}

// New InputAction handler for creation:
InputAction::Create{CapName} => { /* split + create */ }
```

### 4. Input routing

In `src/input/mouse.rs` `handle_scroll_event`:
```rust
Some(PanelType::{CapName}) => {
    // Pixel or line scroll depending on cap needs
    actions.push(InputAction::{CapName}Scroll { panel_id, delta });
}
```

In `src/input/keyboard.rs` (if cap captures keyboard):
```rust
if panel_type == Some(PanelType::{CapName}) {
    // Route key events to cap-specific InputActions
}
```

### 5. Rendering

**GPU cap** — add to the render loop in `app.rs` (around line 1470):
```rust
if panel.panel_type == PanelType::{CapName} {
    if let Some(mgr) = &self.{cap_name}_manager {
        if let Some(state) = mgr.get(&panel_id) {
            let quads = self.{cap_name}_renderer.build_quads(/* ... */);
            quads.extend(cap_quads);
        }
    }
}
```

**Webview cap** — follow the Canvas pattern: `build_as_child()` with `set_bounds()` on resize.

### 6. Register the module

In `src/main.rs` (or wherever modules are declared):
```rust
mod {cap_name};
```

## Design rules

- **No shared mutable state between caps.** Each cap instance is isolated behind its PanelId.
- **Managers own all instances.** The App doesn't hold direct references to cap state.
- **Caps don't know about each other.** Cross-cap communication goes through InputActions.
- **File operations are bounded.** Max file sizes, no arbitrary path traversal. Follow canvas's 50MB limit pattern.
- **Webview caps block external navigation.** Use `with_navigation_handler` to restrict to custom protocol.
- **GPU caps produce quads, not draw calls.** All rendering goes through the shared QuadRenderer + glyphon pipeline.

## Existing caps as reference

| Cap | Rendering | Complexity | Good reference for |
|-----|-----------|------------|-------------------|
| Markdown | GPU | Low | Read-only GPU viewer with file watching and scroll |
| Canvas | Webview | Medium | Webview lifecycle, IPC, auto-save, focus management |
| Terminal | GPU | High | Keyboard capture, PTY lifecycle, rich state machine |

**Start with Markdown as your template** for new GPU caps. Start with Canvas for new webview caps.

## Future: trait migration

The `src/cap/mod.rs` trait definitions (`Cap`, `GpuCap`, `WebviewCap`) define the target interface. As caps accumulate, we will migrate managers to implement these traits and replace the per-type `if` chains in `app.rs` with trait-object dispatch. This is not required for the next ~5 caps but becomes necessary around 10+.

Migration order: Markdown first (simplest), then Canvas, then Terminal (most complex).
