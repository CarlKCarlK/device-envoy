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
    pub esp_button_watch as button_watch => ::device_envoy_esp::__button_watch_generate {
        /// GPIO pin connected to the button, for example `GPIO6`.
        pin: ident,
    }
}
