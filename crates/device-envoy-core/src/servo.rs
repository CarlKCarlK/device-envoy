//! A device abstraction trait for direct servo control shared across platforms.

#[doc(hidden)]
pub use crate::servo_player::{
    __servo_player_animate, __servo_player_hold, __servo_player_relax, __servo_player_set_degrees,
    device_loop,
};
pub use crate::servo_player::{
    AtEnd, ServoPlayer, ServoPlayerHandle, ServoPlayerStatic, combine, linear,
};

/// Platform-agnostic servo device contract.
///
/// Platform crates implement this trait for their concrete servo types so direct
/// servo operations resolve through trait methods instead of inherent methods.
///
/// This page serves as the definitive reference for direct servo control across
/// platforms.
///
/// # Example
///
/// This example demonstrates basic servo control: move to 45 degrees, then
/// 90 degrees, then relax.
///
/// ```rust,no_run
/// use device_envoy_core::servo::Servo;
/// use embassy_time::{Duration, Timer};
///
/// async fn move_and_relax(servo: &impl Servo) {
///     servo.set_degrees(45);                          // Move to 45 degrees and hold.
///     Timer::after(Duration::from_secs(1)).await;     // Give servo reasonable time to reach position
///     servo.set_degrees(90);                          // Move to 90 degrees and hold.
///     Timer::after(Duration::from_secs(1)).await;     // Give servo reasonable time to reach position
///     servo.relax();                                  // Let the servo relax. It will re-enable on next set_degrees()
/// }
///
/// # struct ServoMock;
/// # impl Servo for ServoMock {
/// #     const DEFAULT_MAX_DEGREES: u16 = 180;
/// #     fn set_degrees(&self, _degrees: u16) {}
/// #     fn hold(&self) {}
/// #     fn relax(&self) {}
/// # }
/// # let servo = ServoMock;
/// # let _future = move_and_relax(&servo);
/// # servo.hold();
/// ```
pub trait Servo {
    /// Default maximum rotation range in degrees.
    const DEFAULT_MAX_DEGREES: u16;

    /// Set position in degrees `0..=max_degrees`.
    ///
    /// See the [Servo trait documentation](Self) for usage examples.
    fn set_degrees(&self, degrees: u16);

    /// Keep driving pulses at the last commanded angle.
    fn hold(&self);

    /// Stop driving pulses.
    ///
    /// See the [Servo trait documentation](Self) for usage examples.
    fn relax(&self);
}
