<!-- markdownlint-disable MD041 -->

A device abstraction for NeoPixel-style (WS2812) LED strips.

Use method `write_frame` to set each LED to an individual color. Use method `animate`
to look through a sequence of frames.

## Example

Define a 48-LED strip and set every second LED to blue:

```rust
use device_kit::{Result, led_strip::{self, Frame, colors}};
use embassy_executor::Spawner;

led_strip! {
    LedStrip {
        pin: PIN_3,
        len: 48,
    }
}

async fn example(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip = LedStrip::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;

    let mut frame = Frame::new();
    for pixel_index in (0..frame.len()).step_by(2) {
        frame[pixel_index] = colors::BLUE;
    }
    led_strip.write_frame(frame).await?;

    Ok(core::future::pending().await) // wait forever
}
```

NOTES:

* wraps struct LedStrip and gives access to all its methods
* don't name it LedStrip in texample
* the macro has optional values
* other version of sharing PIO (whatever that is) and 2D

This module treats a strip as a 1D line of lights. If your LED strip forms a grid, see [`Led2d`](crate::led2d::Led2d) for text, graphics, and animation.

## Macro Configuration

In addition to specifying the GPIO `pin` and `len`, the `led_strip!` macro supports optional fields: `pio`, `dma`, `max_current`, `gamma`, and `max_frames`. See the Configuration section below for details.

## The `led_strips!` Macro (Advanced)

For **multiple strips sharing one PIO**, use `led_strips!` instead:

```rust
led_strips! {
    pio: PIO0,
    LedStripGroup {
        strip1: {
            pin: PIN_0,
            len: 8,
        },
        strip2: {
            pin: PIN_1,
            len: 16,
        },
    }
}

// Use the generated group constructor:
let (strip1, strip2) = LedStripGroup::new(
    p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_1, p.DMA_CH1, spawner
)?;
```

Most projects only need `led_strip!`. Use `led_strips!` only when you have multiple strips on different state machines.
