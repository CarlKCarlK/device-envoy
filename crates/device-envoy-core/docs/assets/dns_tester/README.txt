DNS SVG design bundle

Files:
- dns_landscape.svg: 320x240 landscape design.
- dns_portrait.svg: 240x320 portrait design.
- dns_landscape.png and dns_portrait.png: previews with sample dynamic values.
- dns_landscape_guides.png and dns_portrait_guides.png: annotated slot/hitbox previews.
- dns_landscape_background.png and dns_portrait_background.png: static backgrounds.
- dns_landscape.tga and dns_portrait.tga: static RGB TGA backgrounds with dynamic values removed.

Both designs label the footer as SETTINGS and include three controls: CAL, WI-FI, and ROTATE.

The palette uses a saturated mid-blue background with white values and amber controls,
tuned brighter than the initial dark design for CYD displays.

Dynamic text font plan:
- FONT_10X20: target/domain, status, and query/success/failure values.
- PROFONT_24_POINT: centered latency value.

The SVG uses Courier New at matching pixel heights only as a desktop preview approximation.
Each placeholder has XML comments and data attributes describing its firmware coordinates,
bounds, color, alignment, purpose, and font. See dynamic_layout.md for a compact reference.

The sample dynamic text is grouped under:
- preview-values
- preview-values-latency
- preview-values-stats

Remove those groups when creating a static background bitmap.

The hidden developer-guides group contains exact dynamic-slot and touch-region rectangles.
Change its display attribute from none to inline to inspect the guides visually.
