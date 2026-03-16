//! A device abstraction support module for RFID readers.
//!
//! See [`Rfid`] for the trait-based API, and use platform crates
//! (`device_envoy-rp` or `device_envoy-esp`) for hardware-specific constructors/examples.

/// Events received from an RFID reader.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RfidEvent {
    /// A card was detected with a 10-byte UID value.
    CardDetected {
        /// UID bytes, padded with zeros if the physical UID is shorter than 10 bytes.
        uid: [u8; 10],
    },
}

/// Platform-agnostic RFID reader contract.
///
/// Platform crates implement this for concrete `Rfid` types so shared logic can
/// await card-tap events without depending on platform-specific modules.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::rfid::{Rfid, RfidEvent};
///
/// async fn log_card_taps(rfid: &impl Rfid) -> ! {
///     loop {
///         let RfidEvent::CardDetected { uid } = rfid.wait_for_tap().await;
///         let _ = uid;
///     }
/// }
///
/// # struct DemoRfid;
/// # impl Rfid for DemoRfid {
/// #     async fn wait_for_tap(&self) -> RfidEvent {
/// #         RfidEvent::CardDetected { uid: [0; 10] }
/// #     }
/// # }
/// # fn main() {
/// #     let rfid = DemoRfid;
/// #     let _future = log_card_taps(&rfid);
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait Rfid {
    /// Wait for the next RFID event.
    ///
    /// See the [Rfid trait documentation](Self) for usage examples.
    async fn wait_for_tap(&self) -> RfidEvent;
}
