<!-- TODO0 Consider deleting this reviewed Memory-scope evidence once CYD Pass 2 is implemented and released. -->

# CYD Pass 2 Memory Dispositions

This manual rendered-documentation review is keyed to the stable mechanical
inventory IDs. All 23 Memory records reconcile exactly: one owns an example,
19 link directly to an example that visibly exercises the item, and three
concrete component types are meaningfully covered through their constructor
results without naming the inferred concrete type. No Memory record remains
uncovered after the bounded remediation.

| ID | Item | Final disposition | Exact example destination | Visible evidence and item-specific rationale |
|---|---|---|---|---|
| `memory-21e874d6f69b` | `memory` | own-example | `index.html#compiled-cydmemory-example` | The module owns the normal compiled example and also presents focused orientation/failure and standalone-button examples. |
| `memory-0fd79d939860` | `ButtonMemory` | linked-example | `index.html#standalone-buttonmemory-example` | The type page links to an example that names `ButtonMemory`, constructs it, changes its state, and observes the result. |
| `memory-758938881e9c` | `ButtonMemory::new` | linked-example | `index.html#standalone-buttonmemory-example` | The example calls `ButtonMemory::new` and verifies the initial released state. |
| `memory-aa39eb9e47d1` | `ButtonMemory::set_pressed` | linked-example | `index.html#standalone-buttonmemory-example` | The example calls `set_pressed(true)` and verifies the pressed state. |
| `memory-5c4d8c97e37e` | `ButtonMemory::set_pressed_for_frame` | linked-example | `index.html#compiled-cydmemory-example` | The normal example schedules a released state for frame one and observes it after that frame flushes. |
| `memory-ce8ef854d742` | `CydDisplayMemory` | covered-by-type-family | `index.html#compiled-cydmemory-example` | `CydMemory::display` and `owned_parts` return and use the concrete display half for sizing and frame creation; spelling the inferred type adds no behavior. |
| `memory-395d3bcf8976` | `CydFrameMemory` | covered-by-type-family | `index.html#compiled-cydmemory-example` | The example obtains the concrete frame from the display, draws into it, and flushes it; spelling the inferred frame type adds no behavior. |
| `memory-ba6c8a7d2cda` | `CydMemory` | linked-example | `index.html#compiled-cydmemory-example` | The type page links to the example that imports, constructs, and uses `CydMemory` throughout. |
| `memory-9b860872e22f` | `CydMemory::button_memory` | linked-example | `index.html#compiled-cydmemory-example` | The example calls the method and observes ordinary and frame-scheduled button state. |
| `memory-8a18901002f3` | `CydMemory::display` | linked-example | `index.html#compiled-cydmemory-example` | The example clones the display, verifies its size, and uses another clone to create a frame. |
| `memory-59199289b983` | `CydMemory::flush_count` | linked-example | `index.html#compiled-cydmemory-example` | The example asserts the exact count after flushing. |
| `memory-69c67fa9da3e` | `CydMemory::last_flush_rectangle` | linked-example | `index.html#compiled-cydmemory-example` | The example asserts the exact full-screen rectangle after flushing. |
| `memory-f5f8f2be073a` | `CydMemory::new` | linked-example | `index.html#compiled-cydmemory-example` | The example constructs a styled 320×240 in-memory surface with `new`. |
| `memory-6385bfd62dc1` | `CydMemory::new_with_orientation` | linked-example | `index.html#orientation-and-frame-budget-example` | The example selects portrait orientation and verifies both the reported orientation and 240×320 logical size. |
| `memory-d8aa0506708a` | `CydMemory::owned_parts` | linked-example | `index.html#compiled-cydmemory-example` | The example obtains both parts, checks the display, injects touch, and reads it through the touch half. |
| `memory-8b02fe4c4eea` | `CydMemory::pixel` | linked-example | `index.html#compiled-cydmemory-example` | The normal example reads a known bitmap pixel; the orientation example also observes pixel movement after rotation. |
| `memory-33ff6b99ee62` | `CydMemory::push_touch_event` | linked-example | `index.html#compiled-cydmemory-example` | The example injects `TouchEvent::Up` and reads the same event from the owned touch component. |
| `memory-c76d1abeb8c9` | `CydMemory::rotate_framebuffer_180` | linked-example | `index.html#orientation-and-frame-budget-example` | The example draws a red corner pixel, rotates once, and verifies that it moved to the opposite corner. |
| `memory-1a46812ea641` | `CydMemory::set_frame_budget` | linked-example | `index.html#orientation-and-frame-budget-example` | The example sets a one-frame budget, flushes once successfully, and observes failure on the second flush. |
| `memory-5c1bd0593e94` | `CydTouchMemory` | covered-by-type-family | `index.html#compiled-cydmemory-example` | `owned_parts` yields the concrete calibrated touch half and the example reads its injected event; spelling the inferred type adds no behavior. |
| `memory-f6df54173678` | `memory::Error` | linked-example | `index.html#orientation-and-frame-budget-example` | The example imports `Error` and observes it from an exhausted frame-budget flush. |
| `memory-d1d7c21e845e` | `memory::Error::OutOfFrames` | linked-example | `index.html#orientation-and-frame-budget-example` | The example compares the failed second flush with this exact variant. |
| `memory-06ed47e8e931` | `assert_framebuffer_matches_expected_png` | linked-example | `index.html#compiled-cydmemory-example` | The visible example calls the golden-image helper with its manifest directory and asset path and asserts success. |

## Result

The Memory documentation now presents three readable workflows rather than one
overloaded example: normal framebuffer and golden-image testing, orientation
and frame-budget failure, and standalone button testing. Every public Memory
item has a useful description and either direct compiled-example coverage or a
documented, behaviorally exercised inferred-type family relationship.
