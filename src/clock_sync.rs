//! A device abstraction that combines time sync with a local clock.
//! See [`ClockSync`] for usage and examples.

#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant};
use portable_atomic::{AtomicBool, AtomicU64, Ordering};
use time::OffsetDateTime;

use crate::clock::{Clock, ClockStatic};
use crate::time_sync::{TimeSync, TimeSyncEvent, TimeSyncStatic, UnixSeconds};

/// Tick event emitted by [`ClockSync`].
pub struct ClockSyncTick {
    pub local_time: OffsetDateTime,
    pub since_last_sync: Duration,
}

/// Events emitted by [`ClockSync`] when time sync succeeds or fails.
#[derive(Clone)]
pub enum ClockSyncEvent {
    Synced { unix_seconds: UnixSeconds },
    SyncFailed(&'static str),
}

type ClockSyncEvents = Signal<CriticalSectionRawMutex, ClockSyncEvent>;
type SyncReadySignal = Signal<CriticalSectionRawMutex, ()>;

/// Resources needed to construct [`ClockSync`].
pub struct ClockSyncStatic {
    clock_static: ClockStatic,
    clock_cell: static_cell::StaticCell<Clock>,
    time_sync_static: TimeSyncStatic,
    events: ClockSyncEvents,
    sync_ready: SyncReadySignal,
    last_sync_ticks: AtomicU64,
    synced: AtomicBool,
}

/// Combines NTP synchronization with a local clock and tick events.
///
/// `ClockSync` does not emit ticks until the first successful sync (or a manual
/// call to [`ClockSync::set_utc_time`]). Each tick includes how long it has been
/// since the last successful sync.
pub struct ClockSync {
    clock: &'static Clock,
    time_sync: &'static TimeSync,
    events: &'static ClockSyncEvents,
    sync_ready: &'static SyncReadySignal,
    last_sync_ticks: &'static AtomicU64,
    synced: &'static AtomicBool,
}

impl ClockSyncStatic {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            clock_static: Clock::new_static(),
            clock_cell: static_cell::StaticCell::new(),
            time_sync_static: TimeSync::new_static(),
            events: Signal::new(),
            sync_ready: Signal::new(),
            last_sync_ticks: AtomicU64::new(0),
            synced: AtomicBool::new(false),
        }
    }
}

impl ClockSync {
    /// Create [`ClockSync`] resources.
    #[must_use]
    pub const fn new_static() -> ClockSyncStatic {
        ClockSyncStatic::new()
    }

    /// Create a [`ClockSync`] using an existing network stack.
    pub fn new(
        clock_sync_static: &'static ClockSyncStatic,
        stack: &'static Stack<'static>,
        offset_minutes: i32,
        tick_interval: Option<Duration>,
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
            events: &clock_sync_static.events,
            sync_ready: &clock_sync_static.sync_ready,
            last_sync_ticks: &clock_sync_static.last_sync_ticks,
            synced: &clock_sync_static.synced,
        };

        defmt::unwrap!(spawner.spawn(clock_sync_loop(
            clock_sync.clock,
            clock_sync.time_sync,
            clock_sync.events,
            clock_sync.sync_ready,
            clock_sync.last_sync_ticks,
            clock_sync.synced,
        )));

        clock_sync
    }

    /// Wait for and return the next tick after sync.
    pub async fn wait_for_tick(&self) -> ClockSyncTick {
        self.wait_for_first_sync().await;
        let local_time = self.clock.wait_for_tick().await;
        ClockSyncTick {
            local_time,
            since_last_sync: self.since_last_sync(),
        }
    }

    /// Wait for and return the next sync event.
    pub async fn wait_for_sync(&self) -> ClockSyncEvent {
        self.events.wait().await
    }

    /// Get the current local time without waiting for a tick.
    pub fn now_local(&self) -> OffsetDateTime {
        self.clock.now_local()
    }

    /// Update the UTC offset used for local time.
    pub async fn set_offset_minutes(&self, minutes: i32) {
        self.clock.set_offset_minutes(minutes).await;
    }

    /// Get the current UTC offset in minutes.
    pub fn offset_minutes(&self) -> i32 {
        self.clock.offset_minutes()
    }

    /// Set the tick interval. Use `None` to disable periodic ticks.
    pub async fn set_tick_interval(&self, interval: Option<Duration>) {
        self.clock.set_tick_interval(interval).await;
    }

    /// Update the speed multiplier (1.0 = real time).
    pub async fn set_speed(&self, speed_multiplier: f32) {
        self.clock.set_speed(speed_multiplier).await;
    }

    /// Manually set the current UTC time and mark the clock as synced.
    pub async fn set_utc_time(&self, unix_seconds: UnixSeconds) {
        self.clock.set_utc_time(unix_seconds).await;
        self.mark_synced(unix_seconds);
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

    fn mark_synced(&self, unix_seconds: UnixSeconds) {
        let now_ticks = Instant::now().as_ticks();
        self.last_sync_ticks.store(now_ticks, Ordering::Release);
        self.synced.store(true, Ordering::Release);
        self.sync_ready.signal(());
        self.events.signal(ClockSyncEvent::Synced { unix_seconds });
    }
}

#[embassy_executor::task]
async fn clock_sync_loop(
    clock: &'static Clock,
    time_sync: &'static TimeSync,
    events: &'static ClockSyncEvents,
    sync_ready: &'static SyncReadySignal,
    last_sync_ticks: &'static AtomicU64,
    synced: &'static AtomicBool,
) -> ! {
    loop {
        match time_sync.wait_for_sync().await {
            TimeSyncEvent::Success { unix_seconds } => {
                clock.set_utc_time(unix_seconds).await;
                let now_ticks = Instant::now().as_ticks();
                last_sync_ticks.store(now_ticks, Ordering::Release);
                synced.store(true, Ordering::Release);
                sync_ready.signal(());
                events.signal(ClockSyncEvent::Synced { unix_seconds });
            }
            TimeSyncEvent::Failed(message) => {
                events.signal(ClockSyncEvent::SyncFailed(message));
            }
        }
    }
}
