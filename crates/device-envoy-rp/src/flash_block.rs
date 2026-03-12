//! A device abstraction for type-safe persistent storage in flash memory.
//!
//! This module provides a generic flash block storage system that allows storing any
//! `serde`-compatible type in Raspberry Pi Pico's internal flash memory.
//!
//! See [`FlashBlockRp`] for details and usage examples.

use core::array;
use core::cell::RefCell;

use defmt::info;
use embassy_rp::Peri;
use embassy_rp::flash::{Blocking, ERASE_SIZE, Flash as EmbassyFlash};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use portable_atomic::{AtomicU32, Ordering};
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use crate::{Error, Result};
use device_envoy_core::flash_block::{
    self as core_flash, FlashBlock as CoreFlashBlock, FlashBlockError, FlashDevice,
};

pub use device_envoy_core::flash_block::FlashBlock;

// Internal flash size for Raspberry Pi Pico 2 (4 MB).
#[cfg(feature = "pico2")]
const INTERNAL_FLASH_SIZE: usize = 4 * 1024 * 1024;

// Internal flash size for Raspberry Pi Pico 1 W (2 MB).
#[cfg(all(not(feature = "pico2"), feature = "pico1"))]
const INTERNAL_FLASH_SIZE: usize = 2 * 1024 * 1024;

// Internal flash size fallback (2 MB).
#[cfg(all(not(feature = "pico2"), not(feature = "pico1")))]
pub const INTERNAL_FLASH_SIZE: usize = 2 * 1024 * 1024;

const TOTAL_BLOCKS: u32 = (INTERNAL_FLASH_SIZE / ERASE_SIZE) as u32;

// Local adapter — wraps EmbassyFlash so core's FlashDevice trait can be implemented
// for a type defined in this crate (required by the orphan rule).
struct RpFlashAdapter<'a>(&'a mut EmbassyFlash<'static, FLASH, Blocking, INTERNAL_FLASH_SIZE>);

impl FlashDevice for RpFlashAdapter<'_> {
    type Error = embassy_rp::flash::Error;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), embassy_rp::flash::Error> {
        self.0.blocking_read(offset, bytes)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), embassy_rp::flash::Error> {
        self.0.blocking_write(offset, bytes)
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), embassy_rp::flash::Error> {
        self.0.blocking_erase(from, to)
    }
}

fn flash_block_error_to_crate_error(e: FlashBlockError<embassy_rp::flash::Error>) -> Error {
    match e {
        FlashBlockError::Io(err) => Error::Flash(err),
        FlashBlockError::FormatError => Error::FormatError,
        FlashBlockError::StorageCorrupted => Error::StorageCorrupted,
    }
}

/// Shared flash manager that owns the hardware driver and allocation cursor.
struct FlashManager {
    flash: Mutex<
        CriticalSectionRawMutex,
        RefCell<EmbassyFlash<'static, FLASH, Blocking, INTERNAL_FLASH_SIZE>>,
    >,
    next_block: AtomicU32,
}

impl FlashManager {
    fn new(peripheral: Peri<'static, FLASH>) -> Self {
        Self {
            flash: Mutex::new(core::cell::RefCell::new(EmbassyFlash::new_blocking(
                peripheral,
            ))),
            next_block: AtomicU32::new(0),
        }
    }

    fn with_flash<R>(
        &self,
        f: impl FnOnce(&mut EmbassyFlash<'static, FLASH, Blocking, INTERNAL_FLASH_SIZE>) -> Result<R>,
    ) -> Result<R> {
        self.flash.lock(|flash| {
            let mut flash_ref = flash.borrow_mut();
            f(&mut *flash_ref)
        })
    }

    fn reserve<const N: usize>(&'static self) -> Result<[FlashBlockRp; N]> {
        let start = self.next_block.fetch_add(N as u32, Ordering::SeqCst);
        let end = start.checked_add(N as u32).ok_or(Error::IndexOutOfBounds)?;
        if end > TOTAL_BLOCKS {
            // rollback
            self.next_block.fetch_sub(N as u32, Ordering::SeqCst);
            return Err(Error::IndexOutOfBounds);
        }
        Ok(array::from_fn(|idx| FlashBlockRp {
            manager: self,
            block: start + idx as u32,
        }))
    }
}

/// A device abstraction for type-safe persistent storage in flash memory.
///
/// `FlashBlockRp` provides a generic flash-block storage system for Raspberry Pi Pico,
/// allowing you to store any `serde`-compatible type in the device's internal flash.
///
/// Use [`FlashBlockRp::new_array`] to allocate one or more blocks. Block operations like
/// [`load`](FlashBlock::load), [`save`](FlashBlock::save), and
/// [`clear`](FlashBlock::clear) are provided by [`FlashBlock`], so bring the trait into
/// scope:
///
/// `use device_envoy_rp::flash_block::FlashBlock as _;`
///
/// # Features
///
/// - **Type safety**: Hash-based type checking prevents reading data written under a
///   different Rust type name. The hash is derived from the full type path
///   (for example, `app1::BootCounter`). **Trying to read a different type
///   returns `Ok(None)`**. Structural changes (adding or removing fields) do not
///   change the hash, but may cause deserialization to fail and return an error.
/// - **Postcard serialization**: A compact, `no_std`-friendly binary format.
///
/// # Block allocation
///
/// Conceptually, flash is treated as an array of fixed-size erase blocks counted from
/// the end of memory backward. Your code can split that array using destructuring
/// assignment and hand individual blocks to subsystems that need persistent storage.
///
/// ⚠️ **Warning**: Pico 1 and Pico 2 store firmware, vector tables, and user data in the
/// same flash device. Allocating too many blocks can overwrite your firmware.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use defmt_rtt as _;
/// # use core::{convert::Infallible, future};
/// use device_envoy_rp::flash_block::{FlashBlockRp, FlashBlock as _};
/// # use defmt::info;
///
/// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
/// struct BootCounter(u8);
///
/// impl BootCounter {
///     const fn new(value: u8) -> Self {
///         Self(value)
///     }
///
///     fn increment(self) -> Self {
///         Self((self.0 + 1) % 10)
///     }
/// }
///
/// async fn example() -> device_envoy_rp::Result<Infallible> {
///     let p = embassy_rp::init(Default::default());
///     let [mut boot_counter_flash_block] = FlashBlockRp::new_array::<1>(p.FLASH)?;
///
///     let boot_counter = boot_counter_flash_block
///         .load()?
///         .unwrap_or(BootCounter::new(0))
///         .increment();
///
///     boot_counter_flash_block.save(&boot_counter)?;
///     info!("Boot counter: {}", boot_counter.0);
///     future::pending().await
/// }
/// ```
///
#[derive(Clone, Copy)]
pub struct FlashBlockRp {
    manager: &'static FlashManager,
    block: u32,
}

impl CoreFlashBlock for FlashBlockRp {
    type Error = Error;

    fn load<T>(&mut self) -> Result<Option<T>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let offset = block_offset(self.block);
        let result = self.manager.with_flash(|flash| {
            let mut adapter = RpFlashAdapter(flash);
            core_flash::load_block::<T, _>(&mut adapter, offset)
                .map_err(flash_block_error_to_crate_error)
        })?;
        if result.is_some() {
            info!("Flash: Loaded data from block {}", self.block);
        } else {
            info!("Flash: No data at block {}", self.block);
        }
        Ok(result)
    }

    fn save<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let offset = block_offset(self.block);
        self.manager.with_flash(|flash| {
            let mut adapter = RpFlashAdapter(flash);
            core_flash::save_block(&mut adapter, offset, value)
                .map_err(flash_block_error_to_crate_error)
        })?;
        info!("Flash: Saved to block {}", self.block);
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        let offset = block_offset(self.block);
        self.manager.with_flash(|flash| {
            let mut adapter = RpFlashAdapter(flash);
            core_flash::clear_block(&mut adapter, offset).map_err(flash_block_error_to_crate_error)
        })?;
        info!("Flash: Cleared block {}", self.block);
        Ok(())
    }
}

impl FlashBlockRp {
    /// Reserve `N` contiguous flash blocks and return them as an array.
    pub fn new_array<const N: usize>(
        peripheral: Peri<'static, FLASH>,
    ) -> Result<[FlashBlockRp; N]> {
        static FLASH_BLOCK_RP_STATIC: FlashBlockRpStatic = FlashBlockRpStatic::new();
        let manager = FLASH_BLOCK_RP_STATIC.manager(peripheral);
        manager.reserve::<N>()
    }
}

/// Static resources for [`FlashBlockRp`].
pub(crate) struct FlashBlockRpStatic {
    manager_cell: StaticCell<FlashManager>,
    manager_ref: Mutex<CriticalSectionRawMutex, core::cell::RefCell<Option<&'static FlashManager>>>,
}

impl FlashBlockRpStatic {
    #[must_use]
    const fn new() -> Self {
        Self {
            manager_cell: StaticCell::new(),
            manager_ref: Mutex::new(core::cell::RefCell::new(None)),
        }
    }

    fn manager(&'static self, peripheral: Peri<'static, FLASH>) -> &'static FlashManager {
        self.manager_ref.lock(|slot_cell| {
            let mut slot = slot_cell.borrow_mut();
            if slot.is_none() {
                let manager_mut = self.manager_cell.init(FlashManager::new(peripheral));
                let manager_ref: &'static FlashManager = manager_mut;
                *slot = Some(manager_ref);
            }
            slot.expect("manager initialized")
        })
    }
}

/// Blocks are allocated from the end of flash backwards.
fn block_offset(block_id: u32) -> u32 {
    let capacity = INTERNAL_FLASH_SIZE as u32;
    capacity - (block_id + 1) * ERASE_SIZE as u32
}
