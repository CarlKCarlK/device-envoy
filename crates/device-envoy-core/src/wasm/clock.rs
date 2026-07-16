//! Browser wall-clock support for CYD applications.

use core::{
    cell::Cell,
    sync::atomic::{AtomicI64, Ordering},
};

use embassy_time::{Duration, Timer};
use time::{OffsetDateTime, Time, UtcOffset};

use crate::clock_sync::{ClockSync, ClockSyncTick, UnixSeconds};

static SELECTED_TIME_OF_DAY: AtomicI64 = AtomicI64::new(-1);

/// Set the optional deterministic time-of-day override used by browser demos.
pub fn set_time_of_day(seconds_of_day: i32) {
    SELECTED_TIME_OF_DAY.store(i64::from(seconds_of_day), Ordering::Relaxed);
}

/// A [`ClockSync`] implementation backed by browser wall-clock time.
pub struct ClockSyncWasm {
    offset_minutes: Cell<i32>,
}

impl ClockSyncWasm {
    /// Construct a clock using the browser's current local UTC offset.
    pub fn new() -> Self {
        Self {
            offset_minutes: Cell::new(-(js_sys::Date::new_0().get_timezone_offset() as i32)),
        }
    }
}

impl Default for ClockSyncWasm {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockSync for ClockSyncWasm {
    async fn wait_for_tick(&self) -> ClockSyncTick {
        Timer::after(Duration::from_secs(1)).await;
        ClockSyncTick {
            local_time: self.now_local(),
            since_last_sync: Duration::from_secs(0),
        }
    }

    fn now_local(&self) -> OffsetDateTime {
        let unix_seconds = (js_sys::Date::now() / 1000.0) as i64;
        let Ok(utc) = OffsetDateTime::from_unix_timestamp(unix_seconds) else {
            return OffsetDateTime::UNIX_EPOCH;
        };
        let Ok(offset) = UtcOffset::from_whole_seconds(self.offset_minutes.get() * 60) else {
            return utc;
        };
        let local = utc.to_offset(offset);
        let selected_time_of_day = SELECTED_TIME_OF_DAY.load(Ordering::Relaxed);
        if !(0..86_400).contains(&selected_time_of_day) {
            return local;
        }
        let seconds_of_day = selected_time_of_day as u32;
        let hour = (seconds_of_day / 3600) as u8;
        let minute = ((seconds_of_day % 3600) / 60) as u8;
        let second = (seconds_of_day % 60) as u8;
        let Ok(time) = Time::from_hms(hour, minute, second) else {
            return local;
        };
        local.replace_time(time)
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
