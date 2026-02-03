//! A device abstraction that combines time sync with a local clock.
//! See [`ClockSync`] for the full usage example.

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
pub use crate::time_sync::UnixSeconds;
use crate::time_sync::{TimeSync, TimeSyncEvent, TimeSyncStatic};

/// Duration representing one second.
pub const ONE_SECOND: Duration = Duration::from_secs(1);
/// Duration representing one minute (60 seconds).
pub const ONE_MINUTE: Duration = Duration::from_secs(60);
/// Duration representing one day (24 hours).
pub const ONE_DAY: Duration = Duration::from_secs(86_400);

/// Extract hour (12-hour format), minute, second from `OffsetDateTime`.
pub fn h12_m_s(dt: &OffsetDateTime) -> (u8, u8, u8) {
    let hour_24 = dt.hour() as u8;
    let hour_12 = match hour_24 {
        0 => 12,
        1..=12 => hour_24,
        _ => hour_24 - 12,
    };
    let minute = dt.minute() as u8;
    let second = dt.second() as u8;
    (hour_12, minute, second)
}

/// Tick event emitted by [`ClockSync`].
///
/// See the [ClockSync struct example](ClockSync) for usage.
pub struct ClockSyncTick {
    /// The current local time (adjusted by timezone offset if set).
    pub local_time: OffsetDateTime,
    /// Duration since the last successful NTP synchronization.
    pub since_last_sync: Duration,
}

type SyncReadySignal = Signal<CriticalSectionRawMutex, ()>;

/// Resources needed to construct [`ClockSync`].
pub struct ClockSyncStatic {
    clock_static: ClockStatic,
    clock_cell: static_cell::StaticCell<Clock>,
    time_sync_static: TimeSyncStatic,
    sync_ready: SyncReadySignal,
    last_sync_ticks: AtomicU64,
    synced: AtomicBool,
}

/// Combines NTP synchronization with a local clock and tick events.
///
/// `ClockSync` does not emit ticks until the first successful sync (or a manual
/// call to [`ClockSync::set_utc_time`]). Each tick includes how long it has been
/// since the last successful sync.
///
/// # Example: WiFi + ClockSync logging
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use defmt_rtt as _;
/// # use panic_probe as _;
/// use device_envoy::{
///     Error,
///     Result,
///     button::PressedTo,
///     clock_sync::{ClockSync, ClockSyncStatic, ONE_SECOND, h12_m_s},
///     flash_array::FlashArray,
///     wifi_auto::fields::{TimezoneField, TimezoneFieldStatic},
///     wifi_auto::{WifiAuto, WifiAutoEvent},
/// };
/// use defmt::info;
///
/// async fn run(
///     spawner: embassy_executor::Spawner,
///     p: embassy_rp::Peripherals,
/// ) -> Result<(), device_envoy::Error> {
///     let [wifi_credentials_flash_block, timezone_flash_block] = FlashArray::<2>::new(p.FLASH)?;
///
///     static TIMEZONE_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
///     let timezone_field = TimezoneField::new(&TIMEZONE_STATIC, timezone_flash_block);
///
///     let wifi_auto = WifiAuto::new(
///         p.PIN_23,
///         p.PIN_24,
///         p.PIN_25,
///         p.PIN_29,
///         p.PIO0,
///         p.DMA_CH0,
///         wifi_credentials_flash_block,
///         p.PIN_13,
///         PressedTo::Ground,
///         "ClockSync",
///         [timezone_field],
///         spawner,
///     )?;
///
///     let (stack, _button) = wifi_auto
///         .connect(|event| async move {
///             match event {
///                 WifiAutoEvent::CaptivePortalReady => {
///                     info!("WifiAuto: setup mode ready");
///                 }
///                 WifiAutoEvent::Connecting { .. } => {
///                     info!("WifiAuto: connecting");
///                 }
///                 WifiAutoEvent::ConnectionFailed => {
///                     info!("WifiAuto: connection failed");
///                 }
///             }
///             Ok(())
///         })
///         .await?;
///
///     let offset_minutes = timezone_field
///         .offset_minutes()?
///         .ok_or(Error::MissingCustomWifiAutoField)?;
///     static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSync::new_static();
///     let clock_sync = ClockSync::new(
///         &CLOCK_SYNC_STATIC,
///         stack,
///         offset_minutes,
///         Some(ONE_SECOND),
///         spawner,
///     );
///
///     loop {
///         let tick = clock_sync.wait_for_tick().await;
///         let (hours, minutes, seconds) = h12_m_s(&tick.local_time);
///         info!(
///             "Time {:02}:{:02}:{:02}, since sync {}s",
///             hours,
///             minutes,
///             seconds,
///             tick.since_last_sync.as_secs()
///         );
///     }
/// }
/// ```
pub struct ClockSync {
    clock: &'static Clock,
    time_sync: &'static TimeSync,
    sync_ready: &'static SyncReadySignal,
    last_sync_ticks: &'static AtomicU64,
    synced: &'static AtomicBool,
}

impl ClockSyncStatic {
    /// Creates static resources for the ClockSync device.
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
    /// Create [`ClockSync`] resources.
    #[must_use]
    pub const fn new_static() -> ClockSyncStatic {
        ClockSyncStatic::new()
    }

    /// Create a [`ClockSync`] using an existing network stack.
    ///
    /// See the [ClockSync struct example](Self) for usage.
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
            sync_ready: &clock_sync_static.sync_ready,
            last_sync_ticks: &clock_sync_static.last_sync_ticks,
            synced: &clock_sync_static.synced,
        };

        defmt::unwrap!(spawner.spawn(clock_sync_loop(
            clock_sync.clock,
            clock_sync.time_sync,
            clock_sync.sync_ready,
            clock_sync.last_sync_ticks,
            clock_sync.synced,
        )));

        clock_sync
    }

    /// Wait for and return the next tick after sync.
    ///
    /// See the [ClockSync struct example](Self) for usage.
    pub async fn wait_for_tick(&self) -> ClockSyncTick {
        self.wait_for_first_sync().await;
        let local_time = self.clock.wait_for_tick().await;
        ClockSyncTick {
            local_time,
            since_last_sync: self.since_last_sync(),
        }
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
        self.mark_synced();
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
            TimeSyncEvent::Err(message) => {
                defmt::info!("ClockSync time sync failed: {}", message);
            }
        }
    }
}
