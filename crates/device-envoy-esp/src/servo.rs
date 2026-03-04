//! A device abstraction for hobby servos on ESP LEDC PWM.
//!
//! Use [`servo!`] for a keyword-driven typed constructor.

use crate::Result;
use esp_hal::gpio::{interconnect::PeripheralOutput, DriveMode};
use esp_hal::ledc::{channel, timer, LowSpeed};
use esp_hal::ledc::{channel::ChannelIFace, timer::TimerIFace};
use esp_hal::time::Rate;
use static_cell::StaticCell;

const SERVO_PERIOD_US: u32 = 20_000;

/// Default minimum pulse width for hobby servos (microseconds).
pub const SERVO_MIN_US_DEFAULT: u32 = 500;

/// Default maximum pulse width for hobby servos (microseconds).
pub const SERVO_MAX_US_DEFAULT: u32 = 2_500;

/// LEDC-backed servo static resources and motion configuration.
pub struct ServoStatic {
    timer: StaticCell<timer::Timer<'static, LowSpeed>>,
    channel: StaticCell<channel::Channel<'static, LowSpeed>>,
    timer_number: timer::Number,
    channel_number: channel::Number,
    min_us: u32,
    max_us: u32,
    max_degrees: u16,
}

impl ServoStatic {
    /// Create static resources for one servo output.
    #[must_use]
    pub const fn new_static(
        timer_number: timer::Number,
        channel_number: channel::Number,
        min_us: u32,
        max_us: u32,
        max_degrees: u16,
    ) -> Self {
        assert!(min_us < max_us, "min_us must be less than max_us");
        assert!(max_degrees > 0, "max_degrees must be positive");
        Self {
            timer: StaticCell::new(),
            channel: StaticCell::new(),
            timer_number,
            channel_number,
            min_us,
            max_us,
            max_degrees,
        }
    }
}

/// A direct servo output using one LEDC timer and one LEDC channel.
pub struct Servo {
    channel: &'static mut channel::Channel<'static, LowSpeed>,
    min_us: u32,
    max_us: u32,
    max_degrees: u16,
}

impl Servo {
    /// Default maximum rotation range in degrees.
    pub const DEFAULT_MAX_DEGREES: u16 = 180;

    /// Create a servo from static resources and a GPIO output pin.
    pub fn new(
        servo_static: &'static ServoStatic,
        ledc: &esp_hal::ledc::Ledc<'static>,
        pin: impl PeripheralOutput<'static>,
    ) -> Result<Self> {
        let timer = servo_static
            .timer
            .init(ledc.timer::<LowSpeed>(servo_static.timer_number));
        timer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(50),
        })?;

        let channel = servo_static
            .channel
            .init(ledc.channel(servo_static.channel_number, pin));
        channel.configure(channel::config::Config {
            timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })?;

        Ok(Self {
            channel,
            min_us: servo_static.min_us,
            max_us: servo_static.max_us,
            max_degrees: servo_static.max_degrees,
        })
    }

    /// Set position in degrees `0..=max_degrees`.
    pub fn set_degrees(&mut self, degrees: u16) -> Result<()> {
        assert!(degrees <= self.max_degrees);
        let duty_pct = self.degrees_to_duty_pct(degrees);
        self.channel.set_duty(duty_pct)?;
        Ok(())
    }

    /// Keep driving pulses at the last commanded angle.
    pub fn hold(&mut self) {}

    /// Stop driving pulses.
    pub fn relax(&mut self) -> Result<()> {
        self.channel.set_duty(0)?;
        Ok(())
    }

    fn pulse_for_degrees(&self, degrees: u16) -> u32 {
        let pulse_span = self.max_us - self.min_us;
        self.min_us
            + (u32::from(degrees) * pulse_span + u32::from(self.max_degrees / 2))
                / u32::from(self.max_degrees)
    }

    fn degrees_to_duty_pct(&self, degrees: u16) -> u8 {
        let pulse_us = self.pulse_for_degrees(degrees);
        let duty_pct = ((pulse_us * 100) + (SERVO_PERIOD_US / 2)) / SERVO_PERIOD_US;
        assert!(duty_pct <= u8::MAX as u32);
        duty_pct as u8
    }
}

#[doc(hidden)]
pub use paste;

/// Create a typed servo constructor with keyword configuration.
#[macro_export]
#[doc(hidden)]
macro_rules! servo {
    ($($tt:tt)*) => { $crate::__servo_impl! { $($tt)* } };
}
#[doc(inline)]
pub use servo;

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_impl {
    (
        $name:ident {
            timer: $timer:ident,
            channel: $channel:ident,
            $(min_us: $min_us:expr,)?
            $(max_us: $max_us:expr,)?
            $(max_degrees: $max_degrees:expr $(,)?)?
        }
    ) => {
        $crate::servo::paste::paste! {
            pub struct $name;

            // Link-time ownership claims: duplicate timer or channel selection across the
            // final binary should fail the link with duplicate symbol errors.
            #[used]
            #[unsafe(no_mangle)]
            static [<__device_envoy_esp_ledc_timer_claim_ $timer:lower>]: u8 = 0;

            #[used]
            #[unsafe(no_mangle)]
            static [<__device_envoy_esp_ledc_channel_claim_ $channel:lower>]: u8 = 0;

            static [<$name:upper _SERVO_STATIC>]: [<$name Static>] = $name::new_static();

            pub struct [<$name Static>] {
                servo_static: $crate::servo::ServoStatic,
            }

            impl $name {
                #[must_use]
                pub const fn new_static() -> [<$name Static>] {
                    [<$name Static>] {
                        servo_static: $crate::servo::ServoStatic::new_static(
                            ::esp_hal::ledc::timer::Number::$timer,
                            ::esp_hal::ledc::channel::Number::$channel,
                            $crate::__servo_impl!(@min_us $($min_us)?),
                            $crate::__servo_impl!(@max_us $($max_us)?),
                            $crate::__servo_impl!(@max_degrees $($max_degrees)?),
                        ),
                    }
                }

                pub fn new(
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: impl ::esp_hal::gpio::interconnect::PeripheralOutput<'static>,
                ) -> $crate::Result<$crate::servo::Servo> {
                    $crate::servo::Servo::new(&[<$name:upper _SERVO_STATIC>].servo_static, ledc, pin)
                }
            }
        }
    };

    (@min_us $min_us:expr) => { $min_us };
    (@min_us) => { $crate::servo::SERVO_MIN_US_DEFAULT };
    (@max_us $max_us:expr) => { $max_us };
    (@max_us) => { $crate::servo::SERVO_MAX_US_DEFAULT };
    (@max_degrees $max_degrees:expr) => { $max_degrees };
    (@max_degrees) => { $crate::servo::Servo::DEFAULT_MAX_DEGREES };
}
