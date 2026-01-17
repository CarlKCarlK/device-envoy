#![no_std]
#![no_main]
#![allow(dead_code)]
//! Test visibility support for led_strip! macro.

use device_kit::led_strip;
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
    use device_kit::led_strip;

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
    use device_kit::led_strip;

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
    use device_kit::led_strip;

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
    use device_kit::led_strip;

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

    // Private types should NOT be accessible (would cause compile error if uncommented):
    // type _Test4 = test_private::LedStripPrivate;  // Should fail - pub(self) means private
}
