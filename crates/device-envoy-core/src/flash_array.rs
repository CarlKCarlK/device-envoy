//! Shared low-level flash block protocol for type-safe persistent storage.
//!
//! This module provides the platform-independent protocol layer for
//! [`device_envoy_rp::flash_array`] and `device_envoy_esp::flash_array`. See those
//! platform crates for constructors, hardware wiring, and usage examples.

use core::any::type_name;

use crc32fast::Hasher;
use serde::{Deserialize, Serialize};

/// Magic number identifying a valid flash block: `'BLKS'`.
pub const MAGIC: u32 = 0x424C_4B53;

/// Number of bytes in the block header: magic(4) + type\_hash(4) + payload\_len(2).
pub const HEADER_SIZE: usize = 10;

/// Number of bytes used by the CRC trailer.
pub const CRC_SIZE: usize = 4;

/// Size of one flash erase block in bytes.
pub const FLASH_BLOCK_SIZE: usize = 4096;

/// [`FLASH_BLOCK_SIZE`] as a `u32`.
pub const FLASH_BLOCK_SIZE_U32: u32 = FLASH_BLOCK_SIZE as u32;

/// Maximum number of payload bytes that fit in one block.
pub const MAX_PAYLOAD_SIZE: usize = FLASH_BLOCK_SIZE - HEADER_SIZE - CRC_SIZE;

/// Errors returned by [`save_block`], [`load_block`], and [`clear_block`].
#[derive(Debug)]
pub enum FlashBlockError<E> {
    /// An I/O operation on the underlying flash device failed.
    Io(E),
    /// Serialization or deserialization failed.
    FormatError,
    /// The stored data is corrupt (bad CRC or invalid length).
    StorageCorrupted,
}

/// Canonical typed block operations for flash-backed persistence.
///
/// Platform crates implement this trait on their concrete flash block handle
/// types (for example, `device_envoy_rp::flash_array::FlashBlock`).
///
/// Constructors and hardware wiring remain platform-specific; this trait
/// defines the shared operation surface used by higher-level abstractions.
pub trait FlashBlock {
    /// Error returned by block operations.
    type Error;

    /// Load a typed value from this block.
    ///
    /// Returns `Ok(None)` when the block is empty or contains a different type.
    fn load<T>(&mut self) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>;

    /// Save a typed value to this block.
    fn save<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>;

    /// Clear this block.
    fn clear(&mut self) -> Result<(), Self::Error>;
}

/// Low-level read/write/erase interface for a flash device.
///
/// Implement this trait in the platform crate to connect the shared block
/// protocol to the hardware driver.
pub trait FlashDevice {
    /// The error type returned by I/O operations.
    type Error;

    /// Read `bytes.len()` bytes starting at `offset`.
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// Write `bytes` starting at `offset`.
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Erase flash from `from` (inclusive) to `to` (exclusive), in bytes.
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
}

/// Serialize `value` and write it into the block starting at `block_offset`.
///
/// The block is erased before writing. On success the block contains:
/// magic + type hash + payload length + serialized payload + CRC32.
pub fn save_block<T, F>(
    flash: &mut F,
    block_offset: u32,
    value: &T,
) -> Result<(), FlashBlockError<F::Error>>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FlashDevice,
{
    let mut payload_buffer = [0u8; MAX_PAYLOAD_SIZE];
    let payload =
        postcard::to_slice(value, &mut payload_buffer).map_err(|_| FlashBlockError::FormatError)?;
    let payload_len = payload.len();

    let mut block_bytes = [0xFFu8; FLASH_BLOCK_SIZE];
    block_bytes[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    block_bytes[4..8].copy_from_slice(&compute_type_hash::<T>().to_le_bytes());
    block_bytes[8..10].copy_from_slice(&(payload_len as u16).to_le_bytes());
    block_bytes[HEADER_SIZE..HEADER_SIZE + payload_len].copy_from_slice(payload);

    let crc_offset = HEADER_SIZE + payload_len;
    let crc = compute_crc(&block_bytes[..crc_offset]);
    block_bytes[crc_offset..crc_offset + CRC_SIZE].copy_from_slice(&crc.to_le_bytes());

    flash
        .erase(block_offset, block_offset + FLASH_BLOCK_SIZE_U32)
        .map_err(FlashBlockError::Io)?;
    flash
        .write(block_offset, &block_bytes)
        .map_err(FlashBlockError::Io)?;
    Ok(())
}

/// Read the block at `block_offset`.
///
/// Returns `Ok(None)` when the block has no recognized magic or the stored
/// type hash does not match `T`. Returns `Err` when the data is corrupt.
pub fn load_block<T, F>(
    flash: &mut F,
    block_offset: u32,
) -> Result<Option<T>, FlashBlockError<F::Error>>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FlashDevice,
{
    let mut block_bytes = [0u8; FLASH_BLOCK_SIZE];
    flash
        .read(block_offset, &mut block_bytes)
        .map_err(FlashBlockError::Io)?;

    let magic = u32::from_le_bytes(block_bytes[0..4].try_into().expect("4-byte slice"));
    if magic != MAGIC {
        return Ok(None);
    }

    let stored_type_hash = u32::from_le_bytes(block_bytes[4..8].try_into().expect("4-byte slice"));
    if stored_type_hash != compute_type_hash::<T>() {
        return Ok(None);
    }

    let payload_len =
        u16::from_le_bytes(block_bytes[8..10].try_into().expect("2-byte slice")) as usize;
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(FlashBlockError::StorageCorrupted);
    }

    let crc_offset = HEADER_SIZE + payload_len;
    let stored_crc = u32::from_le_bytes(
        block_bytes[crc_offset..crc_offset + CRC_SIZE]
            .try_into()
            .expect("4-byte slice"),
    );
    if stored_crc != compute_crc(&block_bytes[..crc_offset]) {
        return Err(FlashBlockError::StorageCorrupted);
    }

    let payload = &block_bytes[HEADER_SIZE..HEADER_SIZE + payload_len];
    postcard::from_bytes(payload)
        .map(Some)
        .map_err(|_| FlashBlockError::StorageCorrupted)
}

/// Erase the block at `block_offset`.
pub fn clear_block<F: FlashDevice>(
    flash: &mut F,
    block_offset: u32,
) -> Result<(), FlashBlockError<F::Error>> {
    flash
        .erase(block_offset, block_offset + FLASH_BLOCK_SIZE_U32)
        .map_err(FlashBlockError::Io)
}

/// FNV-1a hash of `T`'s fully-qualified type name.
///
/// Used as a type-safety tag stored alongside serialized data so that an attempt
/// to load the wrong type returns `Ok(None)` rather than corrupt data.
pub fn compute_type_hash<T>() -> u32 {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;

    let mut hash = FNV_OFFSET;
    for byte in type_name::<T>().bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// CRC32 checksum.
pub fn compute_crc(bytes: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::{
        FLASH_BLOCK_SIZE, FlashBlockError, FlashDevice, HEADER_SIZE, clear_block, load_block,
        save_block,
    };

    const TEST_FLASH_SIZE: usize = FLASH_BLOCK_SIZE * 4;

    struct MemoryFlashDevice {
        bytes: [u8; TEST_FLASH_SIZE],
    }

    impl MemoryFlashDevice {
        fn new() -> Self {
            Self {
                bytes: [0xFF; TEST_FLASH_SIZE],
            }
        }
    }

    impl FlashDevice for MemoryFlashDevice {
        type Error = ();

        fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), ()> {
            let offset = offset as usize;
            bytes.copy_from_slice(&self.bytes[offset..offset + bytes.len()]);
            Ok(())
        }

        fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), ()> {
            let offset = offset as usize;
            self.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
            Ok(())
        }

        fn erase(&mut self, from: u32, to: u32) -> Result<(), ()> {
            self.bytes[from as usize..to as usize].fill(0xFF);
            Ok(())
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    struct WifiPersistedState {
        ssid: heapless::String<32>,
        password: heapless::String<64>,
        timezone_offset_minutes: i32,
    }

    #[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    struct OtherState {
        timezone_offset_minutes: i32,
    }

    #[test]
    fn save_load_clear_round_trip() {
        let mut device = MemoryFlashDevice::new();
        let state = WifiPersistedState {
            ssid: heapless::String::try_from("demo-net").expect("ssid fits"),
            password: heapless::String::try_from("password123").expect("password fits"),
            timezone_offset_minutes: -300,
        };

        save_block(&mut device, 0, &state).expect("save succeeds");
        let loaded = load_block::<WifiPersistedState, _>(&mut device, 0)
            .expect("load succeeds")
            .expect("value exists");
        assert_eq!(loaded, state);

        clear_block(&mut device, 0).expect("clear succeeds");
        let cleared = load_block::<WifiPersistedState, _>(&mut device, 0).expect("load succeeds");
        assert!(cleared.is_none());
    }

    #[test]
    fn type_mismatch_returns_none() {
        let mut device = MemoryFlashDevice::new();
        let other = OtherState {
            timezone_offset_minutes: 60,
        };
        save_block(&mut device, 0, &other).expect("save succeeds");
        let result = load_block::<WifiPersistedState, _>(&mut device, 0).expect("load succeeds");
        assert!(result.is_none());
    }

    #[test]
    fn corrupted_crc_returns_error() {
        let mut device = MemoryFlashDevice::new();
        let state = WifiPersistedState {
            ssid: heapless::String::new(),
            password: heapless::String::new(),
            timezone_offset_minutes: 0,
        };
        save_block(&mut device, 0, &state).expect("save succeeds");
        device.bytes[HEADER_SIZE + 1] ^= 0x5A;

        let error = load_block::<WifiPersistedState, _>(&mut device, 0)
            .expect_err("crc mismatch should fail");
        assert!(matches!(error, FlashBlockError::<()>::StorageCorrupted));
    }
}
