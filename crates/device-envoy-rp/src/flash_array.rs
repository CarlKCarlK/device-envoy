//! A device abstraction for type-safe persistent storage in flash memory.
//!
//! This module provides a generic flash block storage system that allows storing any
//! `serde`-compatible type in Raspberry Pi Pico's internal flash memory.
//!
//! See [`FlashArray`] for details and usage examples.

use core::array;
use core::cell::RefCell;
use crc32fast::Hasher;
use defmt::{error, info};
use embassy_rp::Peri;
use embassy_rp::flash::{Blocking, ERASE_SIZE, Flash as EmbassyFlash};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use portable_atomic::{AtomicU32, Ordering};
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use crate::{Error, Result};

// Internal flash size for Raspberry Pi Pico 2 (4 MB).
#[cfg(feature = "pico2")]
const INTERNAL_FLASH_SIZE: usize = 4 * 1024 * 1024;

// Internal flash size for Raspberry Pi Pico 1 W (2 MB).
#[cfg(all(not(feature = "pico2"), feature = "pico1"))]
const INTERNAL_FLASH_SIZE: usize = 2 * 1024 * 1024;

// Internal flash size fallback (2 MB).
#[cfg(all(not(feature = "pico2"), not(feature = "pico1")))]
pub const INTERNAL_FLASH_SIZE: usize = 2 * 1024 * 1024;

const MAGIC: u32 = 0x424C_4B53; // 'BLKS'
const HEADER_SIZE: usize = 4 + 4 + 2; // Magic + TypeHash + PayloadLen
const CRC_SIZE: usize = 4;
const MAX_PAYLOAD_SIZE: usize = ERASE_SIZE - HEADER_SIZE - CRC_SIZE; // 3900 bytes
const TOTAL_BLOCKS: u32 = (INTERNAL_FLASH_SIZE / ERASE_SIZE) as u32;

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

    fn reserve<const N: usize>(&'static self) -> Result<[FlashBlock; N]> {
        let start = self.next_block.fetch_add(N as u32, Ordering::SeqCst);
        let end = start.checked_add(N as u32).ok_or(Error::IndexOutOfBounds)?;
        if end > TOTAL_BLOCKS {
            // rollback
            self.next_block.fetch_sub(N as u32, Ordering::SeqCst);
            return Err(Error::IndexOutOfBounds);
        }
        Ok(array::from_fn(|idx| FlashBlock {
            manager: self,
            block: start + idx as u32,
        }))
    }
}

/// Type of a [`FlashArray`] block, with methods such as [`load`](Self::load), [`save`](Self::save), and [`clear`](Self::clear).
///
/// See [`FlashArray`] for usage examples.
pub struct FlashBlock {
    manager: &'static FlashManager,
    block: u32,
}

impl FlashBlock {
    /// Load data stored in this block.
    ///
    /// See [`FlashArray`] for usage examples.
    pub fn load<T>(&mut self) -> Result<Option<T>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        load_block(self.manager, self.block)
    }

    /// Save data to this block.
    ///
    /// See [`FlashArray`] for usage examples.
    pub fn save<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        save_block(self.manager, self.block, value)
    }

    /// Clear this block.
    pub fn clear(&mut self) -> Result<()> {
        clear_block(self.manager, self.block)
    }
}

/// Static resources for [`FlashArray`].
pub(crate) struct FlashArrayStatic {
    manager_cell: StaticCell<FlashManager>,
    manager_ref: Mutex<CriticalSectionRawMutex, core::cell::RefCell<Option<&'static FlashManager>>>,
}

impl FlashArrayStatic {
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

/// A device abstraction for type-safe persistent storage in flash memory.
///
/// This struct provides a generic flash-block storage system for Raspberry Pi Pico,
/// allowing you to store any `serde`-compatible type in the device’s internal flash.
///
/// You choose the number of storage blocks at compile time. Each block holds up to
/// 3900 bytes of postcard-serialized data (a hardware-determined 4 KB flash block
/// minus metadata space).
///
/// # Features
///
/// - **Type safety**: Hash-based type checking prevents reading data written under a
///   different Rust type name. The hash is derived from the full type path
///   (for example, `app1::BootCounter`). **Trying to read a different types
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
/// use device_envoy::flash_array::FlashArray;
/// # use defmt::info;
///
/// /// Boot counter (newtype) that wraps at 10.
/// /// Stored with `postcard` (Serde).
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
/// async fn example() -> device_envoy::Result<Infallible> {
///     let p = embassy_rp::init(Default::default());
///
///     // Create a flash array. You can destructure it however you like.
///     let [mut boot_counter_flash_block] = FlashArray::<1>::new(p.FLASH)?;
///
///     // Read boot counter from flash then increment.
///     // FlashArray includes a runtime type hash so values are only loaded
///     // if the stored type matches the requested type; mismatches yield `None`.
///     let boot_counter = boot_counter_flash_block
///         .load()?
///         .unwrap_or(BootCounter::new(0)) // Default to 0 type not present
///         .increment();
///
///     // Write incremented counter back to flash.
///     // This example writes once per power-up (fine for a demo; don't write in a tight loop).
///     // Flash is typically good for ~100K erase cycles per block.
///     boot_counter_flash_block.save(&boot_counter)?;
///
///     info!("Boot counter: {}", boot_counter.0);
///     future::pending().await // Keep running
/// }
/// ```
pub struct FlashArray<const N: usize>;

impl<const N: usize> FlashArray<N> {
    /// Reserve `N` contiguous blocks and return them as an array that you can destructure however you like.
    ///
    /// See [`FlashArray`] for usage examples.
    pub fn new(peripheral: Peri<'static, FLASH>) -> Result<[FlashBlock; N]> {
        static FLASH_STATIC: FlashArrayStatic = FlashArrayStatic::new();
        let manager = FLASH_STATIC.manager(peripheral);
        manager.reserve::<N>()
    }
}

fn save_block<T>(manager: &'static FlashManager, block: u32, value: &T) -> Result<()>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut payload_buffer = [0u8; MAX_PAYLOAD_SIZE];
    let payload_len = postcard::to_slice(value, &mut payload_buffer)
        .map_err(|_| {
            error!(
                "Flash: Serialization failed or data too large (max {} bytes)",
                MAX_PAYLOAD_SIZE
            );
            Error::FormatError
        })?
        .len();

    let mut buffer = [0xFFu8; ERASE_SIZE];
    buffer[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    buffer[4..8].copy_from_slice(&compute_type_hash::<T>().to_le_bytes());
    buffer[8..10].copy_from_slice(&(payload_len as u16).to_le_bytes());
    buffer[HEADER_SIZE..HEADER_SIZE + payload_len].copy_from_slice(&payload_buffer[..payload_len]);

    let crc_offset = HEADER_SIZE + payload_len;
    let crc = compute_crc(&buffer[0..crc_offset]);
    buffer[crc_offset..crc_offset + CRC_SIZE].copy_from_slice(&crc.to_le_bytes());

    let offset = block_offset(block);
    manager.with_flash(|flash| {
        flash
            .blocking_erase(offset, offset + ERASE_SIZE as u32)
            .map_err(Error::Flash)?;
        flash
            .blocking_write(offset, &buffer)
            .map_err(Error::Flash)?;
        Ok(())
    })?;

    info!("Flash: Saved {} bytes to block {}", payload_len, block);
    Ok(())
}

fn load_block<T>(manager: &'static FlashManager, block: u32) -> Result<Option<T>>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let offset = block_offset(block);
    let mut buffer = [0u8; ERASE_SIZE];

    manager.with_flash(|flash| {
        flash
            .blocking_read(offset, &mut buffer)
            .map_err(Error::Flash)?;
        Ok(())
    })?;

    let magic = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
    if magic != MAGIC {
        info!("Flash: No data at block {}", block);
        return Ok(None);
    }

    let stored_type_hash = u32::from_le_bytes(buffer[4..8].try_into().unwrap());
    let expected_type_hash = compute_type_hash::<T>();
    if stored_type_hash != expected_type_hash {
        info!(
            "Flash: Type mismatch at block {} (expected hash {}, found {})",
            block, expected_type_hash, stored_type_hash
        );
        return Ok(None);
    }

    let payload_len = u16::from_le_bytes(buffer[8..10].try_into().unwrap()) as usize;
    if payload_len > MAX_PAYLOAD_SIZE {
        error!(
            "Flash: Invalid payload length {} at block {}",
            payload_len, block
        );
        return Err(Error::StorageCorrupted);
    }

    let crc_offset = HEADER_SIZE + payload_len;
    let stored_crc = u32::from_le_bytes(
        buffer[crc_offset..crc_offset + CRC_SIZE]
            .try_into()
            .unwrap(),
    );
    let computed_crc = compute_crc(&buffer[0..crc_offset]);
    if stored_crc != computed_crc {
        error!(
            "Flash: CRC mismatch at block {} (expected {}, found {})",
            block, computed_crc, stored_crc
        );
        return Err(Error::StorageCorrupted);
    }

    let payload = &buffer[HEADER_SIZE..HEADER_SIZE + payload_len];
    let value: T = postcard::from_bytes(payload).map_err(|_| {
        error!("Flash: Deserialization failed at block {}", block);
        Error::StorageCorrupted
    })?;

    info!("Flash: Loaded data from block {}", block);
    Ok(Some(value))
}

fn clear_block(manager: &'static FlashManager, block: u32) -> Result<()> {
    let offset = block_offset(block);
    manager.with_flash(|flash| {
        flash
            .blocking_erase(offset, offset + ERASE_SIZE as u32)
            .map_err(Error::Flash)?;
        Ok(())
    })?;
    info!("Flash: Cleared block {}", block);
    Ok(())
}

/// Blocks are allocated from the end of flash backwards.
fn block_offset(block_id: u32) -> u32 {
    let capacity = INTERNAL_FLASH_SIZE as u32;
    capacity - (block_id + 1) * ERASE_SIZE as u32
}

/// Compute FNV-1a hash of the type name for type safety.
fn compute_type_hash<T>() -> u32 {
    const FNV_PRIME: u32 = 16_777_619;
    const FNV_OFFSET: u32 = 2_166_136_261;

    let type_name = core::any::type_name::<T>();
    let mut hash = FNV_OFFSET;

    for byte in type_name.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Compute CRC32 checksum.
fn compute_crc(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}
