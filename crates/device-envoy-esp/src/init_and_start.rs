//! Shared startup helpers for Embassy runtime and common ESP peripherals.
//!
//! See [`init_and_start!`] for one-line startup patterns.

/// Shared RMT setup utilities used by generated devices.
pub mod rmt {
    pub use crate::rmt::*;
}

/// Startup mode markers used by [`init_and_start!`].
pub mod rmt_mode {
    pub use crate::rmt_mode::*;
}

/// Initialize the platform and optionally common peripherals.
///
/// Supported forms:
///
/// - `init_and_start!(p)`
/// - `init_and_start!(p, rmt80: rmt80, mode: init_and_start::rmt_mode::Blocking)`
/// - `init_and_start!(p, rmt80: rmt80, mode: init_and_start::rmt_mode::Async)`
/// - `init_and_start!(p, ledc: ledc)`
/// - Any of the above with both `rmt80` and `ledc` in either order.
#[cfg(target_os = "none")]
#[doc(hidden)]
#[macro_export]
macro_rules! init_and_start {
    (@init_ledc $p:ident, $ledc:ident) => {
        let mut $ledc = $crate::esp_hal::ledc::Ledc::new($p.LEDC);
        $ledc.set_global_slow_clock($crate::esp_hal::ledc::LSGlobalClkSource::APBClk);
    };
    ($p:ident, rmt80: $rmt80:ident, mode: init_and_start::rmt_mode::Blocking) => {
        $crate::init_and_start!($p);
        let $rmt80 = $crate::init_and_start::rmt::new_rmt80($p.RMT).expect("RMT init failed");
    };
    ($p:ident, rmt80: $rmt80:ident, mode: init_and_start::rmt_mode::Async) => {
        $crate::init_and_start!($p);
        let $rmt80 =
            $crate::init_and_start::rmt::into_async(
                $crate::init_and_start::rmt::new_rmt80($p.RMT).expect("RMT init failed"),
            );
    };
    ($p:ident, ledc: $ledc:ident) => {
        $crate::init_and_start!($p);
        $crate::init_and_start!(@init_ledc $p, $ledc);
    };
    ($p:ident, rmt80: $rmt80:ident, mode: init_and_start::rmt_mode::Blocking, ledc: $ledc:ident) => {
        $crate::init_and_start!($p, rmt80: $rmt80, mode: init_and_start::rmt_mode::Blocking);
        $crate::init_and_start!(@init_ledc $p, $ledc);
    };
    ($p:ident, rmt80: $rmt80:ident, mode: init_and_start::rmt_mode::Async, ledc: $ledc:ident) => {
        $crate::init_and_start!($p, rmt80: $rmt80, mode: init_and_start::rmt_mode::Async);
        $crate::init_and_start!(@init_ledc $p, $ledc);
    };
    ($p:ident, ledc: $ledc:ident, rmt80: $rmt80:ident, mode: init_and_start::rmt_mode::Blocking) => {
        $crate::init_and_start!($p, rmt80: $rmt80, mode: init_and_start::rmt_mode::Blocking, ledc: $ledc);
    };
    ($p:ident, ledc: $ledc:ident, rmt80: $rmt80:ident, mode: init_and_start::rmt_mode::Async) => {
        $crate::init_and_start!($p, rmt80: $rmt80, mode: init_and_start::rmt_mode::Async, ledc: $ledc);
    };
    ($p:ident) => {
        let $p = $crate::esp_hal::init($crate::esp_hal::Config::default());
        {
            let timg0 = $crate::esp_hal::timer::timg::TimerGroup::new($p.TIMG0);
            #[cfg(target_arch = "riscv32")]
            {
                let sw = $crate::esp_hal::interrupt::software::SoftwareInterruptControl::new(
                    $p.SW_INTERRUPT,
                );
                $crate::esp_rtos::start(timg0.timer0, sw.software_interrupt0);
            }
            #[cfg(target_arch = "xtensa")]
            {
                $crate::esp_rtos::start(timg0.timer0);
            }
        }
    };
}

#[cfg(target_os = "none")]
#[doc(inline)]
pub use init_and_start;
