#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, future};

use defmt::info;
use defmt_rtt as _;
use device_kit::{
    Result,
    led_strip::{Current, Gamma, colors},
    led2d,
    led2d::Frame2d,
    led2d::layout::LedLayout,
};
use embassy_executor::Spawner;
use embassy_rp::init;
use embassy_time::Duration;
use panic_probe as _;

// Our panel is two 12x4 panels stacked vertically.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

led2d! {
    Led12x8Animated {
        pin: PIN_4, // GPIO pin for LED data signal
        led_layout: LED_LAYOUT_12X8_ROTATED, // Two 12×4 panels stacked and rotated
        font: Led2dFont::Font4x6Trim, // Use a 4x6 pixel font without the usual 1 pixel spacing
        pio: PIO1, // PIO resource, default is PIO0
        dma: DMA_CH1, // DMA resource, default is DMA_CH0
        max_current: Current::Milliamps(300), // Power budget, default is 250 mA
        gamma: Gamma::Linear, // Color correction curve, default is Gamma2_2
        max_frames: 2, // maximum animation frames, default is 16
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("LED 2D Example: Animated text on a rotated 12x8 panel");
    let p = init(Default::default());

    let led_12x8_animated = Led12x8Animated::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;

    let mut frame_0 = Frame2d::new();
    // Empty colors array defaults to white
    led_12x8_animated.write_text_to_frame("Go", &[], &mut frame_0)?;

    let mut frame_1 = Frame2d::new();
    // "/n" starts a new line. Text does not wrap but rather clips.
    led_12x8_animated.write_text_to_frame(
        "\nGo",
        &[colors::HOT_PINK, colors::LIME],
        &mut frame_1,
    )?;

    // Animate between the two frames indefinitely.
    let frame_duration = Duration::from_secs(1);
    led_12x8_animated
        .animate([(frame_0, frame_duration), (frame_1, frame_duration)])
        .await?;

    future::pending().await // run forever
}
