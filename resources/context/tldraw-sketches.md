# Interpreting tldraw Sketches

This project uses Myco with tldraw canvas panels. Canvas files are saved as `.tldr` (JSON) in `.myco/canvas/`. This document tells AI agents how to interpret them.

## Finding Canvas Files

```bash
find .myco/canvas/ -name "*.tldr" -type f 2>/dev/null | sort -t/ -k3 -r
```

Use `ls -lt .myco/canvas/` to find the most recently modified.

## File Format

tldraw files are JSON with two possible structures:

**Myco embedded format:**
```json
{
  "document": {
    "schema": { "schemaVersion": 2, "sequences": {...} },
    "store": {
      "shape:abc123": {...},
      "binding:xyz789": {...},
      "asset:img001": {...}
    }
  }
}
```

**Standard .tldr export:**
```json
{
  "tldrawFileFormatVersion": 1,
  "schema": {...},
  "records": [ { "id": "shape:abc123", "typeName": "shape", ... } ]
}
```

## Extracting Shapes

```bash
# Myco format (store object)
jq '[.document.store | to_entries[] | select(.key | startswith("shape:")) | .value]' file.tldr

# Standard format (records array)
jq '[.records[] | select(.typeName == "shape")]' file.tldr
```

## Shape Types

| Type | What it is | Has readable text? |
|------|-----------|-------------------|
| `geo` | Rectangle, ellipse, diamond, triangle, cloud, star, etc. Subtype in `props.geo` | Yes — `props.richText` or `props.text` |
| `text` | Standalone text block | Yes — `props.richText` or `props.text` |
| `note` | Sticky note | Yes — `props.richText` or `props.text` |
| `arrow` | Directional connector | Sometimes — label in `props.richText` |
| `frame` | Named container, clips children | Yes — `props.name` |
| `draw` | Freehand stroke (base64 points) | No — visual only |
| `highlight` | Semi-transparent marker | No — visual only |
| `line` | Multi-point line | No |
| `group` | Invisible logical container | No |
| `image` | Embedded image (references asset) | `props.altText` if set |
| `bookmark` | URL card | `props.url` |
| `embed` | Iframe (YouTube, Figma, etc.) | `props.url` |

Every shape has: `x`, `y` (position), `rotation`, `opacity`, `parentId` (page or parent shape), `props` (type-specific).

Geo shapes also have `props.w` and `props.h` for dimensions. The `props.geo` field specifies the geometric primitive: `rectangle`, `ellipse`, `triangle`, `diamond`, `pentagon`, `hexagon`, `octagon`, `star`, `rhombus`, `cloud`, `heart`, `oval`, `arrow-right`, `arrow-left`, `arrow-up`, `arrow-down`, `x-box`, `check-box`, `trapezoid`, `rhombus-2`.

## Extracting Text

Text lives in `props.richText` (ProseMirror JSON) or the deprecated `props.text` (plain string).

**richText structure:**
```json
{
  "type": "doc",
  "content": [{
    "type": "paragraph",
    "content": [
      { "type": "text", "text": "Hello" },
      { "type": "text", "text": " bold", "marks": [{"type": "bold"}] }
    ]
  }]
}
```

Walk `richText.content[].content[].text` and concatenate, preserving paragraph breaks.

```bash
jq '[.document.store | to_entries[] | select(.key | startswith("shape:")) | .value | select(.props.richText.content[0].content != null) | {id: .id, type: .type, geo: .props.geo, text: [.props.richText.content[].content[]?.text] | join(" "), x: .x, y: .y, w: .props.w, h: .props.h}]' file.tldr
```

## Connections (Bindings)

Arrows connect shapes via binding records:

```bash
jq '[.document.store | to_entries[] | select(.key | startswith("binding:")) | .value]' file.tldr
```

Each binding has `fromId` (arrow), `toId` (target shape), `props.terminal` (`"start"` or `"end"`). An arrow from A to B produces two bindings — one for each terminal.

## Spatial Analysis

**Position:** Each shape is at `(x, y)` relative to its parent. Geo shapes span `(x, y)` to `(x + props.w, y + props.h)`.

**Clustering:** Shapes within ~50px likely form a group. Vertical stacks (similar x, increasing y) suggest lists. Horizontal rows (similar y, increasing x) suggest alternatives or parallel items.

**Flow direction:** Arrow directions indicate process flow. Check if arrows consistently point one direction.

**Containment:** If a shape's `parentId` references another shape (not `page:*`), it's inside a frame or group.

## Style Properties

| Property | Values |
|----------|--------|
| Color | `black`, `grey`, `light-violet`, `violet`, `blue`, `light-blue`, `yellow`, `orange`, `green`, `light-green`, `light-red`, `red`, `white` |
| Fill | `none`, `semi`, `solid`, `pattern` |
| Dash | `draw`, `solid`, `dashed`, `dotted`, `none` |
| Size | `s`, `m`, `l`, `xl` |
| Font | `draw`, `sans`, `serif`, `mono` |

## Handling Freehand-Only Canvases

Freehand `draw` shapes contain base64-encoded point data — you cannot read the visual content from JSON alone. When the canvas is all freehand:

1. Report spatial distribution (where strokes cluster)
2. Count separate draw shapes
3. Note any structured shapes mixed in
4. Say: "This canvas contains freehand drawings. I can see N strokes clustered at [locations] but cannot interpret the visual content. Describe what you drew, or export as PNG/SVG so I can view the image."

## Output Format

```
## Canvas: [filename]
Shapes: [count] ([breakdown by type])

### Text Content
[Each text/note/geo label with position]

### Layout
[Spatial arrangement — groups, flow, containment]

### Connections
[Arrow connections: "A -> B" with labels]

### Interpretation
[Best guess: wireframe, flow diagram, mind map, architecture sketch, etc.]
```

## Mapping Sketches to Implementation

- **Wireframes/UI sketches** → component hierarchy, CSS Grid layout, page structure
- **Flow diagrams** → state machines, API sequences, user journeys
- **Architecture diagrams** → module structure, data flow, service boundaries
- **Mind maps** → feature lists, requirement trees, exploration areas
- **Annotated screenshots** → specific UI changes, bug locations

Always confirm your interpretation before implementing.
