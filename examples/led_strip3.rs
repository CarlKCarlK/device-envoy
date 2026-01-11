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

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

led_strips! {
    pio: PIO0,
    LedStrips {
        gpio0: {
            pin: PIN_0,
            len: 8,
            max_current: Current::Milliamps(25),
        },
        gpio3: {
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(75),
            dma: DMA_CH11,
        },
        gpio4: {
            pin: PIN_4,
            len: 96,
            max_frames: 2, // cmk000000 test this to failure
            led2d: {
                width: 8,
                height: 12,
                led_layout: LED_LAYOUT_12X8_ROTATED,
                font: Font4x6Trim,
            }
        },
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let (gpio0_led_strip, gpio3_led_strip, gpio4_led2d) = LedStrips::new(
        p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH11, p.PIN_4, p.DMA_CH2, spawner,
    )?;

    info!("Setting GPIO0 to white, GPIO3 to alternating blue, GPIO4 to Go Go animation");

    let frame_gpio0 = Frame::<{ Gpio0LedStrip::LEN }>::filled(colors::WHITE);
    gpio0_led_strip.write_frame(frame_gpio0).await?;

    let mut frame_gpio3 = Frame::<{ Gpio3LedStrip::LEN }>::new();
    for pixel_index in (0..frame_gpio3.len()).step_by(2) {
        frame_gpio3[pixel_index] = colors::BLUE;
    }
    gpio3_led_strip.write_frame(frame_gpio3).await?;

    let mut frame_go_top = Gpio4Led2dFrame::new();
    gpio4_led2d.write_text_to_frame("Go", &[], &mut frame_go_top)?;

    let mut frame_go_bottom = Gpio4Led2dFrame::new();
    gpio4_led2d.write_text_to_frame(
        "\nGo",
        &[colors::HOT_PINK, colors::LIME],
        &mut frame_go_bottom,
    )?;

    let frame_duration = Duration::from_millis(400);
    gpio4_led2d
        .animate([
            (frame_go_top, frame_duration),
            (frame_go_bottom, frame_duration),
        ])
        .await?;

    future::pending::<Result<Infallible>>().await // Run forever
}
