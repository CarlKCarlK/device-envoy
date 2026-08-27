<!-- Shared Rustdoc source included by device-envoy-core::cyd, device-envoy-esp::cyd, device-envoy-rp::cyd, device-envoy-core::wasm, and device-envoy-core::memory. -->

## Choose a drawing strategy

The example uses a full-screen buffered frame, the simplest and most flexible
option but also the one that requires the most memory. [`CydDisplay`] supports
four drawing strategies with different memory requirements:

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
  [`CydDisplay::fill_contiguous_full`] to send row-major pixels directly to the
  display with almost no buffering. This works especially well for bitmaps
  already stored or generated as RGB565 pixels because they can go directly to
  the display without first being copied into a frame buffer. See the
  [`CydDisplay::fill_contiguous` bitmap example] for a minimal demonstration.

Choose according to the available memory and how the application produces its
pixels. The [`CydDisplay` documentation] explains the tradeoffs and coordinate
conventions.

[`CydDisplay`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html
[`CydDisplay::full_frame_mut`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.full_frame_mut
[`CydDisplay::frame_mut`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.frame_mut
[`CydDisplay::for_each_tile`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.for_each_tile
[`CydDisplay::fill_contiguous`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#tymethod.fill_contiguous
[`CydDisplay::fill_contiguous` bitmap example]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#tymethod.fill_contiguous
[`CydDisplay::fill_contiguous_full`]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html#method.fill_contiguous_full
[`CydDisplay` documentation]: https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/trait.CydDisplay.html
