---
slug: add-cap-submenu
status: complete
completed: 2026-05-18
---

# Summary: Add Cap Submenu

Added "Add Cap" submenu to the File menu with all available panel types:
- Terminal (Cmd+T)
- Canvas (Cmd+Shift+T)
- Agent Monitor

## Changes
- `src/platform/menu.rs` — Added `items` field to `MenuItemDef` for nested submenus; extracted `build_menu_items()` helper with recursion support
- `resources/menu.json` — Replaced flat "New Terminal"/"New Canvas" with "Add Cap" submenu
- `src/app.rs` — Wired `create_agent_monitor` action to `InputAction::OpenAgentMonitor`

## Decisions
- Markdown omitted from submenu — requires a file path, opened from sidebar
- Keyboard shortcuts preserved on submenu items (Cmd+T for Terminal, Cmd+Shift+T for Canvas)
- Agent Monitor has no shortcut (singleton — focuses existing if one is open)
