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
    led2d::layout::LedLayout,
};
use embassy_executor::Spawner;
use embassy_rp::init;
use embassy_time::Duration;
use panic_probe as _;

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

led2d! {
    pub Led2DAnimate,
    pin: PIN_4,
    width: 8,
    height: 12,
    led_layout: LED_LAYOUT_12X8_ROTATED,
    font: Font4x6Trim,
    pio: PIO1,
    dma: DMA_CH1,
    max_current: Current::Milliamps(300),
    gamma: Gamma::Linear,
    max_frames: 3,
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("LED 2D Example: Animated text on a rotated 12x8 panel");
    let p = init(Default::default());

    let gpio4_led_strip = Led2DAnimate::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;

    let mut frame_a = Led2DAnimateFrame::new();
    let colors_a = [
        colors::RED,
        colors::ORANGE,
        colors::YELLOW,
        colors::GREEN,
        colors::CYAN,
        colors::BLUE,
    ];
    gpio4_led_strip.write_text_to_frame("Go\nGo", &colors_a, &mut frame_a)?;

    let mut frame_b = Led2DAnimateFrame::new();
    let colors_b = [
        colors::MAGENTA,
        colors::PURPLE,
        colors::BLUE,
        colors::CYAN,
        colors::GREEN,
        colors::YELLOW,
    ];
    gpio4_led_strip.write_text_to_frame("Go\nGo", &colors_b, &mut frame_b)?;

    let mut frame_c = Led2DAnimateFrame::new();
    let colors_c = [
        colors::WHITE,
        colors::PINK,
        colors::LIME,
        colors::ORANGE,
        colors::RED,
        colors::HOT_PINK,
    ];
    gpio4_led_strip.write_text_to_frame("Go\nGo", &colors_c, &mut frame_c)?;

    let frame_duration = Duration::from_millis(250);
    gpio4_led_strip
        .animate([
            (frame_a, frame_duration),
            (frame_b, frame_duration),
            (frame_c, frame_duration),
        ])
        .await?;

    future::pending().await
}
