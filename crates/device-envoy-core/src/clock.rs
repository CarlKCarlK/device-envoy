//! A device abstraction that manages timekeeping and emits tick events.
//!
//! The clock is headless: it uses the monotonic timer only — no external peripherals required —
//! so it can run alongside other devices without owning hardware.
//!
//! See [`Clock`] for the primary API. For platform-specific examples see the platform crate
//! (for example `device_envoy_rp::clock_sync` or `device_envoy_esp::clock_sync`).

#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::sync::atomic::{AtomicI32, Ordering};

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use portable_atomic::{AtomicI64, AtomicU64};
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

// ============================================================================
// UnixSeconds
// ============================================================================

/// Units-safe wrapper for Unix timestamps (seconds since 1970-01-01 00:00:00 UTC).
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    /// Get the underlying `i64` value.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }

    /// Convert NTP seconds (since 1900-01-01) to Unix seconds (since 1970-01-01).
    #[must_use]
    pub const fn from_ntp_seconds(ntp: u32) -> Option<Self> {
        const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
        let seconds = (ntp as i64) - NTP_TO_UNIX_SECONDS;
        if seconds >= 0 {
            Some(Self(seconds))
        } else {
            None
        }
    }

    /// Convert to [`OffsetDateTime`] with the given timezone offset.
    #[must_use]
    pub fn to_offset_datetime(self, offset: UtcOffset) -> Option<OffsetDateTime> {
        OffsetDateTime::from_unix_timestamp(self.as_i64())
            .ok()
            .map(|datetime| datetime.to_offset(offset))
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Maximum absolute offset minutes supported by [`UtcOffset`] (< 24 h).
pub const MAX_OFFSET_MINUTES: i32 = (24 * 60) - 1;
/// Fixed-point scale factor for speed multiplier (parts per million).
const SPEED_SCALE_PPM: u64 = 1_000_000;

// ============================================================================
// Types
// ============================================================================

/// Signal type for clock update notifications.
type ClockUpdates = Signal<CriticalSectionRawMutex, ()>;
/// Signal type for clock tick notifications.
type ClockTicks = Signal<CriticalSectionRawMutex, ()>;

// ============================================================================
// Clock Virtual Device
// ============================================================================

/// Resources needed by the [`Clock`] device.
pub struct ClockStatic {
    updates: ClockUpdates,
    ticks: ClockTicks,
    offset_minutes: AtomicI32,
    tick_interval_ms: AtomicU64,
    /// Base UTC timestamp in microseconds corresponding to `base_instant_ticks` (0 = not set).
    base_unix_micros: AtomicI64,
    /// Monotonic ticks (microseconds) when `base_unix_micros` was captured.
    base_instant_ticks: AtomicU64,
    /// Speed multiplier scaled by `SPEED_SCALE_PPM` (1.0x = 1_000_000).
    speed_scaled_ppm: AtomicU64,
}

impl ClockStatic {
    fn set_offset_minutes(&self, offset_minutes: i32) {
        self.offset_minutes.store(offset_minutes, Ordering::Relaxed);
    }

    fn set_tick_interval_ms(&self, tick_interval_ms: Option<u64>) {
        let value = tick_interval_ms.unwrap_or(0);
        self.tick_interval_ms.store(value, Ordering::Relaxed);
    }
}

/// A device abstraction that manages timekeeping and emits time tick events.
///
/// Pass `Some(duration)` to enable periodic ticks aligned to that interval; use `None` to emit
/// ticks only when time/offset changes. The clock is headless (no hardware ownership) and
/// supports time scaling for demos/tests via [`Clock::set_speed`].
///
/// This type is used internally by `ClockSync` in the platform crate.
pub struct Clock {
    updates: &'static ClockUpdates,
    ticks: &'static ClockTicks,
    offset_minutes: &'static AtomicI32,
    tick_interval_ms: &'static AtomicU64,
    base_unix_micros: &'static AtomicI64,
    base_instant_ticks: &'static AtomicU64,
    speed_scaled_ppm: &'static AtomicU64,
}

impl Clock {
    pub(crate) const fn from_static(clock_static: &'static ClockStatic) -> Self {
        Self {
            updates: &clock_static.updates,
            ticks: &clock_static.ticks,
            offset_minutes: &clock_static.offset_minutes,
            tick_interval_ms: &clock_static.tick_interval_ms,
            base_unix_micros: &clock_static.base_unix_micros,
            base_instant_ticks: &clock_static.base_instant_ticks,
            speed_scaled_ppm: &clock_static.speed_scaled_ppm,
        }
    }

    /// Create [`Clock`] static resources.
    #[must_use]
    pub const fn new_static() -> ClockStatic {
        ClockStatic {
            updates: Signal::new(),
            ticks: Signal::new(),
            offset_minutes: AtomicI32::new(0),
            tick_interval_ms: AtomicU64::new(0),
            base_unix_micros: AtomicI64::new(0),
            base_instant_ticks: AtomicU64::new(0),
            speed_scaled_ppm: AtomicU64::new(SPEED_SCALE_PPM),
        }
    }

    /// Create a new [`Clock`] device and spawn its task.
    ///
    /// The `tick_interval` parameter uses [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html).
    pub fn new(
        clock_static: &'static ClockStatic,
        offset_minutes: i32,
        tick_interval: Option<embassy_time::Duration>,
        spawner: Spawner,
    ) -> Self {
        clock_static.set_offset_minutes(offset_minutes);
        clock_static.set_tick_interval_ms(tick_interval.map(|d| d.as_millis()));
        spawner
            .spawn(clock_device_loop(clock_static))
            .expect("clock task spawn should succeed");
        Self::from_static(clock_static)
    }

    /// Wait for and return the next clock tick event.
    ///
    /// If constructed with `None` tick interval, ticks occur only when time or offset changes.
    /// Passing `Some(duration)` enables periodic ticks aligned to that interval.
    pub async fn wait_for_tick(&self) -> OffsetDateTime {
        self.ticks.wait().await;
        self.now_local()
    }

    /// Get the current local time (offset already applied) without waiting for a tick.
    ///
    /// Computed from atomics + `Instant::now()` — no async required.
    pub fn now_local(&self) -> OffsetDateTime {
        let offset_minutes = self.offset_minutes.load(Ordering::Relaxed);
        let base_unix_micros = self.base_unix_micros.load(Ordering::Relaxed);
        assert!(
            offset_minutes.unsigned_abs() <= MAX_OFFSET_MINUTES as u32,
            "offset minutes within +/-24h"
        );

        if base_unix_micros == 0 {
            // Time not set — return epoch midnight.
            return OffsetDateTime::from_unix_timestamp(0).expect("midnight is valid");
        }

        let base_instant_ticks = self.base_instant_ticks.load(Ordering::Relaxed);
        assert!(
            base_instant_ticks > 0,
            "base_instant_ticks must be set when time is set"
        );
        let now_ticks = Instant::now().as_ticks();
        assert!(now_ticks >= base_instant_ticks);
        let elapsed_ticks = now_ticks - base_instant_ticks;
        let speed_scaled_ppm = self.speed_scaled_ppm.load(Ordering::Relaxed);
        assert!(speed_scaled_ppm > 0, "speed multiplier must be positive");
        let scaled_elapsed_micros = scale_elapsed_microseconds(elapsed_ticks, speed_scaled_ppm);

        let utc_micros = i128::from(base_unix_micros) + i128::from(scaled_elapsed_micros);
        let utc_seconds = i64::try_from(utc_micros / 1_000_000).expect("utc seconds fits");
        let utc_remainder_micros =
            i64::try_from(utc_micros % 1_000_000).expect("microsecond remainder fits");

        #[expect(
            clippy::arithmetic_side_effects,
            reason = "UtcOffset bounds validate minutes"
        )]
        let offset = UtcOffset::from_whole_seconds(offset_minutes * 60)
            .expect("offset minutes within +/-24h");
        let utc = OffsetDateTime::from_unix_timestamp(utc_seconds).expect("valid utc timestamp")
            + TimeDuration::microseconds(utc_remainder_micros);
        utc.to_offset(offset)
    }

    /// Set the current UTC time.
    pub fn set_utc_time(&self, unix_seconds: UnixSeconds) {
        let unix_seconds_val = unix_seconds.as_i64();
        let unix_micros = i128::from(unix_seconds_val) * 1_000_000_i128;
        let unix_micros = i64::try_from(unix_micros).expect("unix micros fits in i64");
        let now_ticks = Instant::now().as_ticks();

        self.base_unix_micros.store(unix_micros, Ordering::Relaxed);
        self.base_instant_ticks.store(now_ticks, Ordering::Relaxed);
        #[cfg(feature = "defmt")]
        defmt::info!("Clock time set: {}", unix_seconds_val);
        self.updates.signal(());
    }

    /// Update the UTC offset used for subsequent [`now_local`](Clock::now_local) results and tick events.
    pub fn set_offset_minutes(&self, minutes: i32) {
        assert!(
            minutes.unsigned_abs() <= MAX_OFFSET_MINUTES as u32,
            "offset minutes within +/-24h"
        );
        self.offset_minutes.store(minutes, Ordering::Relaxed);
        #[cfg(feature = "defmt")]
        defmt::info!("Clock UTC offset updated to {} minutes", minutes);
        self.updates.signal(());
    }

    /// Get the current UTC offset in minutes.
    pub fn offset_minutes(&self) -> i32 {
        self.offset_minutes.load(Ordering::Relaxed)
    }

    /// Set the tick interval (e.g., `Some(ONE_SECOND)`).
    ///
    /// Use `None` to disable periodic ticks (only emit on time/offset changes).
    /// This uses [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html) for interval timing.
    pub fn set_tick_interval(&self, interval: Option<embassy_time::Duration>) {
        let interval_ms = interval.map(|d| d.as_millis()).unwrap_or(0);
        self.tick_interval_ms.store(interval_ms, Ordering::Relaxed);
        #[cfg(feature = "defmt")]
        if interval_ms == 0 {
            defmt::info!("Clock tick interval cleared (ticks only on updates)");
        } else {
            defmt::info!("Clock tick interval updated to {} ms", interval_ms);
        }
        self.updates.signal(());
    }

    /// Update the speed multiplier (1.0 = real time).
    ///
    /// Changing speed resets the base time to the current real time so returning to 1.0 resumes
    /// the correct clock. Useful for fast-forwarding demos or accelerating tests without sleeping
    /// in real time.
    pub fn set_speed(&self, speed_multiplier: f32) {
        assert!(speed_multiplier.is_finite(), "speed must be finite");
        assert!(speed_multiplier > 0.0, "speed must be positive");
        let scaled = speed_multiplier * SPEED_SCALE_PPM as f32 + 0.5;
        assert!(scaled.is_finite(), "scaled speed must be finite");
        assert!(scaled > 0.0, "scaled speed must be positive");
        assert!(scaled <= u64::MAX as f32, "scaled speed must fit in u64");
        let speed_scaled_ppm = scaled as u64;

        let now_ticks = Instant::now().as_ticks();
        let base_unix_micros = self.base_unix_micros.load(Ordering::Relaxed);
        if base_unix_micros != 0 {
            let base_instant_ticks = self.base_instant_ticks.load(Ordering::Relaxed);
            assert!(
                base_instant_ticks > 0,
                "base instant must be set when time is set"
            );
            assert!(now_ticks >= base_instant_ticks);
            let elapsed_real_ticks = now_ticks - base_instant_ticks;
            let elapsed_real_micros =
                i64::try_from(elapsed_real_ticks).expect("elapsed real micros fits in i64");
            let real_unix_micros = i128::from(base_unix_micros) + i128::from(elapsed_real_micros);
            let real_unix_micros =
                i64::try_from(real_unix_micros).expect("real unix micros fits in i64");
            self.base_unix_micros
                .store(real_unix_micros, Ordering::Relaxed);
        }

        self.base_instant_ticks.store(now_ticks, Ordering::Relaxed);
        self.speed_scaled_ppm
            .store(speed_scaled_ppm, Ordering::Relaxed);
        #[cfg(feature = "defmt")]
        defmt::info!("Clock speed set: {} ppm", speed_scaled_ppm);
        self.updates.signal(());
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn clock_device_loop(resources: &'static ClockStatic) -> ! {
    inner_clock_device_loop(resources).await
}

async fn inner_clock_device_loop(resources: &'static ClockStatic) -> ! {
    let mut tick_interval_ms = resources.tick_interval_ms.load(Ordering::Relaxed);
    let mut speed_scaled_ppm = resources.speed_scaled_ppm.load(Ordering::Relaxed);
    let offset_minutes = resources.offset_minutes.load(Ordering::Relaxed);

    #[cfg(feature = "defmt")]
    defmt::info!(
        "Clock device started (UTC offset: {} minutes, tick interval: {} ms, speed: {} ppm)",
        offset_minutes,
        tick_interval_ms,
        speed_scaled_ppm
    );
    #[cfg(not(feature = "defmt"))]
    let _ = offset_minutes;

    let sleep_until_boundary = |interval_micros: u64| -> Duration {
        assert!(interval_micros > 0);
        let now_ticks = Instant::now().as_ticks();
        let ticks_until_next = interval_micros - (now_ticks % interval_micros);
        Duration::from_micros(ticks_until_next)
    };

    let mut emit_tick = true;
    loop {
        if emit_tick {
            resources.ticks.signal(());
        }
        emit_tick = true;

        if tick_interval_ms == 0 {
            // No periodic ticks; wait for a command to trigger a single tick.
            resources.updates.wait().await;
            tick_interval_ms = resources.tick_interval_ms.load(Ordering::Relaxed);
            speed_scaled_ppm = resources.speed_scaled_ppm.load(Ordering::Relaxed);
            continue;
        }

        let interval_micros = scaled_interval_microseconds(tick_interval_ms, speed_scaled_ppm);
        let sleep_duration = sleep_until_boundary(interval_micros);

        match select(Timer::after(sleep_duration), resources.updates.wait()).await {
            Either::First(_) => {
                // Timer elapsed — tick occurred; loop will signal again.
            }
            Either::Second(_) => {
                tick_interval_ms = resources.tick_interval_ms.load(Ordering::Relaxed);
                speed_scaled_ppm = resources.speed_scaled_ppm.load(Ordering::Relaxed);
                emit_tick = true;
            }
        }
    }
}

fn scaled_interval_microseconds(interval_ms: u64, speed_scaled_ppm: u64) -> u64 {
    assert!(interval_ms > 0, "interval must be positive");
    assert!(speed_scaled_ppm > 0, "speed must be positive");
    let interval_micros = interval_ms
        .checked_mul(1_000)
        .expect("interval micros fits in u64");
    let scaled =
        u128::from(interval_micros) * u128::from(SPEED_SCALE_PPM) / u128::from(speed_scaled_ppm);
    let scaled = u64::try_from(scaled).expect("scaled interval fits in u64");
    assert!(scaled > 0, "scaled interval must be positive");
    scaled
}

fn scale_elapsed_microseconds(elapsed_ticks: u64, speed_scaled_ppm: u64) -> i64 {
    assert!(speed_scaled_ppm > 0, "speed must be positive");
    let scaled =
        u128::from(elapsed_ticks) * u128::from(speed_scaled_ppm) / u128::from(SPEED_SCALE_PPM);
    i64::try_from(scaled).expect("scaled elapsed fits in i64")
}
