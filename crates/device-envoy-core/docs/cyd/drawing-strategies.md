<!-- Rustdoc source included by device-envoy-core::cyd. -->

## Choose a drawing strategy

The example uses a full-screen buffered frame, the simplest and most flexible
option but also the one that requires the most memory. Start with
[`CydDisplay::full_frame_mut`] when a full-screen buffer is practical.
[`CydDisplay`] supports four general drawing strategies:

- **Full-screen buffering:** Use [`CydDisplay::full_frame_mut`] to draw the
  complete display into one buffer before flushing it. This is the simplest and
  most flexible approach, but it requires one RGB565 value for every screen
  pixel.
- **Regional buffering:** Use [`CydDisplay::frame_mut`] to draw graphics or text
  into a smaller rectangular buffer and flush it to any chosen region of the
  screen. Successive calls can reuse the same storage with different positions,
  widths, and heights, provided each rectangle fits the buffer's pixel
  capacity.
- **Tiled drawing:** Use [`CydDisplay::for_each_tile`] to redraw the scene one
  small tile at a time when a larger buffer would use too much memory. This
  requires only one tile-sized buffer.
- **Contiguous streaming:** Use [`CydDisplay::fill_contiguous`] or
  [`CydDisplay::fill_contiguous_full`] to generate or supply row-major RGB565
  pixels as the display consumes them. This needs no reusable pixel frame
  buffer and is useful whenever each pixel can be computed quickly from its
  screen coordinates, including simple geometric shapes, procedural graphics,
  and existing bitmap data.

Small heterogeneous scenes can instead be streamed with
[`CydDisplay::draw_items`], which also needs no reusable pixel frame buffer. See
the [`CydDisplay` documentation] for the compact storage table,
immediate-drawing capacity rule, and coordinate conventions.

[`CydDisplay`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html
[`CydDisplay::full_frame_mut`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.full_frame_mut
[`CydDisplay::frame_mut`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.frame_mut
[`CydDisplay::for_each_tile`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.for_each_tile
[`CydDisplay::fill_contiguous`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#tymethod.fill_contiguous
[`CydDisplay::fill_contiguous_full`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.fill_contiguous_full
[`CydDisplay::draw_items`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.draw_items
[`CydDisplay` documentation]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html
