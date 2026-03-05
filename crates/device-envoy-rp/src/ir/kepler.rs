//! A device abstraction for the SunFounder Kepler Kit IR remote control.
//!
//! See [`IrKeplerRp`] for usage examples.

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::Pin;
use embassy_rp::pio::PioPin;

use device_envoy_core::ir::{IrKepler, IrMapping as _};

use crate::Result;
use crate::ir::IrPioPeripheral;
use crate::ir::mapping::IrMappingRp;

pub use device_envoy_core::ir::kepler::{IrKeplerStatic, KEPLER_MAPPING, KeplerButton};

/// Type alias for the Kepler button mapping.
///
/// See [`IrKeplerRp`] for usage examples.
type IrKeplerMapping<'a> = IrMappingRp<'a, KeplerButton, 21>;

/// A device abstraction for the SunFounder Kepler Kit IR remote.
///
/// This provides a simple interface for the Kepler remote with built-in button mappings.
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_envoy_rp::ir::{IrKepler as _, IrKeplerRp, IrKeplerStatic};
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     static IR_KEPLER_STATIC: IrKeplerStatic = IrKeplerRp::new_static();
///     let ir_kepler = IrKeplerRp::new(&IR_KEPLER_STATIC, p.PIN_15, p.PIO0, spawner)?;
///
///     loop {
///         let button = ir_kepler.wait_for_press().await;
///         defmt::info!("Button: {:?}", button);
///     }
/// }
/// ```
pub struct IrKeplerRp<'a> {
    mapping: IrKeplerMapping<'a>,
}

impl<'a> IrKeplerRp<'a> {
    /// Create static channel resources for IR events.
    ///
    /// See [`IrKeplerRp`] for usage examples.
    #[must_use]
    pub const fn new_static() -> IrKeplerStatic {
        IrKeplerStatic::new()
    }

    /// Create a new Kepler remote handler.
    ///
    /// # Parameters
    /// - `ir_kepler_static`: Static reference to the channel resources
    /// - `pin`: GPIO pin connected to the IR receiver
    /// - `pio`: PIO peripheral to use (PIO0, PIO1, or PIO2)
    /// - `spawner`: Embassy spawner for background task
    ///
    /// See [`IrKeplerRp`] for usage examples.
    ///
    /// # Errors
    /// Returns an error if the background task cannot be spawned.
    pub fn new<P, PIO>(
        ir_kepler_static: &'static IrKeplerStatic,
        pin: Peri<'static, P>,
        pio: Peri<'static, PIO>,
        spawner: Spawner,
    ) -> Result<Self>
    where
        P: Pin + PioPin,
        PIO: IrPioPeripheral,
    {
        let mapping = IrMappingRp::new(ir_kepler_static.inner(), pin, pio, &KEPLER_MAPPING, spawner)?;
        Ok(Self { mapping })
    }

}

impl IrKepler for IrKeplerRp<'_> {
    async fn wait_for_press(&self) -> KeplerButton {
        self.mapping.wait_for_press().await
    }
}
