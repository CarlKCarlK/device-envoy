//! A device abstraction that manages timekeeping and emits tick events.
//! See [`Clock`] for the runtime clock API used by [`crate::clock_sync::ClockSync`].

use core::convert::Infallible;
use core::sync::atomic::{AtomicI32, Ordering};

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use log::info;
use portable_atomic::{AtomicI64, AtomicU64};
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

use crate::time_sync::UnixSeconds;
use crate::Result;

const MAX_OFFSET_MINUTES: i32 = (24 * 60) - 1;
const SPEED_SCALE_PPM: u64 = 1_000_000;

enum ClockCommand {
    UpdateTicker,
}

type ClockCommands = Channel<CriticalSectionRawMutex, ClockCommand, 4>;
type ClockTicks = Signal<CriticalSectionRawMutex, ()>;

pub struct ClockStatic {
    commands: ClockCommands,
    ticks: ClockTicks,
    offset_minutes: AtomicI32,
    tick_interval_ms: AtomicU64,
    base_unix_micros: AtomicI64,
    base_instant_ticks: AtomicU64,
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

pub struct Clock {
    commands: &'static ClockCommands,
    ticks: &'static ClockTicks,
    offset_minutes: &'static AtomicI32,
    tick_interval_ms: &'static AtomicU64,
    base_unix_micros: &'static AtomicI64,
    base_instant_ticks: &'static AtomicU64,
    speed_scaled_ppm: &'static AtomicU64,
}

impl Clock {
    #[must_use]
    pub const fn new_static() -> ClockStatic {
        ClockStatic {
            commands: Channel::new(),
            ticks: Signal::new(),
            offset_minutes: AtomicI32::new(0),
            tick_interval_ms: AtomicU64::new(0),
            base_unix_micros: AtomicI64::new(0),
            base_instant_ticks: AtomicU64::new(0),
            speed_scaled_ppm: AtomicU64::new(SPEED_SCALE_PPM),
        }
    }

    pub fn new(
        clock_static: &'static ClockStatic,
        offset_minutes: i32,
        tick_interval: Option<embassy_time::Duration>,
        spawner: Spawner,
    ) -> Self {
        clock_static.set_offset_minutes(offset_minutes);
        clock_static.set_tick_interval_ms(tick_interval.map(|duration| duration.as_millis()));
        spawner
            .spawn(clock_device_loop(clock_static))
            .expect("clock task spawn should succeed");
        Self {
            commands: &clock_static.commands,
            ticks: &clock_static.ticks,
            offset_minutes: &clock_static.offset_minutes,
            tick_interval_ms: &clock_static.tick_interval_ms,
            base_unix_micros: &clock_static.base_unix_micros,
            base_instant_ticks: &clock_static.base_instant_ticks,
            speed_scaled_ppm: &clock_static.speed_scaled_ppm,
        }
    }

    pub async fn wait_for_tick(&self) -> OffsetDateTime {
        self.ticks.wait().await;
        self.now_local()
    }

    pub fn now_local(&self) -> OffsetDateTime {
        let offset_minutes = self.offset_minutes.load(Ordering::Relaxed);
        let base_unix_micros = self.base_unix_micros.load(Ordering::Relaxed);
        assert!(
            offset_minutes.unsigned_abs() <= MAX_OFFSET_MINUTES as u32,
            "offset minutes must be within +/-24h"
        );

        if base_unix_micros == 0 {
            return OffsetDateTime::from_unix_timestamp(0).expect("unix epoch is valid");
        }

        let base_instant_ticks = self.base_instant_ticks.load(Ordering::Relaxed);
        assert!(
            base_instant_ticks > 0,
            "base instant ticks must be initialized"
        );
        let now_ticks = Instant::now().as_ticks();
        assert!(
            now_ticks >= base_instant_ticks,
            "instant ticks must be monotonic"
        );
        let elapsed_ticks = now_ticks - base_instant_ticks;
        let speed_scaled_ppm = self.speed_scaled_ppm.load(Ordering::Relaxed);
        assert!(speed_scaled_ppm > 0, "speed multiplier must be positive");
        let scaled_elapsed_micros = scale_elapsed_microseconds(elapsed_ticks, speed_scaled_ppm);

        let utc_micros = i128::from(base_unix_micros) + i128::from(scaled_elapsed_micros);
        let utc_seconds = i64::try_from(utc_micros / 1_000_000).expect("utc seconds must fit");
        let utc_remainder_micros =
            i64::try_from(utc_micros % 1_000_000).expect("remainder micros must fit");

        let offset =
            UtcOffset::from_whole_seconds(offset_minutes * 60).expect("offset minutes validated");
        let utc = OffsetDateTime::from_unix_timestamp(utc_seconds).expect("valid timestamp")
            + TimeDuration::microseconds(utc_remainder_micros);
        utc.to_offset(offset)
    }

    pub async fn set_utc_time(&self, unix_seconds: UnixSeconds) {
        let unix_seconds = unix_seconds.as_i64();
        let unix_micros = i128::from(unix_seconds) * i128::from(1_000_000);
        let unix_micros = i64::try_from(unix_micros).expect("unix micros must fit i64");
        let now_ticks = Instant::now().as_ticks();

        self.base_unix_micros.store(unix_micros, Ordering::Relaxed);
        self.base_instant_ticks.store(now_ticks, Ordering::Relaxed);
        info!("clock time set: {}", unix_seconds);
        self.commands.send(ClockCommand::UpdateTicker).await;
    }

    pub async fn set_offset_minutes(&self, minutes: i32) {
        assert!(
            minutes.unsigned_abs() <= MAX_OFFSET_MINUTES as u32,
            "offset minutes must be within +/-24h"
        );
        self.offset_minutes.store(minutes, Ordering::Relaxed);
        info!("clock offset set: {} minutes", minutes);
        self.commands.send(ClockCommand::UpdateTicker).await;
    }

    pub fn offset_minutes(&self) -> i32 {
        self.offset_minutes.load(Ordering::Relaxed)
    }

    pub async fn set_tick_interval(&self, interval: Option<embassy_time::Duration>) {
        let interval_ms = interval.map(|duration| duration.as_millis()).unwrap_or(0);
        self.tick_interval_ms.store(interval_ms, Ordering::Relaxed);
        self.commands.send(ClockCommand::UpdateTicker).await;
    }

    pub async fn set_speed(&self, speed_multiplier: f32) {
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
                i64::try_from(elapsed_real_ticks).expect("elapsed micros must fit");
            let real_unix_micros = i128::from(base_unix_micros) + i128::from(elapsed_real_micros);
            let real_unix_micros =
                i64::try_from(real_unix_micros).expect("real unix micros must fit");
            self.base_unix_micros
                .store(real_unix_micros, Ordering::Relaxed);
        }

        self.base_instant_ticks.store(now_ticks, Ordering::Relaxed);
        self.speed_scaled_ppm
            .store(speed_scaled_ppm, Ordering::Relaxed);
        self.commands.send(ClockCommand::UpdateTicker).await;
    }
}

#[embassy_executor::task]
async fn clock_device_loop(resources: &'static ClockStatic) -> ! {
    let err = inner_clock_device_loop(resources).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_clock_device_loop(resources: &'static ClockStatic) -> Result<Infallible> {
    let mut tick_interval_ms = resources.tick_interval_ms.load(Ordering::Relaxed);
    let mut speed_scaled_ppm = resources.speed_scaled_ppm.load(Ordering::Relaxed);
    let mut emit_tick = true;

    let sleep_until_boundary = |interval_micros: u64| -> Duration {
        assert!(interval_micros > 0);
        let now_ticks = Instant::now().as_ticks();
        let ticks_until_next = interval_micros - (now_ticks % interval_micros);
        Duration::from_micros(ticks_until_next)
    };

    loop {
        if emit_tick {
            resources.ticks.signal(());
        }
        emit_tick = true;

        if tick_interval_ms == 0 {
            match resources.commands.receive().await {
                ClockCommand::UpdateTicker => {
                    tick_interval_ms = resources.tick_interval_ms.load(Ordering::Relaxed);
                    speed_scaled_ppm = resources.speed_scaled_ppm.load(Ordering::Relaxed);
                }
            }
            continue;
        }

        let interval_micros = scaled_interval_microseconds(tick_interval_ms, speed_scaled_ppm);
        let sleep_duration = sleep_until_boundary(interval_micros);

        match select(Timer::after(sleep_duration), resources.commands.receive()).await {
            Either::First(_) => {}
            Either::Second(ClockCommand::UpdateTicker) => {
                tick_interval_ms = resources.tick_interval_ms.load(Ordering::Relaxed);
                speed_scaled_ppm = resources.speed_scaled_ppm.load(Ordering::Relaxed);
                emit_tick = true;
            }
        }
    }
}

fn scaled_interval_microseconds(interval_ms: u64, speed_scaled_ppm: u64) -> u64 {
    assert!(interval_ms > 0, "interval must be positive");
    assert!(speed_scaled_ppm > 0, "speed multiplier must be positive");
    let interval_micros = interval_ms
        .checked_mul(1_000)
        .expect("interval micros must fit");
    let scaled =
        u128::from(interval_micros) * u128::from(SPEED_SCALE_PPM) / u128::from(speed_scaled_ppm);
    let scaled = u64::try_from(scaled).expect("scaled interval must fit");
    assert!(scaled > 0, "scaled interval must be positive");
    scaled
}

fn scale_elapsed_microseconds(elapsed_ticks: u64, speed_scaled_ppm: u64) -> i64 {
    assert!(speed_scaled_ppm > 0, "speed multiplier must be positive");
    let scaled =
        u128::from(elapsed_ticks) * u128::from(speed_scaled_ppm) / u128::from(SPEED_SCALE_PPM);
    i64::try_from(scaled).expect("scaled elapsed micros must fit")
}
