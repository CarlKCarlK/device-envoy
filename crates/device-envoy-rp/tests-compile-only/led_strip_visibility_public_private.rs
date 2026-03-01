#![allow(missing_docs)]
#![no_std]
#![no_main]
#![allow(dead_code)]
//! Compile-only checks for public/private visibility in led_strip!.

use device_envoy::led_strip;
use panic_probe as _;

// Public visibility: accessible outside module.
led_strip! {
    pub LedStripPublic {
        pin: PIN_3,
        len: 24,
    }
}

mod private_case {
    use device_envoy::led_strip;

    // Private visibility: accessible only inside this module.
    led_strip! {
        pub(self) LedStripPrivate {
            pin: PIN_4,
            len: 12,
        }
    }

    pub fn use_private() {
        type _Test = LedStripPrivate;
    }
}

fn main() {
    // Public type should be accessible here.
    type _Test = LedStripPublic;

    // Private type should be accessible only via module helpers.
    private_case::use_private();

    // Uncommenting this should fail (private visibility):
    // type _Private = private_case::LedStripPrivate;
}
