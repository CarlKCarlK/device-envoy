//! Example of what the [`led_strip!`](crate::led_strip!) macro generates.
//!
//! This module demonstrates the exact struct shape, associated constants, and constructor signature
//! produced by the [`led_strip!`](crate::led_strip!) macro. It uses the macro to generate a real struct
//! so you can inspect the generated API in documentation.
//!
//! [`LedStripGenerated`] is equivalent to writing this macro invocation:
//!
//! ```ignore
//! led_strip! {
//!     LedStripGenerated {
//!         pin: PIN_3,
//!         len: 48,
//!     }
//! }
//! ```
//!
//! # Generated Members
//!
//! - `const LEN: usize = 48` — The number of LEDs in the strip
//! - `const MAX_BRIGHTNESS: u8` — Maximum brightness (limited by power budget, see [`Current`](crate::led_strip::Current))
//! - `async fn new(pin, pio, dma, spawner) -> Result<Self>` — Constructor that sets up the LED strip

use crate::led_strip;

led_strip! {
    LedStripGenerated {
        pin: PIN_3,
        len: 48,
    }
}
