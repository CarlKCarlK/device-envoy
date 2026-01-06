<!-- markdownlint-disable MD041 -->

A device abstraction for NeoPixel-style (WS2812) LED strips.

Control individual LED colors with [`write_frame`](LedStrip::write_frame) or animate a sequence of frames with [`animate`](LedStrip::animate).

## Related

- [`led_strips!`] — Define multiple LED strips sharing one [PIO](crate#pio-programmable-io) resource
- [`Led2d`](crate::led2d::Led2d) — LED strips arranged in 2D grids. Adds text rendering and graphics

## led_strip! Example

Define a 48-LED strip and set every second LED to blue:

```rust,no_run
#![no_std]
#![no_main]

use panic_probe as _;
use core::convert::Infallible;
use core::default::Default;
use core::result::Result::Ok;

use embassy_executor::Spawner;
use device_kit::{Result, led_strip::{Frame, colors, led_strip}};

led_strip! {
    LedStrip3 {
        pin: PIN_3,
        len: 48,
    }
}

async fn example(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip3 = LedStrip3::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;

    let mut frame = Frame::new();
    for pixel_index in (0..frame.len()).step_by(2) {
        frame[pixel_index] = colors::BLUE;
    }
    led_strip3.write_frame(frame).await?;

    Ok(core::future::pending().await) // run forever
}
```

# How It Works

The [`led_strip!`] macro generates a wrapper struct that encapsulates the underlying [`LedStrip`] type. This generated struct has a `new()` constructor that takes the actual GPIO pin, PIO block, DMA channel, and async spawner.

In the example above, `led_strip! { LedStrip3 { ... } }` generates a struct named `LedStrip3` with all the methods of [`LedStrip`]. You then construct it with `LedStrip3::new(pin, pio, dma, spawner)`.

See [`led_strip!`] macro documentation for all configuration options (PIO, DMA channel, current limiting, gamma correction, frame animation size).

Here is an example using all optional fields with animation:

```rust,no_run
#![no_std]
#![no_main]

use panic_probe as _;
use core::convert::Infallible;
use core::default::Default;
use core::result::Result::Ok;

use device_kit::{Result, led_strip::{Current, Frame, Gamma, colors, led_strip}};
use embassy_executor::Spawner;
use embassy_time::Duration;

led_strip! {
    LedStrip4 {
        pin: PIN_4,
        len: 96,
        pio: PIO1,
        dma: DMA_CH3,
        max_current: Current::Milliamps(1000),
        gamma: Gamma::Linear,
        max_frames: 3,
    }
}

async fn animate_example(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let led_strip4 = LedStrip4::new(p.PIN_4, p.PIO1, p.DMA_CH3, spawner)?;

    let frame_duration = Duration::from_millis(300);

    led_strip4
        .animate([
            (Frame::filled(colors::RED), frame_duration),
            (Frame::filled(colors::GREEN), frame_duration),
            (Frame::filled(colors::BLUE), frame_duration),
        ])
        .await?;

    Ok(core::future::pending().await) // run forever
}
```

cmk LEAVE THIS ALONE

- wraps struct LedStrip and gives access to all its methods
- don't name it LedStrip in texample
- the macro has optional values
- other version of sharing PIO (whatever that is) and 2D
- animation continues until stoped by a new write_frame or animate
- panels vs grid?

Most projects only need `led_strip!`. Use `led_strips!` only when you have multiple strips on different state machines.
