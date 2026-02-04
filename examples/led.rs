#![allow(missing_docs)]
//! Example of using the Led device abstraction for blinking patterns.
#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt_rtt as _;
use device_envoy::{
    Result,
    led::{Led, LedStatic, OnLevel},
};
use embassy_executor::Spawner;
use embassy_rp::gpio::Level;
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    static LED_STATIC: LedStatic = Led::new_static();
    let led = Led::new(&LED_STATIC, p.PIN_1, OnLevel::High, spawner)?;

    // Turn on for 1 second
    led.set_level(Level::High);
    Timer::after(Duration::from_secs(1)).await;

    // Turn off for 1 second
    led.set_level(Level::Low);
    Timer::after(Duration::from_secs(1)).await;

    // Blink: 200ms on, 200ms off (repeating)
    led.animate(&[
        (Level::High, Duration::from_millis(200)),
        (Level::Low, Duration::from_millis(200)),
    ]);

    // Run forever; animation loops continuously
    core::future::pending().await
}
