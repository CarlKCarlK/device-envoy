//! Shared startup helpers for Embassy runtime and common ESP peripherals.
//!
//! See [`init_and_start!`] for one-line startup patterns.

/// Shared RMT setup utilities used by generated devices.
#[cfg(esp_has_rmt)]
pub mod rmt {
    pub use crate::rmt::*;
}

/// Creates the LEDC (PWM) peripheral that `servo!` and `servo_player!` take, clocked for
/// servo timing.
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use esp_backtrace as _;
/// use device_envoy_esp::init_and_start;
///
/// # fn example() {
/// init_and_start!(p);
/// let ledc = device_envoy_esp::new_ledc(p.LEDC);
/// # }
/// ```
#[cfg(all(target_os = "none", esp_has_ledc))]
#[must_use]
pub fn new_ledc(ledc: esp_hal::peripherals::LEDC<'static>) -> esp_hal::ledc::Ledc<'static> {
    let mut ledc = esp_hal::ledc::Ledc::new(ledc);
    ledc.set_global_slow_clock(esp_hal::ledc::LSGlobalClkSource::APBClk);
    ledc
}

/// Initializes the chip and starts the async runtime, binding the remaining peripherals to
/// the name you give.
///
/// It consumes `TIMG0` and `SW_INTERRUPT` to start the runtime; every other peripheral stays
/// available on the binding. Create the peripherals that devices share with functions:
/// [`new_rmt80`](crate::new_rmt80) for LED strips and IR receivers (call `.into_async()` on
/// the result where a device needs async RMT), and [`new_ledc`](crate::new_ledc) for servos.
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use esp_backtrace as _;
/// use device_envoy_esp::init_and_start;
///
/// # fn example() -> device_envoy_esp::Result<()> {
/// init_and_start!(p);
/// let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;
/// # Ok(())
/// # }
/// ```
#[cfg(target_os = "none")]
#[doc(hidden)]
#[macro_export]
macro_rules! init_and_start {
    ($p:ident) => {
        let $p = $crate::esp_hal::init($crate::esp_hal::Config::default());
        {
            let timg0 = $crate::esp_hal::timer::timg::TimerGroup::new($p.TIMG0);
            let sw = $crate::esp_hal::interrupt::software::SoftwareInterruptControl::new(
                $p.SW_INTERRUPT,
            );
            $crate::esp_rtos::start(timg0.timer0, sw.software_interrupt0);
        }
    };
}

#[cfg(target_os = "none")]
#[doc(inline)]
pub use init_and_start;
