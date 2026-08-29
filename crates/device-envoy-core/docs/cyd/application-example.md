<!-- Rustdoc source included by device-envoy-core::cyd. -->

## Application example

Startup code constructs the appropriate CYD device. Application code can then
read calibrated touch input and draw without naming a platform-specific type:

```rust,no_run
# #![cfg_attr(target_os = "none", no_std)]
# #![cfg_attr(target_os = "none", no_main)]
# #[cfg(target_os = "none")]
# #[panic_handler]
# fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{CydFrame, DrawItem},
    touch::TouchEvent,
};
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};

async fn draw_once<C: Cyd>(cyd: &mut C) -> Result<(), C::Error> {
    let (display, touch) = cyd.parts();
    let touch_event = touch.try_read()?;
    let mut frame = display.full_frame_mut();

    frame.write_text("Hello CYD");
    if let Some(TouchEvent::Down { point } | TouchEvent::Move { point }) =
        touch_event
    {
        DrawItem::Circle {
            center: (point.x as f32, point.y as f32),
            pixel_radius: 24.0,
            color: Rgb888::RED,
        }
        .draw(&mut frame);
    }

    frame.flush().await
}
```

An application would normally call `draw_once` repeatedly as part of its event
or frame loop.

![Output from the shared example after a touch at the center, rendered by CydMemory.][cyd_application_preview]
