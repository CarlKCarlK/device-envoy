//! Platform-agnostic servo control trait shared across platforms.

/// Platform-agnostic servo device contract.
///
/// Platform crates implement this trait for their concrete servo types so direct
/// servo operations resolve through trait methods instead of inherent methods.
pub trait Servo {
    /// Default maximum rotation range in degrees.
    const DEFAULT_MAX_DEGREES: u16;

    /// Set position in degrees `0..=max_degrees`.
    fn set_degrees(&self, degrees: u16);

    /// Keep driving pulses at the last commanded angle.
    fn hold(&self);

    /// Stop driving pulses.
    fn relax(&self);
}
