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

/// Code generator for [`servo_player!`](crate::servo::servo_player).
///
/// Called only by `servo_player!` after its `const_structures::define!` schema has validated
/// the input and filled defaults. Must be public for macro expansion in downstream
/// crates, but not user-facing API.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_player_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        timer: $timer:ident,
        channel: $channel:ident,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        direction: $direction:expr,
        max_steps: $max_steps:expr,
    ) => {
        $crate::servo::paste::paste! {
            $(#[$attr])*
            #[doc = $doc]
            $vis struct $name;

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
                    $min_us,
                    $max_us,
                    $max_degrees,
                    $direction,
                );

            static [<$name:upper _SERVO_PLAYER_STATIC>]:
                $crate::servo::ServoPlayerStatic<{ $max_steps }> =
                    $crate::servo::ServoPlayerHandle::<{ $max_steps }>::new_static();

            impl $name {
                pub const MAX_STEPS: usize = $max_steps;

                pub fn new(
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: ::esp_hal::peripherals::$pin<'static>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<$crate::servo::ServoPlayerHandle<{ $max_steps }>> {
                    let servo = $crate::servo::ServoEsp::new(&[<$name:upper _SERVO_STATIC>], ledc, pin)?;
                    let token = [<__ $name:snake _servo_player_task>](&[<$name:upper _SERVO_PLAYER_STATIC>], servo);
                    spawner.spawn(token?);
                    Ok($crate::servo::ServoPlayerHandle::new(&[<$name:upper _SERVO_PLAYER_STATIC>]))
                }
            }

            #[::embassy_executor::task]
            async fn [<__ $name:snake _servo_player_task>](
                servo_player_static: &'static $crate::servo::ServoPlayerStatic<{ $max_steps }>,
                servo: $crate::servo::ServoEsp,
            ) -> ! {
                $crate::servo::device_loop(servo_player_static, servo).await
            }
        }
    };
}
const_structures::define! {
    /// Macro to generate a servo player struct type.
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
    /// [LEDC](crate#glossary) timer/channel availability is chip and board dependent.
    /// Use the capability/board-profile configuration as source of truth for current limits.
    ///
    /// `max_steps = 0` disables animation and allocates no step storage; `set_degrees()`,
    /// `hold()`, and `relax()` are still supported.
    ///
    /// See the [servo module documentation](mod@crate::servo) for details and examples.
    pub servo_player => __servo_player_generate {
        /// GPIO pin for servo output, for example `GPIO10`.
        pin: ident,
        /// LEDC timer, for example `Timer0`; claimed exclusively for the whole binary.
        timer: ident,
        /// LEDC channel, for example `Channel0`; claimed exclusively for the whole binary.
        channel: ident,
        /// Minimum pulse width in microseconds, for 0°.
        #[default_display = "500"]
        min_us: expr = $crate::servo::SERVO_MIN_US_DEFAULT,
        /// Maximum pulse width in microseconds, for `max_degrees`.
        #[default_display = "2500"]
        max_us: expr = $crate::servo::SERVO_MAX_US_DEFAULT,
        /// Maximum servo angle in degrees.
        #[default_display = "180"]
        max_degrees: expr = $crate::servo::ServoEsp::DEFAULT_MAX_DEGREES,
        /// Logical angle direction.
        #[default_display = "Direction::Forward"]
        direction: expr = $crate::servo::Direction::Forward,
        /// Maximum number of animation steps.
        max_steps: expr = 16,
    }
}
