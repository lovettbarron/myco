---
slug: tldraw-sketch-skill
status: complete
created: 2026-05-16
completed: 2026-05-16
---

# Summary: tldraw Sketch Interpretation Skill + Project Init Prompt

## What was done
1. Created a tldraw sketch interpretation guide that ships as part of every Myco project
2. Added a project initialization prompt that appears on startup when `.myco/` doesn't exist

## Files created
- `resources/context/tldraw-sketches.md` — bundled AI context template for tldraw interpretation
- `src/context.rs` — writes default context files to `.myco/context/`

## Files modified
- `src/main.rs` — added `mod context`
- `src/canvas/mod.rs` — calls `ensure_context_files()` on canvas creation
- `src/app.rs` — added `InitPrompt` state, dialog overlay rendering, keyboard handling, and `InitPromptAccept`/`InitPromptDismiss` actions
- `src/input/mod.rs` — added `InitPromptAccept` and `InitPromptDismiss` action variants

## Files removed
- `.claude/skills/SKILL.md` — replaced by project-native `.myco/context/`
- `.claude/skills/interpret-sketches.md` — replaced by project-native `.myco/context/`

## How the init prompt works
1. In `resumed()`, checks if `.myco/` exists in the project directory
2. If missing, sets `init_prompt = InitPrompt::Showing`
3. Renders a centered dialog overlay with semi-transparent backdrop
4. Intercepts keyboard input: Y/Enter to accept, N/Esc to dismiss
5. On accept: creates `.myco/canvas/`, `.myco/context/`, writes `tldraw-sketches.md`
6. On dismiss: prompt disappears, no files created
7. Mouse clicks are blocked while prompt is showing

## How context files work
1. `ensure_context_files()` creates `.myco/context/` and writes `tldraw-sketches.md` from bundled template
2. Only writes if the file doesn't already exist (user edits are preserved)
3. Also called during canvas creation as a fallback path
4. Any AI agent working in the project folder discovers the guide at `.myco/context/tldraw-sketches.md`
