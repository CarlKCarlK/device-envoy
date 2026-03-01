//! A device abstraction for type-safe persistent storage in flash memory.
//!
//! See [`FlashArray`] for details and usage.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

use core::any::type_name;

use crc32fast::Hasher;
#[cfg(target_os = "none")]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(target_os = "none")]
use embassy_sync::blocking_mutex::Mutex;
#[cfg(target_os = "none")]
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "none")]
use static_cell::StaticCell;

use crate::{Error, Result};

const MAGIC: u32 = 0x424C_4B53; // 'BLKS'
const HEADER_SIZE: usize = 10;
const CRC_SIZE: usize = 4;
const FLASH_BLOCK_SIZE: usize = 4096;
const FLASH_BLOCK_SIZE_U32: u32 = 4096;
const MAX_PAYLOAD_SIZE: usize = FLASH_BLOCK_SIZE - HEADER_SIZE - CRC_SIZE;
#[cfg(target_os = "none")]
const DEFAULT_FLASH_REGION_BYTES: u32 = 16 * FLASH_BLOCK_SIZE_U32;

trait FlashDevice {
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<()>;
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<()>;
    fn erase(&mut self, from: u32, to: u32) -> Result<()>;
}

#[cfg(target_os = "none")]
impl FlashDevice for esp_storage::FlashStorage<'static> {
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<()> {
        ReadNorFlash::read(self, offset, bytes).map_err(Error::from)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
        NorFlash::write(self, offset, bytes).map_err(Error::from)
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<()> {
        NorFlash::erase(self, from, to).map_err(Error::from)
    }
}

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FlashRegionRequest {
    Tail { byte_len: u32 },
    Explicit { start_offset: u32, byte_len: u32 },
}

#[cfg(target_os = "none")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResolvedFlashRegion {
    start_offset: u32,
    block_count: u32,
}

#[cfg(target_os = "none")]
impl FlashRegionRequest {
    fn resolve(self, flash_capacity: u32) -> Result<ResolvedFlashRegion> {
        let (start_offset, byte_len) = match self {
            Self::Tail { byte_len } => {
                if byte_len == 0 || byte_len > flash_capacity {
                    return Err(Error::InvalidFlashRegion);
                }
                (flash_capacity - byte_len, byte_len)
            }
            Self::Explicit {
                start_offset,
                byte_len,
            } => (start_offset, byte_len),
        };

        if start_offset % FLASH_BLOCK_SIZE_U32 != 0 || byte_len % FLASH_BLOCK_SIZE_U32 != 0 {
            return Err(Error::InvalidFlashRegion);
        }
        let end_offset = start_offset
            .checked_add(byte_len)
            .ok_or(Error::InvalidFlashRegion)?;
        if end_offset > flash_capacity {
            return Err(Error::InvalidFlashRegion);
        }
        Ok(ResolvedFlashRegion {
            start_offset,
            block_count: byte_len / FLASH_BLOCK_SIZE_U32,
        })
    }
}

#[cfg(target_os = "none")]
struct FlashManager {
    flash_storage:
        Mutex<CriticalSectionRawMutex, core::cell::RefCell<esp_storage::FlashStorage<'static>>>,
    next_block: core::sync::atomic::AtomicU32,
    requested_region: FlashRegionRequest,
    resolved_region: ResolvedFlashRegion,
}

#[cfg(target_os = "none")]
impl FlashManager {
    fn new(
        flash: esp_hal::peripherals::FLASH<'static>,
        requested_region: FlashRegionRequest,
    ) -> Result<Self> {
        let flash_storage = esp_storage::FlashStorage::new(flash);
        let flash_capacity = ReadNorFlash::capacity(&flash_storage) as u32;
        let resolved_region = requested_region.resolve(flash_capacity)?;
        Ok(Self {
            flash_storage: Mutex::new(core::cell::RefCell::new(flash_storage)),
            next_block: core::sync::atomic::AtomicU32::new(0),
            requested_region,
            resolved_region,
        })
    }

    fn with_flash<R>(
        &self,
        f: impl FnOnce(&mut esp_storage::FlashStorage<'static>) -> Result<R>,
    ) -> Result<R> {
        self.flash_storage.lock(|flash_storage| {
            let mut flash_storage_ref = flash_storage.borrow_mut();
            f(&mut flash_storage_ref)
        })
    }

    fn reserve<const N: usize>(&'static self) -> Result<[FlashBlock; N]> {
        let start_block = self
            .next_block
            .fetch_add(N as u32, core::sync::atomic::Ordering::SeqCst);
        let end_block = start_block
            .checked_add(N as u32)
            .ok_or(Error::IndexOutOfBounds)?;
        if end_block > self.resolved_region.block_count {
            self.next_block
                .fetch_sub(N as u32, core::sync::atomic::Ordering::SeqCst);
            return Err(Error::IndexOutOfBounds);
        }

        Ok(core::array::from_fn(|block_index| FlashBlock {
            manager: self,
            block_id: start_block + block_index as u32,
        }))
    }

    fn block_offset(&self, block_id: u32) -> Result<u32> {
        if block_id >= self.resolved_region.block_count {
            return Err(Error::IndexOutOfBounds);
        }
        let reverse_index = self.resolved_region.block_count - 1 - block_id;
        Ok(self.resolved_region.start_offset + reverse_index * FLASH_BLOCK_SIZE_U32)
    }
}

#[cfg(target_os = "none")]
struct FlashArrayStatic {
    manager_cell: StaticCell<FlashManager>,
    manager_ref: Mutex<CriticalSectionRawMutex, core::cell::RefCell<Option<&'static FlashManager>>>,
}

#[cfg(target_os = "none")]
impl FlashArrayStatic {
    const fn new() -> Self {
        Self {
            manager_cell: StaticCell::new(),
            manager_ref: Mutex::new(core::cell::RefCell::new(None)),
        }
    }

    fn manager(
        &'static self,
        flash: esp_hal::peripherals::FLASH<'static>,
        requested_region: FlashRegionRequest,
    ) -> Result<&'static FlashManager> {
        self.manager_ref.lock(|manager_slot| {
            let mut manager_slot = manager_slot.borrow_mut();
            if let Some(manager) = *manager_slot {
                if manager.requested_region != requested_region {
                    return Err(Error::FlashRegionMismatch);
                }
                return Ok(manager);
            }

            let manager_ref = self
                .manager_cell
                .init(FlashManager::new(flash, requested_region)?);
            *manager_slot = Some(manager_ref);
            Ok(manager_ref)
        })
    }
}

#[cfg(target_os = "none")]
/// A device abstraction for type-safe persistent storage in flash memory.
///
/// See this API for storing values such as Wi-Fi credentials and additional
/// setup field values used by `wifi_auto`.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_esp::flash_array::FlashArray;
///
/// #[derive(serde::Serialize, serde::Deserialize, Clone)]
/// struct WifiPersistedState {
///     ssid: heapless::String<32>,
///     password: heapless::String<64>,
///     timezone_offset_minutes: i32,
/// }
///
/// # async fn example() -> device_envoy_esp::Result<core::convert::Infallible> {
/// device_envoy_esp::init_and_start!(p);
/// let [mut wifi_persisted_state_flash_block, mut fields_flash_block] = FlashArray::<2>::new(p.FLASH)?;
///
/// let wifi_persisted_state = wifi_persisted_state_flash_block.load::<WifiPersistedState>()?;
/// if wifi_persisted_state.is_none() {
///     let wifi_persisted_state = WifiPersistedState {
///         ssid: heapless::String::new(),
///         password: heapless::String::new(),
///         timezone_offset_minutes: 0,
///     };
///     wifi_persisted_state_flash_block.save(&wifi_persisted_state)?;
/// }
///
/// fields_flash_block.clear()?;
/// # core::future::pending().await
/// # }
/// ```
pub struct FlashArray<const N: usize>;

#[cfg(target_os = "none")]
impl<const N: usize> FlashArray<N> {
    /// Reserve `N` blocks in the default tail region.
    ///
    /// See the [FlashArray struct example](Self) for usage.
    pub fn new(flash: esp_hal::peripherals::FLASH<'static>) -> Result<[FlashBlock; N]> {
        Self::new_with_request(
            flash,
            FlashRegionRequest::Tail {
                byte_len: DEFAULT_FLASH_REGION_BYTES,
            },
        )
    }

    /// Reserve `N` blocks in an explicit flash region.
    ///
    /// Both `start_offset` and `byte_len` must be multiples of 4096 bytes.
    /// See the [FlashArray struct example](Self) for usage.
    pub fn new_with_region(
        flash: esp_hal::peripherals::FLASH<'static>,
        start_offset: u32,
        byte_len: u32,
    ) -> Result<[FlashBlock; N]> {
        Self::new_with_request(
            flash,
            FlashRegionRequest::Explicit {
                start_offset,
                byte_len,
            },
        )
    }

    fn new_with_request(
        flash: esp_hal::peripherals::FLASH<'static>,
        requested_region: FlashRegionRequest,
    ) -> Result<[FlashBlock; N]> {
        static FLASH_ARRAY_STATIC: FlashArrayStatic = FlashArrayStatic::new();
        let manager = FLASH_ARRAY_STATIC.manager(flash, requested_region)?;
        manager.reserve::<N>()
    }
}

#[cfg(target_os = "none")]
pub struct FlashBlock {
    manager: &'static FlashManager,
    block_id: u32,
}

#[cfg(target_os = "none")]
impl FlashBlock {
    /// Load a typed value from this block.
    ///
    /// See the [FlashArray struct example](crate::flash_array::FlashArray) for usage.
    pub fn load<T>(&mut self) -> Result<Option<T>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let block_offset = self.manager.block_offset(self.block_id)?;
        self.manager
            .with_flash(|flash_storage| load_block(flash_storage, block_offset))
    }

    /// Save a typed value to this block.
    ///
    /// See the [FlashArray struct example](crate::flash_array::FlashArray) for usage.
    pub fn save<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let block_offset = self.manager.block_offset(self.block_id)?;
        self.manager
            .with_flash(|flash_storage| save_block(flash_storage, block_offset, value))
    }

    /// Clear this block.
    ///
    /// See the [FlashArray struct example](crate::flash_array::FlashArray) for usage.
    pub fn clear(&mut self) -> Result<()> {
        let block_offset = self.manager.block_offset(self.block_id)?;
        self.manager
            .with_flash(|flash_storage| clear_block(flash_storage, block_offset))
    }
}

fn save_block<T>(flash_device: &mut impl FlashDevice, block_offset: u32, value: &T) -> Result<()>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut payload_buffer = [0u8; MAX_PAYLOAD_SIZE];
    let payload = postcard::to_slice(value, &mut payload_buffer).map_err(|_| Error::FormatError)?;
    let payload_len = payload.len();

    let mut block_bytes = [0xFFu8; FLASH_BLOCK_SIZE];
    block_bytes[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    block_bytes[4..8].copy_from_slice(&compute_type_hash::<T>().to_le_bytes());
    block_bytes[8..10].copy_from_slice(&(payload_len as u16).to_le_bytes());
    block_bytes[HEADER_SIZE..HEADER_SIZE + payload_len].copy_from_slice(payload);

    let crc_offset = HEADER_SIZE + payload_len;
    let crc = compute_crc(&block_bytes[..crc_offset]);
    block_bytes[crc_offset..crc_offset + CRC_SIZE].copy_from_slice(&crc.to_le_bytes());

    flash_device.erase(block_offset, block_offset + FLASH_BLOCK_SIZE_U32)?;
    flash_device.write(block_offset, &block_bytes)?;
    Ok(())
}

fn load_block<T>(flash_device: &mut impl FlashDevice, block_offset: u32) -> Result<Option<T>>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut block_bytes = [0u8; FLASH_BLOCK_SIZE];
    flash_device.read(block_offset, &mut block_bytes)?;

    let magic = u32::from_le_bytes(block_bytes[0..4].try_into().expect("4 byte slice"));
    if magic != MAGIC {
        return Ok(None);
    }

    let stored_type_hash = u32::from_le_bytes(block_bytes[4..8].try_into().expect("4 byte slice"));
    if stored_type_hash != compute_type_hash::<T>() {
        return Ok(None);
    }

    let payload_len = u16::from_le_bytes(block_bytes[8..10].try_into().expect("2 byte slice"));
    let payload_len = payload_len as usize;
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(Error::StorageCorrupted);
    }

    let crc_offset = HEADER_SIZE + payload_len;
    let stored_crc = u32::from_le_bytes(
        block_bytes[crc_offset..crc_offset + CRC_SIZE]
            .try_into()
            .expect("4 byte slice"),
    );
    let computed_crc = compute_crc(&block_bytes[..crc_offset]);
    if stored_crc != computed_crc {
        return Err(Error::StorageCorrupted);
    }

    let payload = &block_bytes[HEADER_SIZE..HEADER_SIZE + payload_len];
    let value = postcard::from_bytes(payload).map_err(|_| Error::StorageCorrupted)?;
    Ok(Some(value))
}

fn clear_block(flash_device: &mut impl FlashDevice, block_offset: u32) -> Result<()> {
    flash_device.erase(block_offset, block_offset + FLASH_BLOCK_SIZE_U32)?;
    Ok(())
}

fn compute_type_hash<T>() -> u32 {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;

    let mut hash = FNV_OFFSET;
    for byte in type_name::<T>().bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn compute_crc(bytes: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize()
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::{clear_block, load_block, save_block, Error, FLASH_BLOCK_SIZE, HEADER_SIZE};
    use crate::Result;

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

    impl super::FlashDevice for MemoryFlashDevice {
        fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<()> {
            let offset = offset as usize;
            let end = offset + bytes.len();
            if end > self.bytes.len() {
                return Err(Error::IndexOutOfBounds);
            }
            bytes.copy_from_slice(&self.bytes[offset..end]);
            Ok(())
        }

        fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
            let offset = offset as usize;
            let end = offset + bytes.len();
            if end > self.bytes.len() {
                return Err(Error::IndexOutOfBounds);
            }
            self.bytes[offset..end].copy_from_slice(bytes);
            Ok(())
        }

        fn erase(&mut self, from: u32, to: u32) -> Result<()> {
            let from = from as usize;
            let to = to as usize;
            if to > self.bytes.len() || from > to {
                return Err(Error::IndexOutOfBounds);
            }
            self.bytes[from..to].fill(0xFF);
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
        let mut memory_flash_device = MemoryFlashDevice::new();
        let wifi_persisted_state = WifiPersistedState {
            ssid: heapless::String::try_from("demo-net").expect("ssid fits"),
            password: heapless::String::try_from("password123").expect("password fits"),
            timezone_offset_minutes: -300,
        };

        save_block(&mut memory_flash_device, 0, &wifi_persisted_state).expect("save succeeds");
        let loaded_wifi_persisted_state =
            load_block::<WifiPersistedState>(&mut memory_flash_device, 0)
                .expect("load succeeds")
                .expect("value exists");
        assert_eq!(loaded_wifi_persisted_state, wifi_persisted_state);

        clear_block(&mut memory_flash_device, 0).expect("clear succeeds");
        let cleared =
            load_block::<WifiPersistedState>(&mut memory_flash_device, 0).expect("load succeeds");
        assert!(cleared.is_none());
    }

    #[test]
    fn type_mismatch_returns_none() {
        let mut memory_flash_device = MemoryFlashDevice::new();
        let other_state = OtherState {
            timezone_offset_minutes: 60,
        };
        save_block(&mut memory_flash_device, 0, &other_state).expect("save succeeds");
        let wifi_persisted_state =
            load_block::<WifiPersistedState>(&mut memory_flash_device, 0).expect("load succeeds");
        assert!(wifi_persisted_state.is_none());
    }

    #[test]
    fn corrupted_crc_returns_error() {
        let mut memory_flash_device = MemoryFlashDevice::new();
        let wifi_persisted_state = WifiPersistedState {
            ssid: heapless::String::new(),
            password: heapless::String::new(),
            timezone_offset_minutes: 0,
        };
        save_block(&mut memory_flash_device, 0, &wifi_persisted_state).expect("save succeeds");
        memory_flash_device.bytes[HEADER_SIZE + 1] ^= 0x5A;

        let error = load_block::<WifiPersistedState>(&mut memory_flash_device, 0)
            .expect_err("crc mismatch should fail");
        assert!(matches!(error, Error::StorageCorrupted));
    }
}
