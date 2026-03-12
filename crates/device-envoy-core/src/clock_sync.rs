//! A device abstraction that combines NTP time synchronization with a local clock.
//!
//! This module provides platform-independent types and logic for [`ClockSync`]
//! and a concrete runtime implementation re-exported by platform crates.
//! For a complete usage example see the platform crate's `clock_sync` module
//! (for example `device_envoy_rp::clock_sync` or `device_envoy_esp::clock_sync`).
//!
//! # WiFi feature required
//!
//! This module is only available when the `wifi` feature is enabled.

#![allow(clippy::future_not_send, reason = "single-threaded")]

use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant};
use portable_atomic::{AtomicBool, AtomicU64, Ordering};
use time::OffsetDateTime;

use crate::clock::{Clock, ClockStatic};
use crate::time_sync::{TimeSync, TimeSyncEvent, TimeSyncStatic};

// ============================================================================
// Re-exports
// ============================================================================

pub use crate::clock::UnixSeconds;

// ============================================================================
// Constants
// ============================================================================

/// Duration representing one second.
pub const ONE_SECOND: Duration = Duration::from_secs(1);
/// Duration representing one minute (60 seconds).
pub const ONE_MINUTE: Duration = Duration::from_secs(60);
/// Duration representing one day (24 hours).
pub const ONE_DAY: Duration = Duration::from_secs(86_400);

// ============================================================================
// Helpers
// ============================================================================

/// Extract hour (12-hour format), minute, and second from an
/// [`OffsetDateTime`](https://docs.rs/time/latest/time/struct.OffsetDateTime.html).
pub fn h12_m_s(dt: &OffsetDateTime) -> (u8, u8, u8) {
    let hour_24 = dt.hour() as u8;
    let hour_12 = match hour_24 {
        0 => 12,
        1..=12 => hour_24,
        _ => hour_24 - 12,
    };
    (hour_12, dt.minute() as u8, dt.second() as u8)
}

// ============================================================================
// ClockSync types
// ============================================================================

/// Tick event emitted by [`ClockSync`].
///
/// See the platform crate's `clock_sync` module for a usage example.
pub struct ClockSyncTick {
    /// The current local time (adjusted by timezone offset if set).
    pub local_time: OffsetDateTime,
    /// Duration since the last successful NTP synchronization.
    pub since_last_sync: Duration,
}

/// Platform-agnostic ClockSync operation contract.
///
/// Platform crates can use this trait for generic clock control helpers while
/// keeping constructors on the concrete type.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::clock_sync::{ClockSync, h12_m_s};
///
/// async fn log_one_tick(clock_sync: &impl ClockSync) {
///     let clock_sync_tick = clock_sync.wait_for_tick().await;
///     let (hours, minutes, seconds) = h12_m_s(&clock_sync_tick.local_time);
///     let _ = (hours, minutes, seconds);
/// }
///
/// # struct ClockSyncMock;
/// # use device_envoy_core::clock_sync::{ClockSyncTick, UnixSeconds};
/// # use time::OffsetDateTime;
/// # impl ClockSync for ClockSyncMock {
/// #     async fn wait_for_tick(&self) -> ClockSyncTick {
/// #         todo!()
/// #     }
/// #     fn now_local(&self) -> OffsetDateTime {
/// #         todo!()
/// #     }
/// #     fn set_offset_minutes(&self, _minutes: i32) {}
/// #     fn offset_minutes(&self) -> i32 { 0 }
/// #     fn set_tick_interval(&self, _interval: Option<embassy_time::Duration>) {}
/// #     fn set_speed(&self, _speed_multiplier: f32) {}
/// #     fn set_utc_time(&self, _unix_seconds: UnixSeconds) {}
/// # }
/// # let clock_sync_mock = ClockSyncMock;
/// # let _future = log_one_tick(&clock_sync_mock);
/// ```
#[allow(async_fn_in_trait)]
pub trait ClockSync {
    /// Wait for and return the next tick after sync.
    ///
    /// See the [ClockSync trait documentation](Self) for usage examples.
    async fn wait_for_tick(&self) -> ClockSyncTick;

    /// Get the current local time without waiting for a tick.
    fn now_local(&self) -> OffsetDateTime;

    /// Update the UTC offset used for local time.
    fn set_offset_minutes(&self, minutes: i32);

    /// Get the current UTC offset in minutes.
    fn offset_minutes(&self) -> i32;

    /// Set the tick interval. Use `None` to disable periodic ticks.
    ///
    /// This uses [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html) for interval timing.
    fn set_tick_interval(&self, interval: Option<embassy_time::Duration>);

    /// Update the speed multiplier (1.0 = real time).
    fn set_speed(&self, speed_multiplier: f32);

    /// Manually set the current UTC time and mark the clock as synced.
    fn set_utc_time(&self, unix_seconds: UnixSeconds);
}

type SyncReadySignal = Signal<CriticalSectionRawMutex, ()>;

pub struct ClockSyncStatic {
    clock_static: ClockStatic,
    clock_cell: static_cell::StaticCell<Clock>,
    time_sync_static: TimeSyncStatic,
    sync_ready: SyncReadySignal,
    last_sync_ticks: AtomicU64,
    synced: AtomicBool,
}

pub struct ClockSyncRuntime {
    clock: &'static Clock,
    time_sync: &'static TimeSync,
    sync_ready: &'static SyncReadySignal,
    last_sync_ticks: &'static AtomicU64,
    synced: &'static AtomicBool,
}

impl ClockSyncStatic {
    /// Creates static resources for the clock-sync runtime device.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            clock_static: Clock::new_static(),
            clock_cell: static_cell::StaticCell::new(),
            time_sync_static: TimeSync::new_static(),
            sync_ready: Signal::new(),
            last_sync_ticks: AtomicU64::new(0),
            synced: AtomicBool::new(false),
        }
    }
}

impl ClockSyncRuntime {
    /// Create clock-sync static resources.
    #[must_use]
    pub const fn new_static() -> ClockSyncStatic {
        ClockSyncStatic::new()
    }

    /// Create a clock-sync runtime using an existing network stack.
    ///
    /// See the platform crate `clock_sync` module documentation for a full usage example.
    /// The `tick_interval` parameter uses
    /// [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html).
    pub fn new(
        clock_sync_static: &'static ClockSyncStatic,
        stack: &'static Stack<'static>,
        offset_minutes: i32,
        tick_interval: Option<embassy_time::Duration>,
        spawner: Spawner,
    ) -> Self {
        let clock = Clock::new(
            &clock_sync_static.clock_static,
            offset_minutes,
            tick_interval,
            spawner,
        );
        let clock = clock_sync_static.clock_cell.init(clock);
        let time_sync = TimeSync::new(&clock_sync_static.time_sync_static, stack, spawner);

        let clock_sync = Self {
            clock,
            time_sync,
            sync_ready: &clock_sync_static.sync_ready,
            last_sync_ticks: &clock_sync_static.last_sync_ticks,
            synced: &clock_sync_static.synced,
        };

        spawner
            .spawn(clock_sync_loop(
                clock_sync.clock,
                clock_sync.time_sync,
                clock_sync.sync_ready,
                clock_sync.last_sync_ticks,
                clock_sync.synced,
            ))
            .expect("clock_sync task spawn should succeed");

        clock_sync
    }

    fn since_last_sync(&self) -> Duration {
        let last_sync_ticks = self.last_sync_ticks.load(Ordering::Acquire);
        if last_sync_ticks == 0 {
            return Duration::from_secs(0);
        }
        let now_ticks = Instant::now().as_ticks();
        assert!(now_ticks >= last_sync_ticks);
        let elapsed_ticks = now_ticks - last_sync_ticks;
        Duration::from_micros(elapsed_ticks)
    }

    async fn wait_for_first_sync(&self) {
        if self.synced.load(Ordering::Acquire) {
            return;
        }
        self.sync_ready.wait().await;
    }

    fn mark_synced(&self) {
        let now_ticks = Instant::now().as_ticks();
        self.last_sync_ticks.store(now_ticks, Ordering::Release);
        self.synced.store(true, Ordering::Release);
        self.sync_ready.signal(());
    }
}

impl ClockSync for ClockSyncRuntime {
    async fn wait_for_tick(&self) -> ClockSyncTick {
        self.wait_for_first_sync().await;
        let local_time = self.clock.wait_for_tick().await;
        ClockSyncTick {
            local_time,
            since_last_sync: self.since_last_sync(),
        }
    }

    fn now_local(&self) -> OffsetDateTime {
        self.clock.now_local()
    }

    fn set_offset_minutes(&self, minutes: i32) {
        self.clock.set_offset_minutes(minutes);
    }

    fn offset_minutes(&self) -> i32 {
        self.clock.offset_minutes()
    }

    fn set_tick_interval(&self, interval: Option<embassy_time::Duration>) {
        self.clock.set_tick_interval(interval);
    }

    fn set_speed(&self, speed_multiplier: f32) {
        self.clock.set_speed(speed_multiplier);
    }

    fn set_utc_time(&self, unix_seconds: UnixSeconds) {
        self.clock.set_utc_time(unix_seconds);
        self.mark_synced();
    }
}

// ============================================================================
// Task
// ============================================================================

#[embassy_executor::task(pool_size = 2)]
async fn clock_sync_loop(
    clock: &'static Clock,
    time_sync: &'static TimeSync,
    sync_ready: &'static SyncReadySignal,
    last_sync_ticks: &'static AtomicU64,
    synced: &'static AtomicBool,
) -> ! {
    loop {
        match time_sync.wait_for_sync().await {
            TimeSyncEvent::Ok(unix_seconds) => {
                clock.set_utc_time(unix_seconds);
                let now_ticks = Instant::now().as_ticks();
                last_sync_ticks.store(now_ticks, Ordering::Release);
                synced.store(true, Ordering::Release);
                sync_ready.signal(());
            }
            TimeSyncEvent::Err(message) => {
                #[cfg(feature = "defmt")]
                defmt::info!("ClockSync time sync failed: {}", message);
                #[cfg(not(feature = "defmt"))]
                let _ = message;
            }
        }
    }
}
