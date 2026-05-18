# Interpreting Excalidraw Sketches

This project uses Myco with Excalidraw canvas panels. Canvas files are saved as `.excalidraw` (JSON) in `.myco/canvas/`. This document tells AI agents how to interpret them.

## Finding Canvas Files

```bash
find .myco/canvas/ -name "*.excalidraw" -type f 2>/dev/null | sort -t/ -k3 -r
```

Use `ls -lt .myco/canvas/` to find the most recently modified.

## File Format

Excalidraw files are JSON with a flat structure:

```json
{
  "type": "excalidraw",
  "version": 2,
  "source": "myco",
  "elements": [
    {
      "id": "abc123",
      "type": "rectangle",
      "x": 100, "y": 200,
      "width": 300, "height": 150,
      "strokeColor": "#1e1e1e",
      "backgroundColor": "#a5d8ff",
      "fillStyle": "hachure",
      "roughness": 1,
      "opacity": 100,
      "angle": 0,
      "groupIds": [],
      "boundElements": [{ "id": "arrow1", "type": "arrow" }]
    }
  ],
  "appState": {
    "viewBackgroundColor": "#ffffff",
    "theme": "dark"
  }
}
```

## Extracting Elements

```bash
# All elements
jq '.elements' file.excalidraw

# Filter by type
jq '[.elements[] | select(.type == "rectangle")]' file.excalidraw
jq '[.elements[] | select(.type == "text")]' file.excalidraw
jq '[.elements[] | select(.type == "arrow")]' file.excalidraw
```

## Element Types

| Type | What it is | Has readable text? |
|------|-----------|-------------------|
| `rectangle` | Rectangle shape | No (use associated text element) |
| `ellipse` | Ellipse/circle | No |
| `diamond` | Diamond shape | No |
| `text` | Text block | Yes -- `text` field directly |
| `arrow` | Directional connector with optional label | Sometimes -- `text` field |
| `line` | Multi-point line | No |
| `freedraw` | Freehand stroke (point array) | No -- visual only |
| `image` | Embedded image | No |
| `frame` | Named container | Yes -- `name` field |

Every element has: `x`, `y` (position), `width`, `height` (dimensions), `angle` (rotation), `opacity`, `groupIds` (group membership), `boundElements` (connected elements).

## Extracting Text

Text is directly available on text elements:

```bash
# All text content with positions
jq '[.elements[] | select(.type == "text") | {id, text, x, y, width, height, fontSize}]' file.excalidraw

# Text from arrows (labels)
jq '[.elements[] | select(.type == "arrow" and .text != null) | {id, text}]' file.excalidraw
```

## Connections (Bound Elements)

Arrows connect shapes via the `boundElements` array on each shape and `startBinding`/`endBinding` on the arrow:

```bash
# Find all arrows
jq '[.elements[] | select(.type == "arrow")]' file.excalidraw

# Arrow bindings
jq '[.elements[] | select(.type == "arrow") | {id, startBinding: .startBinding.elementId, endBinding: .endBinding.elementId, text}]' file.excalidraw
```

Each arrow has:
- `startBinding.elementId` -- the shape the arrow starts from
- `endBinding.elementId` -- the shape the arrow points to
- `points` -- array of [x, y] relative coordinates for the arrow path

Each connected shape has `boundElements` listing arrows attached to it.

## Spatial Analysis

**Position:** Each element is at `(x, y)` with `width` and `height`. The element spans from `(x, y)` to `(x + width, y + height)`.

**Clustering:** Elements within ~50px likely form a group. Vertical stacks (similar x, increasing y) suggest lists. Horizontal rows (similar y, increasing x) suggest alternatives or parallel items.

**Flow direction:** Arrow directions indicate process flow. Check `startBinding` and `endBinding` to trace connections.

**Grouping:** Elements sharing the same value in `groupIds` are visually grouped. Frame elements contain children by spatial containment.

## Style Properties

| Property | Values |
|----------|--------|
| `strokeColor` | Hex color string (e.g., `"#1e1e1e"`, `"#e03131"`) |
| `backgroundColor` | Hex color or `"transparent"` |
| `fillStyle` | `"hachure"`, `"cross-hatch"`, `"solid"`, `"zigzag"` |
| `strokeWidth` | Number (1, 2, 4) |
| `strokeStyle` | `"solid"`, `"dashed"`, `"dotted"` |
| `roughness` | 0 (architect), 1 (artist), 2 (cartoonist) |
| `opacity` | 0-100 |

## Handling Freehand-Only Canvases

Freehand `freedraw` elements contain point arrays. When the canvas is all freehand:

1. Report spatial distribution (where strokes cluster)
2. Count separate freedraw elements
3. Note any structured shapes mixed in
4. Say: "This canvas contains freehand drawings. I can see N strokes clustered at [locations] but cannot interpret the visual content. Describe what you drew, or export as PNG/SVG so I can view the image."

## Output Format

```
## Canvas: [filename]
Elements: [count] ([breakdown by type])

### Text Content
[Each text element with position]

### Layout
[Spatial arrangement -- groups, flow, containment]

### Connections
[Arrow connections: "A -> B" with labels]

### Interpretation
[Best guess: wireframe, flow diagram, mind map, architecture sketch, etc.]
```

## Mapping Sketches to Implementation

- **Wireframes/UI sketches** -> component hierarchy, CSS Grid layout, page structure
- **Flow diagrams** -> state machines, API sequences, user journeys
- **Architecture diagrams** -> module structure, data flow, service boundaries
- **Mind maps** -> feature lists, requirement trees, exploration areas
- **Annotated screenshots** -> specific UI changes, bug locations

Always confirm your interpretation before implementing.
