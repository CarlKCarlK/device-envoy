//! A device abstraction for type-safe persistent storage in flash memory.
//!
//! See [`FlashArray`] for details and usage.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

#[cfg(target_os = "none")]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(target_os = "none")]
use embassy_sync::blocking_mutex::Mutex;
#[cfg(target_os = "none")]
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
#[cfg(target_os = "none")]
use serde::{Deserialize, Serialize};
#[cfg(target_os = "none")]
use static_cell::StaticCell;

#[cfg(target_os = "none")]
use crate::{Error, Result};
#[cfg(target_os = "none")]
use device_envoy_core::flash_array::{
    self as core_flash, FlashBlock as CoreFlashBlock, FlashBlockError, FlashDevice,
    FLASH_BLOCK_SIZE_U32,
};

pub use device_envoy_core::flash_array::FlashBlock;

#[cfg(target_os = "none")]
const DEFAULT_FLASH_REGION_BYTES: u32 = 16 * core_flash::FLASH_BLOCK_SIZE_U32;

// Local adapter — wraps esp_storage::FlashStorage so core's FlashDevice trait can be
// implemented for a type defined in this crate (required by the orphan rule).
#[cfg(target_os = "none")]
struct EspFlashAdapter<'a>(&'a mut esp_storage::FlashStorage<'static>);

#[cfg(target_os = "none")]
impl FlashDevice for EspFlashAdapter<'_> {
    type Error = esp_storage::FlashStorageError;

    fn read(
        &mut self,
        offset: u32,
        bytes: &mut [u8],
    ) -> Result<(), esp_storage::FlashStorageError> {
        ReadNorFlash::read(self.0, offset, bytes)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), esp_storage::FlashStorageError> {
        NorFlash::write(self.0, offset, bytes)
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), esp_storage::FlashStorageError> {
        NorFlash::erase(self.0, from, to)
    }
}

#[cfg(target_os = "none")]
fn convert_flash_block_error(e: FlashBlockError<esp_storage::FlashStorageError>) -> Error {
    match e {
        FlashBlockError::Io(err) => Error::FlashStorage(err),
        FlashBlockError::FormatError => Error::FormatError,
        FlashBlockError::StorageCorrupted => Error::StorageCorrupted,
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

    fn reserve<const N: usize>(&'static self) -> Result<[FlashBlockEsp; N]> {
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

        Ok(core::array::from_fn(|block_index| FlashBlockEsp {
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
/// use device_envoy_esp::flash_array::FlashBlock as _;
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
    pub fn new(flash: esp_hal::peripherals::FLASH<'static>) -> Result<[FlashBlockEsp; N]> {
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
    ) -> Result<[FlashBlockEsp; N]> {
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
    ) -> Result<[FlashBlockEsp; N]> {
        static FLASH_ARRAY_STATIC: FlashArrayStatic = FlashArrayStatic::new();
        let manager = FLASH_ARRAY_STATIC.manager(flash, requested_region)?;
        manager.reserve::<N>()
    }
}

#[cfg(target_os = "none")]
pub struct FlashBlockEsp {
    manager: &'static FlashManager,
    block_id: u32,
}

#[cfg(target_os = "none")]
impl CoreFlashBlock for FlashBlockEsp {
    type Error = Error;

    fn load<T>(&mut self) -> Result<Option<T>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let block_offset = self.manager.block_offset(self.block_id)?;
        self.manager.with_flash(|flash_storage| {
            let mut adapter = EspFlashAdapter(flash_storage);
            core_flash::load_block::<T, _>(&mut adapter, block_offset)
                .map_err(convert_flash_block_error)
        })
    }

    fn save<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let block_offset = self.manager.block_offset(self.block_id)?;
        self.manager.with_flash(|flash_storage| {
            let mut adapter = EspFlashAdapter(flash_storage);
            core_flash::save_block(&mut adapter, block_offset, value)
                .map_err(convert_flash_block_error)
        })
    }

    fn clear(&mut self) -> Result<()> {
        let block_offset = self.manager.block_offset(self.block_id)?;
        self.manager.with_flash(|flash_storage| {
            let mut adapter = EspFlashAdapter(flash_storage);
            core_flash::clear_block(&mut adapter, block_offset).map_err(convert_flash_block_error)
        })
    }
}
