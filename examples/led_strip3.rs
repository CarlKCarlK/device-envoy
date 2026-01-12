#![no_std]
#![no_main]

use core::{convert::Infallible, future};

use defmt::info;
use defmt_rtt as _;
use device_kit::{
    Result,
    led_strip::{Current, Frame1d, Gamma, colors, led_strips},
    led2d::{Frame2d, layout::LedLayout},
};
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;

// Our 2D panel is two 12x4 panels stacked vertically.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

led_strips! {
    pio: PIO0, // Optional; defaults to PIO0.
    LedStrips0 { // Name for this collection of strips.
        gpio0: {                                    // Prefix used to name generated types.
            pin: PIN_0,                             // GPIO pin for LED data signal.
            len: 8,                                 // 8 LEDs on this strip.
            max_current: Current::Milliamps(25),    // Required per strip.
        },
        gpio3: {
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(75),
            gamma: Gamma::Gamma2_2,                 // Optional; default Gamma2_2.
            max_frames: 1,                          // Optional; default 16.
            dma: DMA_CH11,                          // Optional; auto-assigned by strip order.
        },
        gpio4: {
            pin: PIN_4,
            len: 96,
            max_current: Current::Milliamps(250),
            max_frames: 2,                          // cmk000000 test this to failure
            led2d: {                                // Optional panel configuration for 2D displays.
                width: 8,                           // Panel width in LEDs.
                height: 12,                         // Panel height in LEDs.
                led_layout: LED_LAYOUT_12X8_ROTATED, // Two 12x4 panels stacked and rotated.
                font: Font4x6Trim,                  // 4x6 pixel font without the usual 1 pixel spacing.
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

    // Create the two LED strips and one panel on GPIO0, GPIO3, and GPIO4.
    let (gpio0_led_strip, gpio3_led_strip, gpio4_led2d) = LedStrips0::new(
        p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH11, p.PIN_4, p.DMA_CH2, spawner,
    )?;

    info!("Setting GPIO0 to white, GPIO3 to alternating blue, GPIO4 to Go Go animation");

    // Turn on all-white on GPIO0 strip.
    let frame_gpio0 = Frame1d::filled(colors::WHITE);
    gpio0_led_strip.write_frame(frame_gpio0).await?;

    // Turn on every other LED in blue on GPIO3 strip.
    let mut frame_gpio3 = Frame1d::new();
    for pixel_index in (0..frame_gpio3.len()).step_by(2) {
        frame_gpio3[pixel_index] = colors::BLUE;
    }
    gpio3_led_strip.write_frame(frame_gpio3).await?;

    // Animate "Go Go" text on GPIO4 2D panel.
    let mut frame_go_top = Frame2d::new();
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
