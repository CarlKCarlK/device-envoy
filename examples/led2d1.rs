#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_kit::Result;
use device_kit::led_strip::colors;
use device_kit::led2d;
use device_kit::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_rp::init;
use panic_probe as _;

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();

led2d! {
    pub led12x4,
    pin: PIN_3,
    width: 12,
    height: 4,
    led_layout: LED_LAYOUT_12X4,
    font: Font3x4Trim,
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("LED 2D Example: Write text on a 12x4 panel");
    let p = init(Default::default());

    let led12x4 = Led12x4::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;

    let colors = [
        colors::RED,
        colors::ORANGE,
        colors::YELLOW,
        colors::GREEN,
        colors::BLUE,
        colors::PURPLE,
    ];
    led12x4.write_text("Rust", &colors).await?;

    core::future::pending().await
}
