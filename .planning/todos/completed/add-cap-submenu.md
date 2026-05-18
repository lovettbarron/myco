---
name: add-cap-submenu
type: feature
priority: low
source: conversation
created: 2026-05-18
---

## Add Cap Submenu in File Menu

File menu currently only has "New Terminal" and "New Canvas" as top-level items. Other panel types (Markdown, Agent Monitor) are only reachable programmatically or via keyboard shortcuts.

**Proposal:** Add a `File > Add Cap >` submenu listing all available panel types:
- Terminal
- Canvas
- Markdown
- Agent Monitor

This makes all cap types discoverable from the menu bar. Small scope — involves `src/platform/menu.rs` (or wherever the native menu is built) and wiring each item to the existing `Panel::new_*` constructors.
