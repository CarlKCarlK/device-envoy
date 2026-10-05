//! Declaration macros for `device-envoy-rp` and `device-envoy-esp`.
//!
//! Each macro is declared once from a schema. The platform crates re-export
//! them under their user-facing names (for example
//! `pub use device_envoy_macros::rp_button_watch as button_watch;`) and add the
//! hand-written docs and examples there. Do not depend on this crate directly.

const_structures::define! {
    pub rp_button_watch as button_watch => ::device_envoy_rp::__button_watch_generate {
        /// GPIO pin connected to the button, for example `PIN_13`.
        pin: ident,
    }
}

const_structures::define! {
    pub rp_led_strips as led_strips => ::device_envoy_rp::__led_strips_generate {
        /// PIO resource shared by every strip and panel in the group.
        pio: ident = PIO0,
        /// Each member is one LED strip or 2D panel and uses one PIO state machine.
        members 1..=4 {
            /// GPIO pin for LED data, for example `PIN_0`.
            pin: ident,
            /// Number of LEDs (pixels).
            len: expr,
            /// Electrical current budget, for example `Current::Milliamps(250)`.
            max_current: expr,
            /// DMA channel.
            dma: ident = by_index[DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3],
            /// Color correction curve.
            #[default_display = "Gamma::Srgb"]
            gamma: expr = ::device_envoy_rp::led_strip::Gamma::Srgb,
            /// Maximum number of animation frames; `0` disables animation.
            max_frames: expr = 16,
            /// Makes this member a 2D panel.
            led2d?: {
                /// Physical layout; a `const` `LedLayout` that defines the panel size.
                led_layout: expr,
                /// Built-in font for text, for example `Led2dFont::Font4x6Trim`.
                font: expr,
            },
        },
    }
}

const_structures::define! {
    pub rp_led_strip as led_strip => ::device_envoy_rp::__led_strip_generate {
        /// GPIO pin for LED data, for example `PIN_0`.
        pin: ident,
        /// Number of LEDs (pixels).
        len: expr,
        /// PIO resource.
        pio: ident = PIO0,
        /// DMA channel.
        dma: ident = DMA_CH0,
        /// Electrical current budget.
        #[default_display = "Current::Milliamps(250)"]
        max_current: expr = ::device_envoy_rp::led_strip::MAX_CURRENT_DEFAULT,
        /// Color correction curve.
        #[default_display = "Gamma::Srgb"]
        gamma: expr = ::device_envoy_rp::led_strip::Gamma::Srgb,
        /// Maximum number of animation frames; `0` disables animation.
        max_frames: expr = 16,
    }
}

const_structures::define! {
    pub rp_led2d as led2d => ::device_envoy_rp::__led2d_generate {
        /// GPIO pin for LED data, for example `PIN_4`.
        pin: ident,
        /// Physical layout; a `const` `LedLayout` that defines the panel size.
        led_layout: expr,
        /// Built-in font for text, for example `Led2dFont::Font4x6Trim`.
        font: expr,
        /// PIO resource.
        pio: ident = PIO0,
        /// DMA channel.
        dma: ident = DMA_CH0,
        /// Electrical current budget.
        #[default_display = "Current::Milliamps(250)"]
        max_current: expr = ::device_envoy_rp::led_strip::MAX_CURRENT_DEFAULT,
        /// Color correction curve.
        #[default_display = "Gamma::Srgb"]
        gamma: expr = ::device_envoy_rp::led_strip::Gamma::Srgb,
        /// Maximum number of animation frames; `0` disables animation.
        max_frames: expr = 16,
    }
}

const_structures::define! {
    pub esp_button_watch as button_watch => ::device_envoy_esp::__button_watch_generate {
        /// GPIO pin connected to the button, for example `GPIO6`.
        pin: ident,
    }
}
