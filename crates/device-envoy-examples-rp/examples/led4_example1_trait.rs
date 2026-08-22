#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use defmt_rtt as _;
use device_envoy_core::led4::Led4;
use device_envoy_rp::{
    Result,
    led4::{BlinkState, Led4Rp, Led4RpStatic, OutputArray, circular_outline_animation},
};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Duration, Timer};
use panic_probe as _;

async fn show_status(led4: &impl Led4) -> Infallible {
    // Blink "1234" for three seconds.
    led4.write_text(['1', '2', '3', '4'], BlinkState::BlinkingAndOn);
    Timer::after(Duration::from_secs(3)).await;

    // Run the circular outline animation for three seconds.
    led4.animate_text(circular_outline_animation(true));
    Timer::after(Duration::from_secs(3)).await;

    // Show "rUSt" solid forever.
    led4.write_text(['r', 'U', 'S', 't'], BlinkState::Solid);
    pending().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let cell_pins = OutputArray::new([
        Output::new(p.PIN_1, Level::High),
        Output::new(p.PIN_2, Level::High),
        Output::new(p.PIN_3, Level::High),
        Output::new(p.PIN_4, Level::High),
    ]);

    let segment_pins = OutputArray::new([
        Output::new(p.PIN_5, Level::Low),
        Output::new(p.PIN_6, Level::Low),
        Output::new(p.PIN_7, Level::Low),
        Output::new(p.PIN_8, Level::Low),
        Output::new(p.PIN_9, Level::Low),
        Output::new(p.PIN_10, Level::Low),
        Output::new(p.PIN_11, Level::Low),
        Output::new(p.PIN_12, Level::Low),
    ]);

    static LED4_STATIC: Led4RpStatic = Led4Rp::new_static();
    let led4 = Led4Rp::new(&LED4_STATIC, cell_pins, segment_pins, spawner)?;

    Ok(show_status(&led4).await)
}
