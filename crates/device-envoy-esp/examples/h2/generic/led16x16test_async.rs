//! 16x16 panel mapping test using async RMT TX directly.
//!
//! This isolates async TX behavior from the higher-level led_strip device path.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    Error, Result,
    esp_hal::{
        gpio::Level,
        rmt::{PulseCode, TxChannelCreator as _},
    },
    init_and_start,
    init_and_start::rmt,
    led_strip::{RGB8, colors},
    led2d::layout::LedLayout,
};

esp_bootloader_esp_idf::esp_app_desc!();

const PANEL_16X16_PIN_NUM: u8 = 2;
const DOT_DELAY: Duration = Duration::from_millis(500);
const LEDS: usize = 256;
const PULSES: usize = LEDS * 24 + 1;
const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_column_major();

const BIT0: PulseCode = PulseCode::new(Level::High, 8, Level::Low, 17);
const BIT1: PulseCode = PulseCode::new(Level::High, 16, Level::Low, 9);

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!(
        "led16x16test_async starting on GPIO{} (one moving foreground dot)",
        PANEL_16X16_PIN_NUM
    );

    let mut tx_channel = rmt80
        .channel0
        .configure_tx(p.GPIO2, rmt::ws2812_tx_config())?;

    let mut frame1d = [colors::BLACK; LEDS];
    let mut pulse_buf = [PulseCode::end_marker(); PULSES];
    let xy_to_index = build_xy_to_index(LED_LAYOUT_16X16.index_to_xy());

    loop {
        for y_index in 0..16usize {
            for x_index in 0..16usize {
                frame1d.fill(colors::BLACK);
                let led_index = xy_to_index[y_index * 16 + x_index] as usize;
                frame1d[led_index] = colors::WHITE;

                encode_ws2812(&frame1d, &mut pulse_buf);
                tx_channel.transmit(&pulse_buf).await.map_err(Error::Rmt)?;
                Timer::after(DOT_DELAY).await;
            }
        }
    }
}

fn encode_ws2812(frame1d: &[RGB8; LEDS], pulse_buf: &mut [PulseCode; PULSES]) {
    for (led_index, pixel) in frame1d.iter().enumerate() {
        let grb = ((pixel.g as u32) << 16) | ((pixel.r as u32) << 8) | (pixel.b as u32);
        for bit_index in 0..24 {
            let bit = (grb >> (23 - bit_index)) & 1;
            pulse_buf[led_index * 24 + bit_index] = if bit == 1 { BIT1 } else { BIT0 };
        }
    }
    pulse_buf[LEDS * 24] = PulseCode::end_marker();
}

fn build_xy_to_index(index_to_xy: &[(u16, u16); LEDS]) -> [u16; LEDS] {
    let mut xy_to_index = [0u16; LEDS];
    for (led_index, &(x_index, y_index)) in index_to_xy.iter().enumerate() {
        let flat_xy_index = usize::from(y_index) * 16 + usize::from(x_index);
        xy_to_index[flat_xy_index] = led_index as u16;
    }
    xy_to_index
}
