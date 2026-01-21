#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};

use device_kit::{
    Result,
    led_strip::{Current, Frame1d, Gamma, colors, led_strip},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    pub LedStrip96 { // can add 'pub' to make struct public
        pin: PIN_4,
        len: 96,
        // Optionals
        pio: PIO1,     // which of 2 or 3 PIO resources to use
        dma: DMA_CH5,  // which of 12 DMA resources to use
        max_current: Current::Milliamps(500), // default is 300ma
        gamma: Gamma::SmartLeds, // compatibility curve (= 2.8)
        max_frames: 0, // Disable animation; write_frame() still works
    }
    // Gamma correction and current limiting are folded into a single lookup table
    // (one table lookup per RGB channel at runtime).
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Must match the pin, pio, dma in LedStrip96 above to avoid compilation error.
    let led_strip96 = LedStrip96::new(p.PIN_4, p.PIO1, p.DMA_CH5, spawner)?;

    let mut frame1d = Frame1d::filled(colors::BLUE);
    loop {
        for dot_index in 0..LedStrip96::LEN {
            frame1d[dot_index] = colors::LIGHT_GRAY;
            led_strip96.write_frame(frame1d).await?;
            Timer::after(Duration::from_millis(50)).await;
            frame1d[dot_index] = colors::BLUE;
        }
    }

    // Issues Fixed:
    //   -- Full-white estimate: ~60mA/pixel × 96 ≈ 5.76A (too much for my supply).
    //   -- Web and X11 colors, PNGs, mp4 assume sRGB color space. LEDs are linear.
    // Issues Remaining:
    // - Because of the weird wiring, it is hard to write text or draw a line.
    // - Can only connect 2 or 3 strips.
}
