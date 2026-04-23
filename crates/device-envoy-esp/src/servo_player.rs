//! A device abstraction for servo animation control on ESP LEDC PWM.
//!
//! Use [`servo_player!`] for typed servo players.

/// Sample generated servo-player type documentation.
pub mod servo_player_generated;

/// Combine multiple animation step arrays into one larger array.
///
/// This macro allows combining any number of const arrays with a clean syntax.
///
/// **Syntax:**
///
/// ```text
/// combine!()
/// combine!(<steps_expr>)
/// combine!(<first_steps_expr>, <second_steps_expr>, ... )
/// ```
///
/// See the [servo module documentation](mod@crate::servo) for usage examples.
#[doc(hidden)]
#[macro_export]
macro_rules! combine {
    () => {
        []
    };
    ($single:expr) => {
        $single
    };
    ($first:expr, $second:expr) => {{
        const FIRST: &[(u16, ::embassy_time::Duration)] = &$first;
        const SECOND: &[(u16, ::embassy_time::Duration)] = &$second;
        $crate::servo::combine::<{FIRST.len()}, {SECOND.len()}, {FIRST.len() + SECOND.len()}>($first, $second)
    }};
    ($first:expr, $($rest:expr),+ $(,)?) => {{
        const FIRST: &[(u16, ::embassy_time::Duration)] = &$first;
        const REST: &[(u16, ::embassy_time::Duration)] = &$crate::combine!($($rest),+);
        $crate::servo::combine::<{FIRST.len()}, {REST.len()}, {FIRST.len() + REST.len()}>($first, $crate::combine!($($rest),+))
    }};
}

/// Macro to generate a servo player struct type (includes syntax details).
///
/// This page provides the primary documentation for configuring individual servo players.
///
/// See the [servo module documentation](mod@crate::servo) for complete examples.
///
/// **After reading the configuration details below, see also:**
///
/// - [`servo`](mod@crate::servo) module - Complete examples and usage patterns
///
/// Use this macro when your project has a servo that needs scripted animation control.
/// The macro generates a struct type and spawns a background task to execute
/// animation sequences.
///
/// The macro claims one whole [LEDC](crate#glossary) timer resource and one whole
/// [LEDC](crate#glossary) channel resource for the generated servo player. These
/// resources cannot be shared with other servo players or other [LEDC](crate#glossary)
/// users. If you need multiple servo players, generate separate types with separate
/// timer/channel selections.
///
/// Current [LEDC](crate#glossary) resource counts:
///
/// - ESP32-C6: 4 timers, 6 channels
/// - ESP32-S3: 4 timers, 8 channels
///
/// **Syntax:**
///
/// ```text
/// servo_player! {
///     [<visibility>] <Name> {
///         pin: <pin_ident>,
///         timer: <timer_ident>,
///         channel: <channel_ident>,
///         min_us: <u32_expr>,         // optional
///         max_us: <u32_expr>,         // optional
///         max_degrees: <u16_expr>,    // optional
///         direction: <Direction_expr>, // optional
///         max_steps: <usize_expr>,    // optional
///     }
/// }
/// ```
///
/// # Configuration
///
/// **Required fields:**
///
/// - `pin` - GPIO pin for servo output
/// - `timer` - [LEDC](crate#glossary) timer resource
/// - `channel` - [LEDC](crate#glossary) channel resource
///
/// **Optional fields:**
///
/// - `min_us` - Minimum pulse width in microseconds for 0° (default: `500`)
/// - `max_us` - Maximum pulse width in microseconds for `max_degrees` (default: `2500`)
/// - `max_degrees` - Maximum servo angle in degrees (default: `180`)
/// - `direction` - Logical direction mapping (`Direction::Forward` by default)
/// - `max_steps` - Maximum number of animation steps (default: `16`)
///
/// `max_steps = 0` disables animation and allocates no step storage; `set_degrees()`,
/// `hold()`, and `relax()` are still supported.
///
/// See the [servo module documentation](mod@crate::servo) for details and examples.
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
            pin: $pin:ident,
            timer: $timer:ident,
            channel: $channel:ident,
            $(min_us: $min_us:expr,)?
            $(max_us: $max_us:expr,)?
            $(max_degrees: $max_degrees:expr,)?
            $(direction: $direction:expr,)?
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
                    $crate::__servo_player_impl!(@direction $($direction)?),
                );

            static [<$name:upper _SERVO_PLAYER_STATIC>]:
                $crate::servo::ServoPlayerStatic<{ $crate::__servo_player_impl!(@max_steps $($max_steps)?) }> =
                    $crate::servo::ServoPlayerHandle::<{ $crate::__servo_player_impl!(@max_steps $($max_steps)?) }>::new_static();

            impl $name {
                pub const MAX_STEPS: usize = $crate::__servo_player_impl!(@max_steps $($max_steps)?);

                pub fn new(
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: ::esp_hal::peripherals::$pin<'static>,
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
    (
        $vis:vis $name:ident {
            pin: $pin:ident,
            timer: $timer:ident,
            channel: $channel:ident,
            $(min_us: $min_us:expr,)?
            $(max_us: $max_us:expr,)?
            $(max_degrees: $max_degrees:expr,)?
            $(direction: $direction:expr,)?
            $(max_steps: $max_steps:expr $(,)?)?
        }
    ) => {
        $crate::__paste! {
            $crate::__servo_player_impl! {
                [<__ $name _visibility_inner>] {
                    pin: $pin,
                    timer: $timer,
                    channel: $channel,
                    $(min_us: $min_us,)?
                    $(max_us: $max_us,)?
                    $(max_degrees: $max_degrees,)?
                    $(direction: $direction,)?
                    $(max_steps: $max_steps,)?
                }
            }
            $vis type $name = [<__ $name _visibility_inner>];
        }
    };

    (@min_us $min_us:expr) => { $min_us };
    (@min_us) => { $crate::servo::SERVO_MIN_US_DEFAULT };
    (@max_us $max_us:expr) => { $max_us };
    (@max_us) => { $crate::servo::SERVO_MAX_US_DEFAULT };
    (@max_degrees $max_degrees:expr) => { $max_degrees };
    (@max_degrees) => { $crate::servo::ServoEsp::DEFAULT_MAX_DEGREES };
    (@direction $direction:expr) => { $direction };
    (@direction) => { $crate::servo::Direction::Forward };
    (@max_steps $max_steps:expr) => { $max_steps };
    (@max_steps) => { 16 };
}
