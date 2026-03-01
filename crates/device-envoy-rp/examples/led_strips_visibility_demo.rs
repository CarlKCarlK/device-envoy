#![allow(missing_docs)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use device_envoy::led_strip::{Current, led_strips};
use panic_probe as _;

// Public (visibility is required)
led_strips! {
    pub PublicLedStrips {
        Gpio0LedStrip: { pin: PIN_0, len: 8, max_current: Current::Milliamps(25) }
    }
}

// Explicitly crate-private (new capability)
led_strips! {
    pio: PIO1,
    pub(crate) CrateLedStrips {
        Gpio2LedStrip: { pin: PIN_2, len: 16, max_current: Current::Milliamps(50) }
    }
}

mod inner {
    use super::*;

    // Module-private (new capability)
    led_strips! {
        pio: PIO0,
        pub(super) SuperLedStrips {
            Gpio4LedStrip: { pin: PIN_4, len: 24, max_current: Current::Milliamps(75) }
        }
    }

    // This function can access SuperLedStrips because it's in the same module
    #[allow(dead_code)]
    pub fn demonstrate_super_access() {
        type _Super = SuperLedStrips; // Only accessible from this module and parents
    }
}

// This function can access all types because they're in the same module
#[allow(dead_code)]
fn demonstrate_access() {
    // All these work because we're in the same module
    type _Public = PublicLedStrips; // Would be accessible from other crates
    type _Crate = CrateLedStrips; // Only accessible within this crate
    type _Super = inner::SuperLedStrips; // Only accessible from parent modules
}

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) -> ! {
    loop {}
}
