//! A device abstraction for servo animation control on ESP LEDC PWM.
//!
//! Use [`servo_player!`] for typed servo players.
// TODO0 Document LEDC timer/channel ownership protocol and link-time claim behavior in module docs/README.

/// Create a typed servo player with keyword configuration.
#[macro_export]
#[doc(hidden)]
macro_rules! servo_player {
    ($($tt:tt)*) => { $crate::__servo_player_impl! { $($tt)* } };
}
#[doc(inline)]
pub use servo_player;

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_player_impl {
    (
        $name:ident {
            timer: $timer:ident,
            channel: $channel:ident,
            $(min_us: $min_us:expr,)?
            $(max_us: $max_us:expr,)?
            $(max_degrees: $max_degrees:expr,)?
            $(max_steps: $max_steps:expr $(,)?)?
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

            static [<$name:upper _SERVO_STATIC>]: $crate::servo::ServoStatic =
                $crate::servo::ServoStatic::new_static(
                    ::esp_hal::ledc::timer::Number::$timer,
                    ::esp_hal::ledc::channel::Number::$channel,
                    $crate::__servo_player_impl!(@min_us $($min_us)?),
                    $crate::__servo_player_impl!(@max_us $($max_us)?),
                    $crate::__servo_player_impl!(@max_degrees $($max_degrees)?),
                );

            static [<$name:upper _SERVO_PLAYER_STATIC>]:
                $crate::servo::ServoPlayerStatic<{ $crate::__servo_player_impl!(@max_steps $($max_steps)?) }> =
                    $crate::servo::ServoPlayerHandle::<{ $crate::__servo_player_impl!(@max_steps $($max_steps)?) }>::new_static();

            impl $name {
                pub const MAX_STEPS: usize = $crate::__servo_player_impl!(@max_steps $($max_steps)?);

                pub fn new(
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: impl ::esp_hal::gpio::interconnect::PeripheralOutput<'static>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<$crate::servo::ServoPlayerHandle<{ $crate::__servo_player_impl!(@max_steps $($max_steps)?) }>> {
                    let servo = $crate::servo::ServoEsp::new(&[<$name:upper _SERVO_STATIC>], ledc, pin)?;
                    let token = [<__ $name:snake _servo_player_task>](&[<$name:upper _SERVO_PLAYER_STATIC>], servo);
                    spawner.spawn(token)?;
                    Ok($crate::servo::ServoPlayerHandle::new(&[<$name:upper _SERVO_PLAYER_STATIC>]))
                }
            }

            #[::embassy_executor::task]
            async fn [<__ $name:snake _servo_player_task>](
                servo_player_static: &'static $crate::servo::ServoPlayerStatic<{ $crate::__servo_player_impl!(@max_steps $($max_steps)?) }>,
                servo: $crate::servo::ServoEsp,
            ) -> ! {
                $crate::servo::device_loop(servo_player_static, servo).await
            }
        }
    };

    (@min_us $min_us:expr) => { $min_us };
    (@min_us) => { $crate::servo::SERVO_MIN_US_DEFAULT };
    (@max_us $max_us:expr) => { $max_us };
    (@max_us) => { $crate::servo::SERVO_MAX_US_DEFAULT };
    (@max_degrees $max_degrees:expr) => { $max_degrees };
    (@max_degrees) => { $crate::servo::ServoEsp::DEFAULT_MAX_DEGREES };
    (@max_steps $max_steps:expr) => { $max_steps };
    (@max_steps) => { 16 };
}
