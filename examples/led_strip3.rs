#![no_std]
#![no_main]

use core::convert::Infallible;
use core::future;

use defmt::info;
use defmt_rtt as _;
use device_kit::Result;
use device_kit::led_strip::led_strips;
use device_kit::led_strip::{Current, Frame, colors};
use device_kit::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;

led_strips! {
    LedStrips {
        gpio0: {
            dma: DMA_CH0,
            pin: PIN_0,
            len: 8,
            max_current: Current::Milliamps(50),
        },
        gpio3: {
            dma: DMA_CH1,
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(150),
            led2d: {
                width: 12,
                height: 4,
                led_layout: LED_LAYOUT_12X4,
                max_frames: 1, // cmk000000 delete
                font: Font3x4Trim,
            }
        },
        gpio4: {
            dma: DMA_CH2,
            pin: PIN_4,
            len: 96,
            max_current: Current::Milliamps(100),
            led2d: {
                width: 8,
                height: 12,
                led_layout: LED_LAYOUT_12X8_ROTATED,
                max_frames: 2, // cmk000000 move
                font: Font4x6Trim,
            }
        },
    }
}

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let (gpio0_led_strip, gpio3_led_strip, gpio4_led_strip) = LedStrips::new(
        p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH1, p.PIN_4, p.DMA_CH2, spawner,
    )?;

    info!("Setting GPIO0 to white, GPIO3 to Rust text, GPIO4 to Go Go animation");

    let frame_gpio0 = Frame::<{ Gpio0LedStrip::LEN }>::filled(colors::WHITE);
    gpio0_led_strip.write_frame(frame_gpio0).await?;

    let gpio3_led_strip_led2d = Gpio3LedStripLed2d::from_strip(gpio3_led_strip, spawner)?;
    let text_colors = [colors::RED, colors::GREEN, colors::BLUE];
    gpio3_led_strip_led2d
        .write_text("Rust", &text_colors)
        .await?;

    let gpio4_led_strip_led2d = Gpio4LedStripLed2d::from_strip(gpio4_led_strip, spawner)?;

    let mut frame_go_top = Gpio4LedStripLed2dFrame::new();
    gpio4_led_strip_led2d.write_text_to_frame("Go", &[], &mut frame_go_top)?;

    let mut frame_go_bottom = Gpio4LedStripLed2dFrame::new();
    gpio4_led_strip_led2d.write_text_to_frame(
        "\nGo",
        &[colors::HOT_PINK, colors::LIME],
        &mut frame_go_bottom,
    )?;

    let frame_duration = Duration::from_millis(400);
    gpio4_led_strip_led2d
        .animate([
            (frame_go_top, frame_duration),
            (frame_go_bottom, frame_duration),
        ])
        .await?;

    future::pending::<Result<Infallible>>().await // Run forever
}
