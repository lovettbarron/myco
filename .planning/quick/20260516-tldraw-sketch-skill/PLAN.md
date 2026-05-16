---
slug: tldraw-sketch-skill
description: Create a Claude Code skill for interpreting tldraw canvas files in Myco projects
status: in-progress
created: 2026-05-16
---

# Quick Task: tldraw Sketch Interpretation Skill

## Goal
Create a Claude Code skill at `.claude/skills/` that allows any Claude Code instance working on a Myco project to interpret tldraw canvas files as contextual input when asked to "check the most recent sketches."

## Tasks

1. **Create `.claude/skills/interpret-sketches.md`** — the skill file with:
   - Instructions for finding .tldr files in `.myco/canvas/`
   - JSON parsing guide for both tldraw file formats (records array and store object)
   - Shape type extraction and semantic interpretation
   - Spatial analysis heuristics (clustering, flow direction, containment)
   - Text extraction from richText/text properties
   - Connection graph reconstruction from bindings
   - Handling of freehand-only canvases (acknowledge limitation, describe spatial layout)
   - Output format: structured natural language description

2. **Create `.claude/skills/SKILL.md`** — index file registering the skill

3. **Validate** — dry-run the skill against the existing canvas file at `.myco/canvas/canvas-1778934089.tldr`
