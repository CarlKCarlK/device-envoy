#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, future};

use defmt::info;
use defmt_rtt as _;
use device_kit::Result;
use device_kit::led_strip::Current;
use device_kit::led_strip::Gamma;
use device_kit::led_strip::colors;
use device_kit::led_strip::led_strips;
use device_kit::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_rp::init;
use embassy_time::Duration;
use panic_probe as _;

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 12, 8> = LED_LAYOUT_12X8.rotate_180();

led_strips! {
    pio: PIO0,
    LedStripsPio0 {
        gpio4: {
            dma: DMA_CH1,
            pin: PIN_4,
            len: 96,
            max_current: Current::Milliamps(1000),
            gamma: Gamma::Linear,
            led2d: {
                width: 12,
                height: 8,
                led_layout: LED_LAYOUT_12X8_ROTATED,
                max_frames: 8,
                font: Font4x6Trim,
            }
        }
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

    let (gpio4_led_strip,) = LedStripsPio0::new(p.PIO0, p.PIN_4, p.DMA_CH1, spawner)?;
    let gpio4_led_strip = Gpio4LedStripLed2d::from_strip(gpio4_led_strip, spawner)?;

    let mut frame_a = Gpio4LedStripLed2d::new_frame();
    let colors_a = [
        colors::RED,
        colors::ORANGE,
        colors::YELLOW,
        colors::GREEN,
        colors::CYAN,
        colors::BLUE,
    ];
    gpio4_led_strip.write_text_to_frame("Go\nGo", &colors_a, &mut frame_a)?;

    let mut frame_b = Gpio4LedStripLed2d::new_frame();
    let colors_b = [
        colors::MAGENTA,
        colors::PURPLE,
        colors::BLUE,
        colors::CYAN,
        colors::GREEN,
        colors::YELLOW,
    ];
    gpio4_led_strip.write_text_to_frame("Go\nGo", &colors_b, &mut frame_b)?;

    let mut frame_c = Gpio4LedStripLed2d::new_frame();
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
