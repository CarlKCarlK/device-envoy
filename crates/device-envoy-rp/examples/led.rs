#![allow(missing_docs)]
//! Example of using the Led device abstraction for blinking patterns.
#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt_rtt as _;
use device_envoy_rp::{
    Result,
    led::{Led as _, LedLevel, LedRp, LedRpStatic, OnLevel},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    static LED_RP_STATIC: LedRpStatic = LedRp::new_static();
    let led_rp = LedRp::new(&LED_RP_STATIC, p.PIN_1, OnLevel::High, spawner)?;

    // Turn the LED on
    led_rp.set_level(LedLevel::On);
    Timer::after(Duration::from_secs(1)).await;

    // Turn the LED off
    led_rp.set_level(LedLevel::Off);
    Timer::after(Duration::from_millis(500)).await;

    // Play a blinking animation (looping: 200ms on, 200ms off)
    led_rp.animate([
        (LedLevel::On, Duration::from_millis(200)),
        (LedLevel::Off, Duration::from_millis(200)),
    ]);

    // Run forever; animation loops continuously
    core::future::pending().await
}
