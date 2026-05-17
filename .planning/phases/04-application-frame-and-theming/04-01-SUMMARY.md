---
phase: 04-application-frame-and-theming
plan: 01
subsystem: theme
tags: [theme, color-system, registry, json-loader]
dependency_graph:
  requires: []
  provides: [ThemeDefinition, ThemeRegistry, ThemeSwitch-action, theme-module]
  affects: [src/app.rs, src/terminal/renderer.rs, src/input/mod.rs]
tech_stack:
  added: []
  patterns: [data-driven-theme, registry-pattern, hex-to-linear-conversion]
key_files:
  created:
    - src/theme/mod.rs
    - src/theme/definition.rs
    - src/theme/builtin.rs
    - src/theme/colors.rs
    - src/theme/loader.rs
  modified:
    - src/app.rs
    - src/input/mod.rs
    - src/terminal/renderer.rs
  deleted:
    - src/theme.rs
decisions:
  - "Theme module refactored from single file to directory module with 5 submodules"
  - "buffer_cache field (not panel_caches) used for invalidation -- matches existing code naming"
  - "Two new Theme fields added (markdown_table_header_bg, markdown_table_border) for future table rendering"
metrics:
  duration: 5 min
  completed: "2026-05-17T05:09:23Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 8
  tests_added: 8
  tests_total_passing: 68
---

# Phase 04 Plan 01: Theme System Refactoring Summary

Data-driven theme architecture with four built-in themes, custom JSON loading from ~/.myco/themes/, and live theme switching that updates GPU + terminal colors simultaneously.

## Task Completion

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Create theme module with definition structs, builtins, and registry | 6d19ace | src/theme/{mod,definition,builtin,colors,loader}.rs |
| 2 | Wire theme switching into App with ThemeRegistry and InputAction::ThemeSwitch | b9db319 | src/app.rs, src/input/mod.rs, src/terminal/renderer.rs |

## What Was Built

1. **Theme module** (`src/theme/`): Replaced monolithic `src/theme.rs` with a 5-file directory module:
   - `colors.rs`: sRGB/linear conversion utilities (hex_to_linear, srgb_to_linear, linear_to_srgb_u8, hex_to_srgb_u8)
   - `definition.rs`: Serde-enabled ThemeDefinition, ThemeBase, ThemeAnsi structs
   - `builtin.rs`: Four built-in themes (Dracula, Solarized Dark, Solarized Light, Obsidian) with exact UI-SPEC hex values
   - `loader.rs`: JSON loader for ~/.myco/themes/ with size guards (1MB limit), extension filtering, filename-as-display-name
   - `mod.rs`: Theme struct (19 fields), from_definition derivation, ThemeRegistry, to_ansi_palette

2. **Theme switching** (`InputAction::ThemeSwitch`): New action variant that:
   - Updates active theme in ThemeRegistry
   - Replaces the app-wide Theme struct (all GPU colors)
   - Replaces the terminal AnsiPalette (16 ANSI colors + fg/bg)
   - Invalidates all terminal buffer caches (hashes stale after palette change)
   - Requests a full window redraw

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Corrected cache field name**
- **Found during:** Task 2
- **Issue:** Plan referenced `panel_caches` field but actual field is `buffer_cache`
- **Fix:** Used `buffer_cache.clear()` in `invalidate_all_caches()`
- **Files modified:** src/terminal/renderer.rs

## Key Technical Decisions

- **Theme::from_definition derivation table**: All 19 Theme fields are derived from 10 semantic base colors + an overrides map, enabling any theme to be defined with minimal data
- **Darkened code block background**: Computed by multiplying sRGB channels by 0.85 before linear conversion (not darkening in linear space)
- **Filename as display name (T-04-03)**: Custom theme display names use the JSON filename stem, ignoring the `name` field in JSON content, preventing path traversal in display contexts

## Verification Results

- `cargo build`: Clean compile (17 warnings, all unused-function -- consumed by future plans)
- `cargo test`: 68/68 tests passing (8 new theme tests + 60 existing)
- Theme unit tests cover: hex conversion roundtrip, builtin validation, JSON serde roundtrip, AnsiPalette conversion, registry operations, error handling

## Self-Check: PASSED
