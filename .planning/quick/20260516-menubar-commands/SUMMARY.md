---
slug: menubar-commands
status: complete
created: 2026-05-16
completed: 2026-05-16
---

# Summary: Native macOS Menu Bar

## What was done
Built a config-driven native macOS menu bar using objc2 NSMenu/NSMenuItem APIs. All existing keyboard shortcuts are surfaced in the menu, and toggle items (like sidebar visibility) update their labels dynamically.

## Files created
- `resources/menu.json` — Menu configuration (menus, items, actions, shortcuts, toggles)
- `src/platform/menu.rs` — Menu building, Objective-C action handler, toggle label updates

## Files modified
- `Cargo.toml` — Added NSMenu, NSMenuItem, NSApplication, NSEvent features to objc2-app-kit
- `src/platform/mod.rs` — Added `pub mod menu`
- `src/app.rs` — Added `UserEvent::MenuAction`, `menu_state` field, `handle_menu_action()`, `update_menu_toggles()`, wiring in `resumed()` and `user_event()`
- `src/input/mod.rs` — (from prior task) InitPrompt actions

## Architecture
- **Config-driven**: `resources/menu.json` defines all menus. Add/edit items by editing JSON.
- **Action routing**: Menu items use NSMenuItem tags -> `UserEvent::MenuAction(tag)` -> `handle_menu_action()` resolves action name to `InputAction`
- **Dynamic labels**: Toggle items (e.g., "Show/Hide File Browser") update via `update_menu_toggles()` called after state changes
- **Keyboard shortcuts**: Set via NSMenuItem key equivalents with modifier masks — displayed automatically by macOS
- **Custom ObjC class**: `MenuActionHandler` defined via `define_class!` receives selector calls, posts events through winit's `EventLoopProxy` via thread-local storage

## Menu structure
- **Myco**: About, Quit (Cmd+Q)
- **File**: New Terminal (Cmd+T), New Canvas (Cmd+Shift+T), Initialize Project, Close Tab (Cmd+W)
- **Edit**: Copy (Cmd+C), Paste (Cmd+V), Find (Cmd+F)
- **View**: Show/Hide File Browser (Cmd+B), Font Size +/- (Cmd+=, Cmd+-)
- **Tab**: Split Right (Cmd+D), Split Down (Cmd+Shift+D), Next/Prev Tab (Cmd+]/[), Toggle Fullscreen (Esc)
- **Window**: Minimize (Cmd+M), Zoom
