# DNS dynamic layout

Coordinates are screen pixels. Dynamic field coordinates describe reserved rectangles as
`(x, y, width, height)`. Static captions and labels are baked into the background.

## Landscape (320x240)

| Purpose | Rectangle | Font | Color | Alignment |
|---|---:|---|---|---|
| DNS target | `(22, 76, 150, 20)` | `FONT_10X20` | `#ffffff` | Left |
| Lookup status | `(244, 82, 56, 20)` | `FONT_10X20` | `#79e2a4` or `#ff756e` | Center |
| Latest latency | `(100, 100, 120, 24)` | `PROFONT_24_POINT` | `#ffffff` | Center |
| Query count | `(27, 156, 50, 20)` | `FONT_10X20` | `#ffffff` | Center |
| Success count | `(135, 156, 50, 20)` | `FONT_10X20` | `#79e2a4` | Center |
| Failure count | `(243, 156, 50, 20)` | `FONT_10X20` | `#ff756e` | Center |

Touch rectangles:

| Button | Rectangle |
|---|---:|
| CAL | `(10, 202, 100, 28)` |
| WI-FI | `(110, 202, 100, 28)` |
| ROTATE | `(210, 202, 100, 28)` |

## Portrait (240x320)

| Purpose | Rectangle | Font | Color | Alignment |
|---|---:|---|---|---|
| DNS target | `(22, 68, 190, 20)` | `FONT_10X20` | `#ffffff` | Left |
| Latest latency | `(60, 119, 120, 24)` | `PROFONT_24_POINT` | `#ffffff` | Center |
| Query count | `(160, 180, 50, 20)` | `FONT_10X20` | `#ffffff` | Right |
| Success count | `(160, 202, 50, 20)` | `FONT_10X20` | `#79e2a4` | Right |
| Failure count | `(160, 220, 50, 20)` | `FONT_10X20` | `#ff756e` | Right |

Touch rectangles:

| Button | Rectangle |
|---|---:|
| CAL | `(10, 276, 73, 36)` |
| WI-FI | `(83, 276, 74, 36)` |
| ROTATE | `(157, 276, 73, 36)` |

## SVG workflow

- Remove `preview-values` before exporting a production background.
- `developer-guides` is hidden by default and can also be removed before export.
- Change `developer-guides` from `display="none"` to `display="inline"` to show all slots.
- SVG text positions use baselines; the `data-screen-*` attributes describe firmware rectangles.
