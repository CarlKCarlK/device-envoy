#![allow(missing_docs)]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, panic};
use defmt::info;

use device_envoy::{
    Result,
    button::{Button, PressedTo},
    led_strip::{Current, Frame1d, colors, led_strips},
    led2d::Frame2d,
    led2d::Led2dFont,
    led2d::layout::LedLayout,
};
use {defmt_rtt as _, panic_probe as _};

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_4X12_ROTATED: LedLayout<48, 4, 12> = LED_LAYOUT_12X4.rotate_cw();

led_strips! {
    pio: PIO0,
    LedStripsPio0 {
        Gpio0LedStrip: {
            pin: PIN_0,
            len: 8,
            max_current: Current::Milliamps(100),
        },
        Gpio3Led2d: {
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(300),
            led2d: {
                led_layout: LED_LAYOUT_4X12_ROTATED,
                font: Led2dFont::Font4x6Trim,
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let (gpio0_led_strip, gpio3_led2d) =
        LedStripsPio0::new(p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH1, spawner)?;
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    const ANIMATION_DELAY: embassy_time::Duration = embassy_time::Duration::from_millis(50);

    loop {
        let mut frame1d = Frame1d::new();
        let mut frame1d_b = Frame1d::new();
        for index in 0..4 {
            let mut frame2d = Frame2d::new();
            let mut frame2d_b = Frame2d::new();
            let text = match index {
                0..=3 => &["1", "2", "3", "4"][index],
                _ => " ",
            };
            info!("Demo with text {:?}", text);

            // write
            frame1d[index] = colors::YELLOW;
            gpio0_led_strip.write_frame(frame1d)?;
            gpio3_led2d.write_text(text, &[colors::YELLOW]).await?;
            button.wait_for_press().await;

            // write again
            frame1d[index] = colors::RED;
            gpio0_led_strip.write_frame(frame1d)?;
            gpio3_led2d.write_text(text, &[colors::RED]).await?;
            button.wait_for_press().await;

            // animate
            frame1d_b[index] = colors::YELLOW;
            gpio0_led_strip.animate([(frame1d_b, ANIMATION_DELAY), (frame1d, ANIMATION_DELAY)])?;
            gpio3_led2d.write_text_to_frame(text, &[colors::RED], &mut frame2d)?;
            gpio3_led2d.write_text_to_frame(text, &[colors::YELLOW], &mut frame2d_b)?;
            gpio3_led2d.animate([(frame2d_b, ANIMATION_DELAY), (frame2d, ANIMATION_DELAY)])?;
            button.wait_for_press().await;

            // animate again
            frame1d_b[index] = colors::CYAN;
            gpio0_led_strip.animate([(frame1d_b, ANIMATION_DELAY), (frame1d, ANIMATION_DELAY)])?;
            gpio3_led2d.write_text_to_frame(text, &[colors::YELLOW], &mut frame2d)?;
            gpio3_led2d.write_text_to_frame(text, &[colors::CYAN], &mut frame2d_b)?;
            gpio3_led2d.animate([(frame2d_b, ANIMATION_DELAY), (frame2d, ANIMATION_DELAY)])?;
            button.wait_for_press().await;

            // write again, again
            frame1d[index] = colors::LIME;
            gpio0_led_strip.write_frame(frame1d)?;
            gpio3_led2d.write_text(text, &[colors::LIME]).await?;
            button.wait_for_press().await;
        }
    }
}
