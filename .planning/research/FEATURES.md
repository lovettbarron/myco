# Feature Research

**Domain:** AI-native terminal/workspace application (developer tool)
**Researched:** 2026-05-15
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features every terminal emulator must have. Missing any of these and developers will not switch from their current tool. These are non-negotiable for a product that includes a terminal emulator.

#### Core Terminal Rendering

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Full VTE/PTY support (bash, zsh, fish) | Any shell must work correctly; broken shell = unusable product | HIGH | alacritty_terminal (Apache-2.0) handles VTE parsing and PTY management. Battle-tested by Alacritty and Zed. Text rendering (shaping, ligatures, emoji) is the single biggest time sink in custom GPU rendering per PROJECT.md |
| True color (24-bit) | 92% of modern terminals support this; absence is immediately visible in vim/neovim themes, bat output, etc. | LOW | wgpu pipeline must support 24-bit color output. Standard in GPU-rendered terminals |
| Unicode and emoji rendering | 97% of terminals handle Unicode correctly; broken rendering in git logs, filenames, or comments is jarring | MEDIUM | Requires proper glyph shaping (harfbuzz or equivalent), correct width calculation for CJK, combining characters, and emoji presentation selectors |
| Scrollback buffer | Users scroll up to review command output constantly. Alacritty has configurable scrollback up to 100K lines | LOW | alacritty_terminal provides scrollback. Must be configurable (default ~10K lines) |
| In-terminal search (Ctrl+Shift+F) | Finding text in scrollback is a daily workflow. Every competitor supports it | MEDIUM | Search within scrollback buffer with highlighting. Vi-mode search (forward/backward) is the standard pattern |
| Keyboard shortcuts (copy, paste, clear, navigation) | Platform-native Cmd+C/V on macOS, standard terminal keybinds | LOW | Map to macOS conventions. PROJECT.md specifies Warp-style shortcuts as inspiration |
| Configurable font and font size | Developers are particular about their terminal font. Must support popular dev fonts (JetBrains Mono, Fira Code, etc.) | LOW | Font selection in settings, runtime font size adjustment (Cmd+/Cmd-) |
| Cursor styles (block, beam, underline) | Every terminal offers this. Shell tools (vi-mode, etc.) depend on cursor shape switching | LOW | VTE sequences for cursor shape change. Standard escape codes |

#### Window and Layout Management

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Split panes (horizontal/vertical) | 53% of terminals support native splits; the rest delegate to tmux. For a workspace product, native splits are mandatory | HIGH | Myco's grid layout system IS the split pane system. Each terminal is a "cap" in the resizable grid. This is already planned in PROJECT.md |
| Multiple terminal instances | Opening several terminals in one window is basic workflow (editor in one, server in another, git in a third) | MEDIUM | Each terminal cap is an independent PTY session. Grid can hold N terminal caps simultaneously |
| Resizable panels | Users must be able to drag-resize their terminal panes | MEDIUM | Already planned as core grid feature. Must feel native -- smooth, responsive, snap-to-grid optional |

#### Shell Integration

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Working directory tracking | Terminal needs to know the current directory for features like "open new tab here" or project-scoped behavior | MEDIUM | OSC 7 (working directory reporting) is the standard. Shell integration scripts inject escape sequences. 39% of terminals support shell integration per Terminal Trove data |
| Clickable URLs/file paths | Cmd+Click on URLs and file paths is expected in modern terminals (iTerm2, Warp, Ghostty, Zed all do it) | MEDIUM | Regex detection of URLs and file paths in terminal output. Open in browser cap or system browser |
| Hyperlinks (OSC 8) | 68% of terminals support OSC 8 clickable hyperlinks. Tools like `ls --hyperlink` and `gh` emit these | LOW | Standard escape sequence parsing in VTE layer |

#### Visual and Theming

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Color scheme / theming | Developers expect to customize terminal colors. Dark mode is the default expectation | MEDIUM | PROJECT.md specifies Solarized and Obsidian minimalist defaults. Must support standard 16-color ANSI palette + configurable theme |
| Font ligatures | 34% of terminals support ligatures (Kitty, Ghostty, WezTerm, Warp). Expected by developers using Fira Code, JetBrains Mono | HIGH | Requires proper font shaping pipeline with harfbuzz. Significant rendering complexity. Could defer to v1.x if text rendering timeline is tight |
| Selection and copy behavior | Text selection, rectangular selection, and proper copy-paste are fundamental | MEDIUM | Mouse-based selection in GPU-rendered surface. Must handle line wrapping correctly |

#### Configuration and Persistence

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Persistent configuration file | Every terminal emulator has a config file. Users expect their settings to survive restarts | LOW | Already planned: .myco JSON config per project, ~/.myco global config |
| Session persistence (layout memory) | When reopening a project, the layout should restore. VS Code, Zed, and Beam all do this | MEDIUM | Save panel layout to .myco project config. Restore on project open. Beam specifically highlights this as a key feature |

### Differentiators (Competitive Advantage)

These features set Myco apart from terminal emulators and from AI-native IDEs. They align with the core thesis: the project folder is the context surface.

#### Folder-as-Context-Surface (Core Thesis)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Project folder as the AI context surface | **The core thesis.** No competitor treats the folder as persistent AI memory. Cursor has .cursorrules, Claude Code has CLAUDE.md, but nobody builds a visual layer around the insight that .planning/, .claude/, .myco are the same context surface. Warp treats the terminal session as context; Myco treats the folder | HIGH | .myco config, file-watching for context changes, surfacing folder state in the UI. This is the product bet -- everything else supports this |
| Cross-project dashboard (top bar stats) | No terminal shows aggregated stats across projects. Warp shows per-session agent status; Myco can show token usage, active LLMs, and project health across all projects from the top bar | MEDIUM | ~/.myco global folder with project registry, aggregated stats, cross-project trends. Already in PROJECT.md requirements |
| Project sidebar with status | Quick-switch between projects with live status indicators (agents running, git status, last activity). Beam does workspace switching but without status richness | MEDIUM | ~/.myco project registry powers the sidebar. Each project has a .myco config file with metadata |

#### Hybrid Workspace (Terminal + Canvas + Docs)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| TLDraw canvas cap (saves to folder) | **No terminal emulator or IDE offers an embedded drawing canvas.** Sketch architecture, plan features, diagram flows -- all saved as files in the project folder, readable by AI agents. Zed and Cursor have no drawing surface. VS Code has Excalidraw as a plugin but it is not a first-class citizen | HIGH | Webview-based via wry. TLDraw SDK is React/TypeScript. Saves .tldr files to project folder. Key focus routing challenge between GPU surface and webview (documented in PROJECT.md) |
| Markdown viewer/editor cap | View planning docs (.planning/*.md, CLAUDE.md, README) alongside your terminal. Obsidian-flavored rendering. No context-switching to a separate app | MEDIUM | Webview-based via wry. Render markdown with Obsidian-style formatting. File-watching for live updates. Many markdown renderers available for web |
| Browser view cap | Load URLs, preview local dev servers, view documentation -- all within the workspace grid. Warp has no browser; Zed has no browser; VS Code has Simple Browser but it is clunky | MEDIUM | Embedded Chromium via wry/webview. URL-loadable from other caps. PROJECT.md marks this as v2 |
| Table view cap (CSV/TSV) | View data files directly in the workspace. Useful for ML projects, data analysis, config review | MEDIUM | Lazy-loaded rendering for large files. PROJECT.md marks this as v2 |
| Resizable grid layout (caps) | Other tools use fixed dock positions (Zed: terminal at bottom/left/right). Myco's grid is fully flexible -- any cap in any position, user-arranged. Closer to a tiling window manager philosophy applied to workspace panels | HIGH | Core UI architecture. GPU-rendered grid with draggable, closable panels. Already the planned architecture |

#### AI-Native Monitoring (Not Running, Monitoring)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Agent monitor cap | **Myco watches AI agents; it does not run them.** See what Claude Code, Codex, Gemini CLI are doing across your terminal caps from a single monitoring view. Warp runs its own agent (Oz) but treats third-party agents as terminal processes. Beam organizes sessions but has no monitoring. This is a unique layer | HIGH | Must detect agent processes in PTY sessions, parse their output for status signals, surface "waiting for input" / "running" / "error" states. Toast notifications for human intervention needed. PROJECT.md already specifies this |
| Background agentic contexts | Run AI agent sessions that do not require an open terminal cap. Monitor from the agent dashboard. No competitor offers this -- Warp requires a visible pane; Claude Code's background agents require a terminal | HIGH | Background PTY sessions managed by Myco, visible in agent monitor. PROJECT.md specifies this |
| Toast notifications for agent intervention | When Claude Code asks "Do you want to proceed?" or an agent hits an error, Myco surfaces a toast notification so you can respond. Warp has this for its own Oz agent; Myco generalizes it to any agent | MEDIUM | Parse terminal output for known intervention patterns (Claude Code's permission prompts, etc.). System notification integration on macOS |
| Per-cap process monitoring (CPU, RAM) | See resource usage per terminal cap / agent. No terminal emulator shows per-PTY resource metrics. iTerm2 has a status bar with system stats but not per-session | MEDIUM | Read process tree from PTY child PID, aggregate CPU/memory. Display in cap header or status area. Freeze capability for runaway processes (already in PROJECT.md) |
| Token usage and cost tracking (top bar) | Surface LLM token usage and costs across sessions. Developers using Claude Code, Cursor, etc. have no unified view of their AI spending. Myco's top bar can aggregate this from agent output parsing or log files | HIGH | Parse agent output or read from ~/.claude/ cost logs, aggregate across projects. Display in configurable top bar. Novel -- no existing tool does this at the terminal/workspace level |

#### Developer Workflow

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Git status in bottom bar | Branch, dirty state, commits ahead/behind. iTerm2 has this as a status bar component; Zed shows it in the editor status bar. For a workspace tool, it is table-stakes-adjacent but differentiating for a terminal-first tool | LOW | Run git commands on working directory, parse output, display in bottom bar. Already in PROJECT.md |
| Block-based command model | Warp's signature feature: each command+output is a discrete, navigable block. Greatly improves terminal UX for reviewing output, copying results, sharing. No other terminal has adopted this | HIGH | Requires shell integration to detect command boundaries (OSC 133 semantic prompts). Significant VTE layer work. HIGH value but HIGH cost for a solo developer. Consider as v2 |
| Shell integration (OSC 133 semantic prompts) | Detect command start/end, exit codes, duration. Enables prompt-to-prompt navigation, command duration display, and feeds into the block model. Ghostty, WezTerm, iTerm2, Warp all support this | MEDIUM | Inject shell integration scripts for bash/zsh/fish. Parse OSC 133 sequences in VTE layer. Foundation for many higher-level features |

### Anti-Features (Deliberately NOT Building)

Features that seem appealing but conflict with Myco's thesis, add unsustainable scope, or create the wrong product.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Built-in LLM inference / own AI agent | Every AI tool builds their own agent (Warp has Oz, Cursor has its agent, Zed has its agent). Seems necessary to compete | **Myco monitors agents; it does not run them.** Building an AI agent is a full product in itself. It would require cloud infrastructure, model access management, billing, and ongoing model evaluation. A solo developer cannot compete with Anthropic, OpenAI, etc. on agent quality. The value is in the orchestration layer, not the agent | Monitor Claude Code, Codex, Gemini CLI, and any future agent. Be the best surface for seeing what agents are doing, not another agent |
| Full IDE features (LSP, code completion, debugger) | Developers want everything in one place. Cursor and Zed prove IDE+AI is viable | **Myco is a workspace, not an IDE.** LSP integration alone is months of work. Code completion competes with nvim/Zed/Cursor where developers already have preferences. The terminal cap runs your editor of choice (nvim, helix, etc.) | Terminal cap runs your preferred editor. Myco provides the surrounding workspace context (canvas, docs, monitoring) that editors lack |
| Plugin/extension marketplace | VS Code's marketplace is its moat. Extensibility seems essential | **Architectural modularity without ecosystem overhead.** A marketplace requires review processes, security scanning, versioning, hosting, and developer relations. For a solo dev this is years of distraction. Caps are modular by architecture -- new cap types can be added | Add new cap types over time. Community can contribute caps via PRs to the open-source repo. No runtime plugin loading, no marketplace |
| Real-time collaboration / multiplayer | Cursor, Zed, and tldraw all support multiplayer. Seems like table stakes for modern tools | **Myco is a single-user control surface.** Multiplayer requires CRDT/OT infrastructure, server infrastructure, account management, and fundamentally changes the architecture. The thesis is personal project context, not shared editing | Share project context via the folder (git). Collaborate through standard dev workflows (PRs, shared docs). TLDraw files in the folder can be opened in tldraw.com for collaborative sessions if needed |
| Cloud sync for settings/state | Warp Drive syncs commands and workflows via the cloud. Feels modern | **Folder-first philosophy.** Cloud sync requires servers, accounts, auth, and privacy considerations. The ~/.myco folder IS the sync mechanism -- put it in a git repo and it syncs. Already planned in PROJECT.md (git sync on startup) | ~/.myco git sync (already planned). Zero infrastructure, user controls their data |
| Built-in image display protocol (Sixel/Kitty graphics) | 48% of terminals support inline images. Kitty protocol is increasingly popular | **Extremely high complexity for GPU-rendered terminal.** Compositing images into the wgpu text rendering pipeline requires significant work. Only 6 terminals support Kitty graphics. For Myco's use case (AI workspace), the browser cap handles rich content | Browser cap and TLDraw canvas handle visual content. Inline terminal images can be a v3 feature if demand warrants it |
| Windows support in v1 | Windows has the largest developer population. Seems like leaving market share on the table | **macOS-first reduces surface area dramatically.** Metal-specific optimizations, wry/WebKit on macOS, Apple Developer signing -- all these are macOS patterns. Linux portability is achievable via wgpu abstraction. Windows adds WinRT, ConPTY, and entirely different webview (WebView2) complexity | macOS first, Linux second (wgpu + wry support both). Windows deferred until the product thesis is validated |
| Tmux/terminal multiplexer integration | Power users run tmux inside terminals. Some terminals (WezTerm) have built-in multiplexers | **Myco's grid IS the multiplexer.** Supporting tmux-inside-Myco creates confusing UX (nested splits, conflicting keybinds). Myco's native grid layout replaces tmux's use case | Native grid layout with resizable caps. Users who need tmux for remote sessions can still use it inside a terminal cap, but Myco should not try to integrate with or replace tmux |

## Feature Dependencies

```
[VTE/PTY Support]
    +--requires--> [Scrollback Buffer]
    |                  +--enables--> [In-terminal Search]
    +--requires--> [Unicode Rendering]
    +--requires--> [True Color Support]
    +--enables---> [Shell Integration (OSC 133)]
    |                  +--enables--> [Command Block Detection]
    |                  |                 +--enables--> [Block-based UI] (v2)
    |                  +--enables--> [Working Directory Tracking]
    |                  |                 +--enables--> [Git Status in Bottom Bar]
    |                  +--enables--> [Agent Process Detection]
    |                                    +--enables--> [Agent Monitor Cap]
    |                                    +--enables--> [Toast Notifications]
    +--enables---> [Per-cap Process Monitoring]

[GPU Grid Layout]
    +--requires--> [Resizable Panels]
    +--requires--> [Panel Create/Close/Drag]
    +--enables---> [Multiple Terminal Caps]
    +--enables---> [Webview Caps (via wry)]
    |                  +--enables--> [TLDraw Canvas Cap]
    |                  +--enables--> [Markdown Viewer Cap]
    |                  +--enables--> [Browser View Cap] (v2)
    |                  +--enables--> [Table View Cap] (v2)
    +--enables---> [Per-cap Process Monitoring]

[.myco Project Config]
    +--enables---> [Layout Persistence]
    +--enables---> [Project Sidebar]
    +--enables---> [Theme Configuration]

[~/.myco Global Config]
    +--enables---> [Project Registry]
    |                  +--enables--> [Project Sidebar Navigation]
    |                  +--enables--> [Cross-project Dashboard]
    +--enables---> [Global Preferences]
    +--enables---> [Token Usage History]
    +--enables---> [Git Sync for Settings]

[Shell Integration] --conflicts-with--> [Block-based UI complexity in v1]
    (Shell integration is feasible in v1; full block UI requires much more work)

[Background Agent Contexts] --requires--> [Agent Monitor Cap]
    (Without monitoring, background agents are invisible and unmanageable)
```

### Dependency Notes

- **VTE/PTY is the foundation:** Nearly every feature depends on a working terminal. This must be rock-solid before anything else.
- **GPU Grid Layout is the second foundation:** The hybrid workspace thesis depends on the grid being flexible, performant, and stable.
- **Shell Integration unlocks the AI layer:** OSC 133 command detection is the bridge from "terminal emulator" to "AI-aware workspace." It enables agent detection, command timing, and eventually block-based UI.
- **Agent Monitor requires Shell Integration:** Detecting which process in a PTY is an AI agent, and whether it needs attention, requires parsing terminal output with awareness of command boundaries.
- **Background Agent Contexts require Agent Monitor:** Running agents without a visible cap only makes sense if there is a monitoring interface to check on them.
- **Webview caps are independent of terminal:** TLDraw and Markdown caps can be developed in parallel with terminal work since they use wry webviews, not the VTE pipeline.
- **Block-based UI conflicts with v1 timeline:** While shell integration is achievable, the full block-based command model (Warp's signature feature) is a massive undertaking that should be deferred.

## MVP Definition

### Launch With (v1)

Minimum viable product that validates the thesis: "the project folder is the context surface, and a workspace should make that visible."

- [ ] **GPU-rendered terminal cap** (VTE/PTY via alacritty_terminal, scrollback, search, true color, Unicode) -- the product is unusable without a working terminal
- [ ] **Resizable grid layout** with draggable, closable panels -- the workspace metaphor requires flexible layout
- [ ] **TLDraw canvas cap** (webview, saves .tldr to project folder) -- proves the hybrid workspace thesis: sketch + terminal in one window
- [ ] **Markdown viewer cap** (webview, renders .md files from project folder) -- view planning docs alongside work
- [ ] **Application frame** (left nav, top bar, bottom bar) -- chrome that holds the workspace together
- [ ] **.myco project config** (layout, theme, project metadata in JSON) -- folder-first persistence
- [ ] **~/.myco global config** (project registry, preferences) -- enables multi-project awareness
- [ ] **Project sidebar** with cross-project navigation -- switch between projects without leaving Myco
- [ ] **Theming** (Solarized + Obsidian defaults) -- developers will not use an ugly terminal
- [ ] **Basic keyboard shortcuts** (Warp-inspired navigation, window management) -- keyboard-driven workflow
- [ ] **macOS app signing and notarization** -- required for distribution

### Add After Validation (v1.x)

Features to add once the core loop is working and daily-drivable.

- [ ] **Shell integration (OSC 133)** -- enables command detection, working directory tracking, duration display. Foundation for AI features. Trigger: once terminal cap is stable
- [ ] **Git status in bottom bar** -- branch, dirty state, ahead/behind. Trigger: once shell integration provides working directory
- [ ] **Agent monitor cap** -- detect AI agents in terminal caps, show status. Trigger: once shell integration can detect command boundaries
- [ ] **Toast notifications for agent intervention** -- parse agent output for intervention prompts. Trigger: once agent monitor works
- [ ] **Per-cap process monitoring** (CPU, RAM, freeze) -- resource visibility per panel. Trigger: when running multiple agents simultaneously
- [ ] **Font ligatures** -- requires harfbuzz in the rendering pipeline. Trigger: when text rendering is stable and performant
- [ ] **Browser view cap** -- embedded web browser in grid. Trigger: after webview cap pattern is proven with TLDraw/Markdown
- [ ] **Table view cap** -- CSV/TSV rendering. Trigger: when building data-heavy projects

### Future Consideration (v2+)

Features to defer until the thesis is validated and the product has daily users.

- [ ] **Block-based command model** -- Warp's signature UX. Extremely high complexity but transformative for terminal usability. Defer because: requires deep VTE integration, custom input handling, and fundamentally changes how the terminal works
- [ ] **Background agentic contexts** -- PTY sessions without visible caps. Defer because: requires agent monitor to be solid first, and the monitoring UX needs real-world iteration
- [ ] **Token usage tracking and cross-project dashboard** -- aggregate LLM costs across projects. Defer because: parsing agent output for cost data varies per tool and changes frequently
- [ ] **Configurable top bar statistics** -- runtime stats surface. Defer because: depends on data sources that need v1.x features (agent monitor, token tracking)
- [ ] **~/.myco git sync** -- auto-pull on startup. Defer because: git automation has edge cases (merge conflicts, auth) that distract from core product
- [ ] **Inline image protocol support** -- Sixel/Kitty graphics in terminal. Defer because: compositing images in GPU pipeline is extremely complex
- [ ] **Linux support** -- wgpu and wry both support Linux, but testing, packaging (.deb, .AppImage), and platform-specific bugs add significant maintenance. Defer until macOS version is stable

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| GPU terminal (VTE/PTY, scrollback, search) | HIGH | HIGH | P1 |
| Resizable grid layout | HIGH | HIGH | P1 |
| TLDraw canvas cap | HIGH | MEDIUM | P1 |
| Markdown viewer cap | MEDIUM | LOW | P1 |
| App frame (nav, top bar, bottom bar) | HIGH | MEDIUM | P1 |
| .myco project config | HIGH | LOW | P1 |
| ~/.myco global config + project registry | MEDIUM | LOW | P1 |
| Project sidebar | MEDIUM | MEDIUM | P1 |
| Theming (Solarized, Obsidian) | MEDIUM | LOW | P1 |
| Keyboard shortcuts | HIGH | LOW | P1 |
| macOS signing/notarization | HIGH | LOW | P1 |
| Shell integration (OSC 133) | HIGH | MEDIUM | P2 |
| Git status in bottom bar | MEDIUM | LOW | P2 |
| Agent monitor cap | HIGH | HIGH | P2 |
| Toast notifications (agent) | HIGH | MEDIUM | P2 |
| Per-cap process monitoring | MEDIUM | MEDIUM | P2 |
| Font ligatures | LOW | HIGH | P2 |
| Browser view cap | MEDIUM | LOW | P2 |
| Table view cap | LOW | MEDIUM | P2 |
| Block-based command model | HIGH | VERY HIGH | P3 |
| Background agentic contexts | HIGH | HIGH | P3 |
| Token usage tracking | MEDIUM | HIGH | P3 |
| Top bar configurable stats | MEDIUM | MEDIUM | P3 |
| ~/.myco git sync | LOW | MEDIUM | P3 |
| Inline image protocol | LOW | VERY HIGH | P3 |
| Linux support | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for launch -- validates the workspace thesis
- P2: Should have, add when daily-driving reveals the need
- P3: Nice to have, defer until thesis is validated with real users

## Competitor Feature Analysis

| Feature | Warp | Zed | VS Code | iTerm2 | Ghostty | Kitty | Alacritty | Beam | Myco Approach |
|---------|------|-----|---------|--------|---------|-------|-----------|------|---------------|
| GPU rendering | Yes (Metal/wgpu) | Yes (GPUI) | No (Electron) | No (Cocoa) | Yes (Metal/OpenGL) | Yes (OpenGL) | Yes (OpenGL) | No | Yes (wgpu) |
| Terminal emulator | Yes | Yes (alacritty_terminal) | Yes (xterm.js) | Yes | Yes | Yes | Yes | Yes (shell passthrough) | Yes (alacritty_terminal) |
| Split panes | Yes | Yes (dock positions) | Yes (fixed dock) | Yes | Yes | Yes | No (use tmux) | Yes | Yes (flexible grid -- any position) |
| AI agent (built-in) | Yes (Oz) | Yes (Zed Agent) | Yes (Copilot) | No | No | No | No | No | **No -- monitors, does not run** |
| Agent monitoring | Yes (own agent only) | Yes (own agent) | Yes (own agent) | No | No | No | No | No (organizes sessions) | **Yes -- any agent, across caps** |
| Drawing canvas | No | No | No (plugin only) | No | No | No | No | No | **Yes (TLDraw, first-class)** |
| Markdown viewer | No | Yes (preview) | Yes (preview) | No | No | No | No | No | **Yes (cap in grid, Obsidian-style)** |
| Block-based commands | Yes (signature) | No | No | No | No | No | No | No | v2 consideration |
| Project folder as context | No (session-based) | Partial (workspace) | Partial (.vscode/) | No | No | No | No | Partial (workspaces) | **Yes (core thesis -- .myco is context)** |
| Shell integration | Yes | Yes | Yes | Yes (deep) | Yes | Partial | No | No | v1.x (OSC 133) |
| Cross-project dashboard | No | No | No | No | No | No | No | No (switcher only) | **Yes (top bar + sidebar)** |
| Team collaboration | Yes (Warp Drive) | Yes (multiplayer) | Yes (Live Share) | No | No | No | No | No | **No -- single-user by design** |
| Image protocol | Yes (Kitty) | No | No | Yes (iTerm2 protocol) | Yes (Kitty) | Yes (Kitty, native) | No | No | v3 (defer) |
| Open source | Yes (MIT + AGPL) | Yes (AGPL + proprietary) | Partial (MIT core, proprietary builds) | Yes (GPL) | Yes (MIT) | Yes (GPL) | Yes (Apache-2.0/MIT) | No (commercial) | Yes (open source from start) |
| Config format | Cloud + local | JSON | JSON | Plist/GUI | Plain text | Conf | TOML | GUI | JSON (.myco files) |

## Sources

- [Terminal Trove Comparison Table (2026)](https://terminaltrove.com/compare/terminals/) - Feature matrix across 41 terminals (HIGH confidence)
- [Warp Terminal Features](https://www.warp.dev/terminal) - Block-based interface, Warp Drive, agent management (HIGH confidence)
- [Warp Open Source Announcement](https://www.warp.dev/blog/warp-is-now-open-source) - MIT + AGPL dual license, April 2026 (HIGH confidence)
- [Warp Agent Management](https://docs.warp.dev/agents/using-agents/managing-agents) - Multi-agent monitoring panel, notifications (HIGH confidence)
- [Zed Terminal Integration](https://zed.dev/docs/terminal) - alacritty_terminal backend, dock positions (HIGH confidence)
- [Zed Agent Panel](https://zed.dev/docs/ai/agent-panel) - Context management, thread management, review workflow (HIGH confidence)
- [Zed AI Features](https://zed.dev/ai) - Agent panel, context assembly (HIGH confidence)
- [VS Code Workspaces](https://code.visualstudio.com/docs/editor/workspaces) - Folder-based workspace model (HIGH confidence)
- [iTerm2 Features](https://iterm2.com/features.html) - Shell integration, triggers, profiles, status bar (HIGH confidence)
- [iTerm2 Shell Integration](https://iterm2.com/3.0/documentation-shell-integration.html) - Prompt detection, command history (HIGH confidence)
- [Ghostty Features](https://ghostty.org/docs/features) - Zig-based, Metal/OpenGL, 120fps, ~45MB RAM (HIGH confidence)
- [Kitty Terminal](https://sw.kovidgoyal.net/kitty/) - GPU rendering, image protocol, kittens plugin system (HIGH confidence)
- [Alacritty Features](https://github.com/alacritty/alacritty/blob/master/docs/features.md) - Minimalist philosophy, no tabs/splits by design (HIGH confidence)
- [Rio Terminal](https://rioterm.com/) - Rust + WebGPU, Sugarloaf renderer, native splits (MEDIUM confidence)
- [WezTerm Multiplexing](https://wezterm.org/multiplexing.html) - Built-in multiplexer, Lua config, SSH domains (HIGH confidence)
- [Beam Terminal Organizer](https://getbeam.dev/) - Workspace-based session management, saved layouts (MEDIUM confidence)
- [Intent Workspace](https://www.augmentcode.com/product/intent) - Spec-driven multi-agent orchestration, isolated workspaces (MEDIUM confidence)
- [Cursor Context Management](https://datalakehousehub.com/blog/2026-03-context-management-cursor/) - Workspace indexing, .cursorrules, Notepads (MEDIUM confidence)
- [OSC 133 Shell Integration Protocol](https://gist.github.com/tep/e3f3d384de40dbda932577c7da576ec3) - FinalTerm semantic prompt specification (HIGH confidence)
- [Ghostty Shell Integration](https://deepwiki.com/ghostty-org/ghostty/9.1-shell-integration-system) - Shell detection, auto-configuration (MEDIUM confidence)
- [AI Agent Configuration Patterns](https://www.sotaaz.com/post/ai-coding-rules-guide-en) - CLAUDE.md, .cursorrules, AGENTS.md ecosystem (MEDIUM confidence)
- [tldraw SDK](https://tldraw.dev/) - React-based infinite canvas, custom shapes, file persistence (HIGH confidence)
- [Terminal Graphics Protocols](https://akmatori.com/blog/terminal-graphics-protocols) - Kitty, Sixel, iTerm2 image protocols (MEDIUM confidence)

---
*Feature research for: AI-native terminal/workspace application*
*Researched: 2026-05-15*
