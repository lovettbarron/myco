---
slug: add-cap-submenu
description: Add Cap submenu to File menu listing all panel types
status: executing
created: 2026-05-18
---

# Feature: Add Cap Submenu

## Goal
Add a "Add Cap" submenu under the File menu listing available panel types.

## Tasks

### Task 1: Add submenu support to MenuItemDef
- **File:** `src/platform/menu.rs`
- **Change:** Add optional `items` field to `MenuItemDef` for nested submenus

### Task 2: Update menu builder to handle submenus
- **File:** `src/platform/menu.rs`
- **Change:** When `items` is present, create an NSMenu submenu and attach it

### Task 3: Update menu.json
- **File:** `resources/menu.json`
- **Change:** Replace flat "New Terminal" / "New Canvas" with "Add Cap" submenu containing:
  - Terminal (Cmd+T)
  - Canvas (Cmd+Shift+T)
  - Agent Monitor (no shortcut — singleton)
- **Note:** Markdown omitted — requires file path, opened from sidebar

### Task 4: Wire "create_agent_monitor" action in handle_menu_action
- **File:** `src/app.rs`
- **Change:** Map "create_agent_monitor" to `InputAction::OpenAgentMonitor`

### Task 5: Build and test
