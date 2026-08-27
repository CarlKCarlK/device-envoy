//! Browser wall-clock support for CYD applications.
//!
//! ```rust,no_run
//! use device_envoy_core::wasm::clock::ClockSyncWasm;
//!
//! let clock_sync = ClockSyncWasm::new();
//! assert!(!clock_sync.control_is_visible());
//! clock_sync.show();
//! assert!(clock_sync.control_is_visible());
//! ```

use core::cell::Cell;
use std::rc::Rc;

use embassy_time::{Duration, Timer};
use time::{OffsetDateTime, Time, UtcOffset};

use crate::clock_sync::{ClockSync, ClockSyncTick, UnixSeconds};

/// A [`ClockSync`] implementation backed by browser wall-clock time.
/// See the compiled [`crate::wasm::clock`] example.
pub struct ClockSyncWasm {
    offset_minutes: Cell<i32>,
    time_of_day: Rc<Cell<Option<u32>>>,
    visible: Rc<Cell<bool>>,
}

impl ClockSyncWasm {
    /// Construct a clock using the browser's current local UTC offset.
    /// See the compiled [`crate::wasm::clock`] example.
    pub fn new() -> Self {
        Self::new_with_control_state(Rc::new(Cell::new(None)), Rc::new(Cell::new(false)))
    }

    pub(crate) fn new_with_control_state(
        time_of_day: Rc<Cell<Option<u32>>>,
        visible: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            offset_minutes: Cell::new(-(js_sys::Date::new_0().get_timezone_offset() as i32)),
            time_of_day,
            visible,
        }
    }

    /// Request the shared browser shell to display the time control.
    /// See the compiled [`crate::wasm::clock`] example.
    pub fn show(&self) {
        self.visible.set(true);
    }

    /// Return whether the shared browser shell should display the control.
    /// See the compiled [`crate::wasm::clock`] example.
    pub fn control_is_visible(&self) -> bool {
        self.visible.get()
    }

    fn browser_local_time(&self) -> OffsetDateTime {
        let unix_seconds = (js_sys::Date::now() / 1000.0) as i64;
        let Ok(utc) = OffsetDateTime::from_unix_timestamp(unix_seconds) else {
            return OffsetDateTime::UNIX_EPOCH;
        };
        let Ok(offset) = UtcOffset::from_whole_seconds(self.offset_minutes.get() * 60) else {
            return utc;
        };
        let local = utc.to_offset(offset);
        let Some(seconds_of_day) = self.time_of_day.get() else {
            return local;
        };
        let hour = (seconds_of_day / 3600) as u8;
        let minute = ((seconds_of_day % 3600) / 60) as u8;
        let second = (seconds_of_day % 60) as u8;
        let Ok(time) = Time::from_hms(hour, minute, second) else {
            return local;
        };
        local.replace_time(time)
    }
}

impl Default for ClockSyncWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "wifi")]
impl ClockSync for ClockSyncWasm {
    async fn wait_for_tick(&self) -> ClockSyncTick {
        Timer::after(Duration::from_secs(1)).await;
        ClockSyncTick {
            local_time: self.browser_local_time(),
            since_last_sync: Duration::from_secs(0),
        }
    }

    fn now_local(&self) -> OffsetDateTime {
        self.browser_local_time()
    }

    fn set_offset_minutes(&self, minutes: i32) {
        self.offset_minutes.set(minutes);
    }

    fn offset_minutes(&self) -> i32 {
        self.offset_minutes.get()
    }

    fn set_tick_interval(&self, _interval: Option<Duration>) {}
    fn set_speed(&self, _speed_multiplier: f32) {}
    fn set_utc_time(&self, _unix_seconds: UnixSeconds) {}
}

#[cfg(not(feature = "wifi"))]
impl ClockSync for ClockSyncWasm {
    fn wait_for_tick(&self) -> impl core::future::Future<Output = ClockSyncTick> {
        async {
            Timer::after(Duration::from_secs(1)).await;
            ClockSyncTick {
                local_time: self.browser_local_time(),
                since_last_sync: Duration::from_secs(0),
            }
        }
    }

    fn now_local(&self) -> OffsetDateTime {
        self.browser_local_time()
    }

    fn set_offset_minutes(&self, minutes: i32) {
        self.offset_minutes.set(minutes);
    }

    fn offset_minutes(&self) -> i32 {
        self.offset_minutes.get()
    }

    fn set_tick_interval(&self, _interval: Option<Duration>) {}
    fn set_speed(&self, _speed_multiplier: f32) {}
    fn set_utc_time(&self, _unix_seconds: UnixSeconds) {}
}
