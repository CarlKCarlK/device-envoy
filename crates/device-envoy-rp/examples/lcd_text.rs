#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, panic};
use device_envoy_rp::{Result, lcd_text::LcdText as _};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

device_envoy_rp::lcd_text! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTextSimple {
        width: 16,
        height: 2,
        address: 0x27
    }
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;

    lcd_text_simple.write_text("This line is definitely longer than sixteen\nAnd this one too");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("Unicode: cafe\u{301} ☕\nnaive — piñata");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("Line 1\nLine 2\nLine 3\nLine 4");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("Hello from\ndevice-envoy!");

    pending().await
}
