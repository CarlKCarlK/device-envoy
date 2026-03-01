//! A device abstraction that combines NTP sync with local ticking time.
//! See [`ClockSync`] for usage.

use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant};
use portable_atomic::{AtomicBool, AtomicU64, Ordering};
use time::OffsetDateTime;

use crate::clock::{Clock, ClockStatic};
pub use crate::time_sync::UnixSeconds;
use crate::time_sync::{TimeSync, TimeSyncEvent, TimeSyncStatic};

pub const ONE_SECOND: Duration = Duration::from_secs(1);
pub const ONE_MINUTE: Duration = Duration::from_secs(60);
pub const ONE_DAY: Duration = Duration::from_secs(86_400);

#[must_use]
pub fn h12_m_s(datetime: &OffsetDateTime) -> (u8, u8, u8) {
    let hour_24 = datetime.hour() as u8;
    let hour_12 = match hour_24 {
        0 => 12,
        1..=12 => hour_24,
        _ => hour_24 - 12,
    };
    (hour_12, datetime.minute() as u8, datetime.second() as u8)
}

pub struct ClockSyncTick {
    pub local_time: OffsetDateTime,
    pub since_last_sync: Duration,
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

pub struct ClockSync {
    clock: &'static Clock,
    time_sync: &'static TimeSync,
    sync_ready: &'static SyncReadySignal,
    last_sync_ticks: &'static AtomicU64,
    synced: &'static AtomicBool,
}

impl ClockSyncStatic {
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

impl ClockSync {
    #[must_use]
    pub const fn new_static() -> ClockSyncStatic {
        ClockSyncStatic::new()
    }

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

    pub async fn wait_for_tick(&self) -> ClockSyncTick {
        self.wait_for_first_sync().await;
        let local_time = self.clock.wait_for_tick().await;
        ClockSyncTick {
            local_time,
            since_last_sync: self.since_last_sync(),
        }
    }

    pub fn now_local(&self) -> OffsetDateTime {
        self.clock.now_local()
    }

    pub async fn set_offset_minutes(&self, minutes: i32) {
        self.clock.set_offset_minutes(minutes).await;
    }

    pub fn offset_minutes(&self) -> i32 {
        self.clock.offset_minutes()
    }

    pub async fn set_tick_interval(&self, interval: Option<embassy_time::Duration>) {
        self.clock.set_tick_interval(interval).await;
    }

    pub async fn set_speed(&self, speed_multiplier: f32) {
        self.clock.set_speed(speed_multiplier).await;
    }

    pub async fn set_utc_time(&self, unix_seconds: UnixSeconds) {
        self.clock.set_utc_time(unix_seconds).await;
        self.mark_synced();
    }

    fn since_last_sync(&self) -> Duration {
        let last_sync_ticks = self.last_sync_ticks.load(Ordering::Acquire);
        if last_sync_ticks == 0 {
            return Duration::from_secs(0);
        }
        let now_ticks = Instant::now().as_ticks();
        assert!(
            now_ticks >= last_sync_ticks,
            "instant ticks must be monotonic"
        );
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

#[embassy_executor::task]
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
                clock.set_utc_time(unix_seconds).await;
                let now_ticks = Instant::now().as_ticks();
                last_sync_ticks.store(now_ticks, Ordering::Release);
                synced.store(true, Ordering::Release);
                sync_ready.signal(());
            }
            TimeSyncEvent::Err(_message) => {}
        }
    }
}
