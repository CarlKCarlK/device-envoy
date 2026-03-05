//! A device abstraction for mapping IR remote buttons to application-specific actions.
//!
//! See [`IrMappingRp`] for usage examples.

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::Pin;
use embassy_rp::pio::PioPin;

use crate::Result;
use crate::ir::{IrMapping, IrMappingAdapter, IrPioPeripheral, IrRp};

pub use device_envoy_core::ir::mapping::IrMappingStatic;

/// A generic device abstraction that maps IR remote button presses to user-defined button types.
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::ir::{IrMapping as _, IrMappingRp, IrMappingStatic};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
/// #[derive(Debug, Clone, Copy)]
/// enum RemoteButton { Power, Play, Stop }
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     let button_map = [
///         (0x0000, 0x45, RemoteButton::Power),
///         (0x0000, 0x0C, RemoteButton::Play),
///         (0x0000, 0x08, RemoteButton::Stop),
///     ];
///
///     static IR_MAPPING_STATIC: IrMappingStatic = IrMappingRp::<RemoteButton, 3>::new_static();
///     let ir_mapping: IrMappingRp<RemoteButton, 3> = IrMappingRp::new(&IR_MAPPING_STATIC, p.PIN_15, p.PIO0, &button_map, spawner)?;
///
///     loop {
///         let button = ir_mapping.wait_for_press().await;
///         // Use button...
///     }
/// }
/// ```
pub struct IrMappingRp<'a, B, const N: usize> {
    mapping: IrMappingAdapter<IrRp<'a>, B, N>,
}

impl<'a, B, const N: usize> IrMappingRp<'a, B, N>
where
    B: Copy,
{
    /// Create static channel resources for IR mapping events.
    ///
    /// See [`IrMappingRp`] for usage examples.
    #[must_use]
    pub const fn new_static() -> IrMappingStatic {
        IrMappingStatic::new()
    }

    /// Create a new IR remote button mapper.
    ///
    /// # Parameters
    /// - `ir_mapping_static`: Static reference to the channel resources
    /// - `pin`: GPIO pin connected to the IR receiver
    /// - `pio`: PIO peripheral to use (PIO0, PIO1, or PIO2)
    /// - `button_map`: Array mapping (address, command) pairs to button types
    /// - `spawner`: Embassy spawner for background task
    ///
    /// See [`IrMappingRp`] for usage examples.
    ///
    /// # Errors
    /// Returns an error if the background task cannot be spawned.
    pub fn new<P, PIO>(
        ir_mapping_static: &'static IrMappingStatic,
        pin: Peri<'static, P>,
        pio: Peri<'static, PIO>,
        button_map: &[(u16, u8, B)],
        spawner: Spawner,
    ) -> Result<Self>
    where
        P: Pin + PioPin,
        PIO: IrPioPeripheral,
    {
        let ir = IrRp::new(ir_mapping_static.inner(), pin, pio, spawner)?;
        let mapping = IrMappingAdapter::new(ir, button_map);
        Ok(Self { mapping })
    }
}

impl<B, const N: usize> IrMapping<B> for IrMappingRp<'_, B, N>
where
    B: Copy,
{
    async fn wait_for_press(&self) -> B {
        self.mapping.wait_for_press().await
    }
}
