#![allow(missing_docs)]
#![no_std]
#![no_main]
#![allow(dead_code)]
//! Test visibility support for led_strip! and led_strips! macros.

use device_envoy::led_strip;
use panic_probe as _;

// Test default visibility (public)
led_strip! {
    LedStripDefault {
        pin: PIN_3,
        len: 48,
    }
}

// Test explicit public visibility - separate module to avoid conflicts
mod test_explicit_pub {
    use device_envoy::led_strip;

    led_strip! {
        pub LedStripExplicitPub {
            pin: PIN_4,
            len: 24,
        }
    }

    // Re-export to demonstrate it's public
    // pub use LedStripExplicitPub as ExportedPub;
}

// Test pub(crate) visibility - separate module
mod test_pub_crate {
    use device_envoy::led_strip;

    led_strip! {
        pub(crate) LedStripPubCrate {
            pin: PIN_5,
            len: 12,
        }
    }

    // Re-export to demonstrate it's accessible within crate (would fail outside crate)
    // pub use LedStripPubCrate as ExportedPubCrate;
}

// Test private visibility - separate module
mod test_private {
    use device_envoy::led_strip;

    led_strip! {
        pub(self) LedStripPrivate {
            pin: PIN_6,
            len: 8,
        }
    }

    // This function can access the private type within the module
    pub fn use_private_type() -> &'static str {
        type _Test = LedStripPrivate;
        "accessible within module"
    }

    // Cannot re-export private type (would cause compile error):
    // pub use LedStripPrivate as ExportedPrivate;  // Uncomment to see error
}

// Test module visibility
mod test_module_visibility {
    use device_envoy::led_strip;

    led_strip! {
        pub(super) LedStripModulePrivate {
            pin: PIN_7,
            len: 16,
        }
    }

    // Module function that uses the type
    pub fn use_module_type() -> &'static str {
        type _Test = LedStripModulePrivate;
        "accessible from parent"
    }

    // Re-export to parent module (would fail outside crate)
    // pub use LedStripModulePrivate as ExportedToParent;
}

// led_strips! public visibility (strip mode)
mod test_led_strips_public_strip {
    use device_envoy::led_strip::Current;
    use device_envoy::led_strips;

    // Public visibility
    led_strips! {
        pub LedStripsPublicStrip {
            Gpio0LedStrip: { pin: PIN_0, len: 8, max_current: Current::Milliamps(250) }
        }
    }

    pub fn use_public_strip() {
        type _Test = LedStripsPublicStrip;
    }
}

// led_strips! private visibility (strip mode)
mod test_led_strips_private_strip {
    use device_envoy::led_strip::Current;
    use device_envoy::led_strips;

    led_strips! {
        pub(self) LedStripsPrivateStrip {
            Gpio2LedStrip: { pin: PIN_2, len: 24, max_current: Current::Milliamps(500) }
        }
    }

    pub fn use_private_strip() {
        type _Test = LedStripsPrivateStrip;
    }
}

// led_strips! public visibility (led2d mode)
mod test_led_strips_public_led2d {
    use device_envoy::led_strip::Current;
    use device_envoy::led_strips;
    use device_envoy::led2d::Led2dFont;
    use device_envoy::led2d::layout::LedLayout;

    const LED_LAYOUT_PUBLIC: LedLayout<12, 4, 3> = LedLayout::serpentine_column_major();

    led_strips! {
        pio: PIO1,
        pub LedStripsPublicLed2d {
            Gpio4Led2d: {
                pin: PIN_4,
                len: 12,
                max_current: Current::Milliamps(250),
                led2d: {
                    led_layout: LED_LAYOUT_PUBLIC,
                    font: Led2dFont::Font3x4Trim,
                }
            }
        }
    }

    pub fn use_public_led2d() {
        type _Test = LedStripsPublicLed2d;
    }
}

// led_strips! private visibility (led2d mode)
mod test_led_strips_private_led2d {
    use device_envoy::led_strip::Current;
    use device_envoy::led_strips;
    use device_envoy::led2d::Led2dFont;
    use device_envoy::led2d::layout::LedLayout;

    const LED_LAYOUT_PRIVATE: LedLayout<12, 4, 3> = LedLayout::serpentine_column_major();

    led_strips! {
        pub(self) LedStripsPrivateLed2d {
            Gpio6Led2d: {
                pin: PIN_6,
                len: 12,
                max_current: Current::Milliamps(250),
                led2d: {
                    led_layout: LED_LAYOUT_PRIVATE,
                    font: Led2dFont::Font3x4Trim,
                }
            }
        }
    }

    pub fn use_private_led2d() {
        type _Test = LedStripsPrivateLed2d;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_visibility() {
        // Default should be public
        type _Test = LedStripDefault;
    }

    #[test]
    fn test_explicit_pub() {
        // Explicit pub should work
        type _Test = LedStripExplicitPub;
    }

    #[test]
    fn test_pub_crate() {
        // pub(crate) should be accessible within crate
        type _Test = LedStripPubCrate;
    }

    #[test]
    fn test_access_module_export() {
        // Module-exported type should be accessible
        type _Test = test_module::ExportedModulePrivate;
    }

    // Note: LedStripPrivate should NOT be accessible here since it's pub(self)
    // Note: test_module::LedStripModuleDefault should NOT be accessible here
}

// This should compile successfully, demonstrating that all visibility modifiers work correctly
fn main() {
    // Public types should be accessible
    type _Test1 = LedStripDefault;
    type _Test2 = test_explicit_pub::LedStripExplicitPub;
    type _Test3 = test_pub_crate::LedStripPubCrate;

    let _ = test_private::use_private_type();
    let _ = test_module_visibility::use_module_type();

    let _ = test_led_strips_public_strip::use_public_strip();
    let _ = test_led_strips_private_strip::use_private_strip();
    let _ = test_led_strips_public_led2d::use_public_led2d();
    let _ = test_led_strips_private_led2d::use_private_led2d();

    // Private types should NOT be accessible (would cause compile error if uncommented):
    // type _Test4 = test_private::LedStripPrivate;  // Should fail - pub(self) means private
}
