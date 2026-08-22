#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, panic};
use device_envoy_rp::{Result, i2cs, lcd_text::LcdText as _};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    I2cs0 {
        LcdText16x2 { width: 16, height: 2, address: 0x27 },
        LcdText20x4 { width: 20, height: 4, address: 0x3F },
    }
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let (lcd_text16x2, lcd_text20x4) = I2cs0::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;

    loop {
        lcd_text16x2.write_text("LCD #1\n16x2");
        lcd_text20x4.write_text("LCD #2\n20x4\nshared i2c\naddress 0x3F");
        Timer::after(Duration::from_secs(1)).await;

        lcd_text16x2.write_text("Tick");
        lcd_text20x4.write_text("Tock");
        Timer::after(Duration::from_secs(1)).await;
    }
}
