# Myco

## What This Is

An AI-native project control surface built in Rust. Myco treats the project folder as the persistent context surface for AI-assisted work — a grid-based workspace where terminal, canvas, and document panels share a folder as the source of truth. Everything saves to the folder, everything is readable by AI agents. macOS first, Linux portable.

## Core Value

The project folder is the context surface. Sketch an idea on the canvas, it's a file. Run a command in the terminal, the output is in the folder's history. View a planning doc alongside code. AI agents read the same folder you're looking at. The folder is the memory, not the chat session.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] GPU-rendered terminal emulator (PTY, VTE, full bash/zsh support)
- [ ] Resizable grid layout with draggable, closable panels (caps)
- [ ] TLDraw canvas cap (webview-based, saves to project folder)
- [ ] Markdown viewer/editor cap (Obsidian-flavored, webview-based)
- [ ] Project sidebar with cross-project navigation and status
- [ ] Application frame: left nav bar, top stats bar, bottom project info bar
- [ ] .myco JSON config file per project (layout, theme, cap config, project metadata)
- [ ] ~/.myco global folder with project registry, preferences, aggregated stats, cross-project trends, token usage history
- [ ] Theming system (Solarized and Obsidian minimalist defaults)
- [ ] Per-cap process monitoring (CPU, RAM) with freeze capability
- [ ] macOS app signing and notarization via Apple Developer account
- [ ] Browser view cap (embedded Chromium via webview, URL-loadable from other caps)
- [ ] Table view cap (CSV/TSV rendering, performance-focused with lazy loading for large files)
- [ ] Agent monitor cap (background AI process viewer, openable as cap on demand)
- [ ] Toast notifications for human intervention needed (e.g., Claude Code awaiting input)
- [ ] Keyboard shortcuts modeled after Warp.dev (navigation, window management)
- [ ] Settings view (Cmd+, shortcut)
- [ ] Git status in bottom bar (branch, open PRs, local vs remote commits)
- [ ] Top bar configurable statistics surface (token usage, active LLMs, projects running)
- [ ] Background agentic contexts that run without an open cap
- [ ] ~/.myco git sync (auto-pull on startup if git repo detected)

### Out of Scope

- Mobile or tablet version — desktop control surface only
- Windows support in v1 — macOS first, Linux second, Windows deferred
- Built-in LLM inference — myco monitors and surfaces agents, doesn't run them
- Collaborative/multiplayer editing — single-user tool
- Plugin marketplace — modularity is architectural, not ecosystem
- Full IDE features (code completion, LSP, debugger) — the terminal cap runs your editor of choice

## Context

**Thesis**: AI tools treat projects as chat sessions. The folder/file/history structure IS the context surface and nobody builds a visual layer around that insight. .planning/, .claude/, .myco — these file-based context patterns are emerging across AI tooling. Myco is the surface that makes this visible and usable.

**Inspiration**:
- Warp.dev (open-sourced at github.com/warpdotdev/warp) — terminal architecture, block-based command model, keyboard shortcuts, settings patterns. WarpUI (MIT-licensed) and their forked VTE/font-kit deps (Apache-2.0) inform the rendering approach
- earendil-works/pi — agent-core/UI separation pattern, unified LLM abstraction, differential TUI rendering, sandboxed artifact rendering
- Obsidian — markdown rendering style, minimalist theme, file-first philosophy
- VS Code — workspace concept (project = folder with config)

**Technical research findings**:
- Warp's stack: Rust + custom GPU UI framework (WarpUI, MIT) + Metal/wgpu + forked alacritty VTE. 60+ crates, >144 FPS rendering, ~1.9ms screen redraw
- Embedding webviews in GPU-rendered Rust apps is feasible via wry (Tauri's crate). Webviews are native overlays (WKWebView on macOS), not composited into the GPU pipeline. Known keyboard focus routing issues between GPU surface and webviews
- AGPL on Warp's terminal code means clean-room approach required. alacritty_terminal (Apache-2.0/MIT) is the alternative for VTE/terminal state
- Text rendering is the biggest time sink in custom GPU rendering (font shaping, ligatures, emoji)

**User context**: Developer is a PM at LEGO based in Denmark, intermediate Rust experience, builds AI-assisted applications. TypeScript is the default stack but Rust is a deliberate strategic choice here for performance and the learning investment. Has Apple developer account for signing/distribution.

## Constraints

- **Stack**: Rust + wgpu (GPU rendering) + wry (webview embedding) + alacritty_terminal (VTE/PTY). No Electron
- **Platform**: macOS first. Architecture must support Linux portability (wgpu + wry both support Linux, but macOS-specific optimizations like Metal are acceptable)
- **Licensing**: Cannot use Warp's AGPL code (terminal, editor, core). Can use WarpUI (MIT) and forked deps (Apache-2.0). Clean-room approach for any patterns inspired by AGPL crates
- **Config format**: JSON for .myco project files and ~/.myco global config
- **Distribution**: DMG with code signing and notarization via Apple Developer account
- **Solo developer**: Architecture decisions must be realistic for one person. Prioritize shipping a usable core loop over comprehensive features
- **Folder-first**: All project state lives in the project folder (.myco file) or the global ~/.myco folder. No hidden databases, no cloud sync, no state outside these two locations

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust + wgpu over Electron | Strategic bet on performance and the Rust ecosystem. 4-5x more dev time but produces a qualitatively different product (30-40 MB memory vs 500+ MB, GPU-accelerated rendering). Aligns with Warp/Zed direction | — Pending |
| Hybrid GPU + webview architecture | Terminal and layout engine GPU-rendered for performance where it matters. TLDraw, browser, markdown via wry webviews because they're inherently web content. Pragmatic middle ground | — Pending |
| alacritty_terminal over Warp's terminal code | Warp's terminal crate is AGPL and tightly coupled. alacritty_terminal is Apache-2.0, battle-tested, and modular | — Pending |
| JSON config over TOML | Universal readability — any AI tool or script can parse JSON. Trades Rust-ecosystem convention for broader tooling compatibility | — Pending |
| Project folder as context surface (thesis) | The bet: AI-native tools should treat the folder as persistent context, not ephemeral chat. This is the product thesis to validate | — Pending |
| Open source from the start | Strategic bet on category definition. If "project as context" is a real insight, open source accelerates adoption and validation | — Pending |
| Full grid + 3 cap types for v1 | Terminal + TLDraw + Markdown in a resizable grid with app frame. Proves the thesis in a usable form. Browser, table, agent monitor are v2 | — Pending |
| Dogfood as validation strategy | Switch to myco as daily driver within 3 months. Own friction = primary feedback signal before seeking external users | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-15 after initialization*
