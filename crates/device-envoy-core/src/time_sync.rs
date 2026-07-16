//! A device abstraction for Network Time Protocol (NTP) time synchronization over WiFi.
//!
//! This module provides platform-independent NTP sync using an existing
//! [`embassy_net::Stack`]. See the platform crate's `clock_sync` module for
//! a complete usage example.
//!
//! # WiFi feature required
//!
//! This module is only available when the `wifi` feature is enabled.

#![allow(clippy::future_not_send, reason = "single-threaded")]

use embassy_executor::Spawner;
use embassy_net::{Stack, dns, udp};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use portable_atomic::{AtomicBool, Ordering};

use crate::clock::UnixSeconds;
use crate::{Error, Result};

// ============================================================================
// Types
// ============================================================================

/// Result of a time sync attempt emitted by [`TimeSync`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TimeSyncEvent {
    /// Time synchronization succeeded with the given Unix seconds.
    Ok(UnixSeconds),
    /// Time synchronization failed with the given error message.
    Err(&'static str),
}

/// Signal type used by [`TimeSync`] to publish events.
pub(crate) type TimeSyncEvents = Signal<CriticalSectionRawMutex, TimeSyncEvent>;

/// Resources needed to construct a [`TimeSync`].
pub struct TimeSyncStatic {
    events: TimeSyncEvents,
    initialized: AtomicBool,
}

// ============================================================================
// TimeSync Virtual Device
// ============================================================================

/// Device abstraction for Network Time Protocol (NTP) synchronization over WiFi.
///
/// Uses an existing network stack (typically from a platform WiFi driver).
///
/// # Sync timing
///
/// - **Initial sync**: Fires immediately on start (retries at 10 s, 30 s, 60 s, then 5 min
///   intervals on failure).
/// - **Periodic sync**: After first success, syncs every hour (retries every 5 min on failure).
///
/// See the platform crate's `clock_sync` module documentation for a full usage example.
pub struct TimeSync {
    events: &'static TimeSyncEvents,
}

impl TimeSync {
    /// Create [`TimeSync`] static resources.
    #[must_use]
    pub const fn new_static() -> TimeSyncStatic {
        TimeSyncStatic {
            events: Signal::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Create a new [`TimeSync`] using an existing Embassy network stack.
    pub fn new(
        time_sync_static: &'static TimeSyncStatic,
        stack: &'static Stack<'static>,
        spawner: Spawner,
    ) -> Result<Self> {
        let time_sync_uninitialized = time_sync_static
            .initialized
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        assert!(
            time_sync_uninitialized,
            "TimeSync::new must be called at most once per TimeSyncStatic"
        );

        spawner.spawn(
            time_sync_stack_loop(stack, &time_sync_static.events).map_err(Error::TaskSpawn)?,
        );

        Ok(Self {
            events: &time_sync_static.events,
        })
    }

    /// Wait for and return the next [`TimeSyncEvent`].
    pub async fn wait_for_sync(&self) -> TimeSyncEvent {
        self.events.wait().await
    }

    pub(crate) fn events(&self) -> &'static TimeSyncEvents {
        self.events
    }
}

// ============================================================================
// Task
// ============================================================================

#[embassy_executor::task(pool_size = 2)]
async fn time_sync_stack_loop(
    stack: &'static Stack<'static>,
    sync_events: &'static TimeSyncEvents,
) -> ! {
    run_time_sync_loop(stack, sync_events).await
}

async fn run_time_sync_loop(
    stack: &'static Stack<'static>,
    sync_events: &'static TimeSyncEvents,
) -> ! {
    #[cfg(feature = "defmt")]
    defmt::info!("TimeSync device started");

    // Initial sync with exponential backoff: 10 s, 30 s, 60 s, then 5 min.
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        #[cfg(feature = "defmt")]
        defmt::info!("TimeSync attempt {}", attempt);

        match fetch_ntp_time(stack).await {
            Ok(unix_seconds) => {
                #[cfg(feature = "defmt")]
                defmt::info!(
                    "TimeSync initial sync successful: unix_seconds={}",
                    unix_seconds.as_i64()
                );
                sync_events.signal(TimeSyncEvent::Ok(unix_seconds));
                break;
            }
            Err(msg) => {
                #[cfg(feature = "defmt")]
                defmt::info!("TimeSync attempt {} failed: {}", attempt, msg);
                sync_events.signal(TimeSyncEvent::Err(msg));
                let delay_secs: u64 = if attempt == 1 {
                    10
                } else if attempt == 2 {
                    30
                } else if attempt == 3 {
                    60
                } else {
                    300 // 5 minutes for subsequent attempts
                };
                #[cfg(feature = "defmt")]
                defmt::info!("TimeSync retrying in {} s", delay_secs);
                Timer::after_secs(delay_secs).await;
            }
        }
    }

    // Hourly sync loop (retry every 5 min on failure).
    let mut last_success_elapsed = 0_u64;
    loop {
        let wait_secs: u64 = if last_success_elapsed == 0 { 3600 } else { 300 };
        Timer::after_secs(wait_secs).await;
        last_success_elapsed = last_success_elapsed.saturating_add(wait_secs);

        #[cfg(feature = "defmt")]
        defmt::info!(
            "TimeSync periodic sync ({} s since last success)",
            last_success_elapsed
        );

        match fetch_ntp_time(stack).await {
            Ok(unix_seconds) => {
                #[cfg(feature = "defmt")]
                defmt::info!(
                    "TimeSync periodic sync successful: unix_seconds={}",
                    unix_seconds.as_i64()
                );
                sync_events.signal(TimeSyncEvent::Ok(unix_seconds));
                last_success_elapsed = 0;
            }
            Err(msg) => {
                #[cfg(feature = "defmt")]
                defmt::info!("TimeSync periodic sync failed: {}", msg);
                sync_events.signal(TimeSyncEvent::Err(msg));
            }
        }
    }
}

// ============================================================================
// NTP fetch
// ============================================================================

async fn fetch_ntp_time(stack: &Stack<'static>) -> Result<UnixSeconds, &'static str> {
    use dns::DnsQueryType;
    use udp::UdpSocket;

    const NTP_SERVER: &str = "pool.ntp.org";
    const NTP_PORT: u16 = 123;

    #[cfg(feature = "defmt")]
    defmt::info!("TimeSync resolving NTP host {}...", NTP_SERVER);

    let dns_result = stack
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await
        .map_err(|_| "DNS lookup failed")?;
    let server_addr = dns_result.first().ok_or("No DNS results")?;

    #[cfg(feature = "defmt")]
    defmt::info!(
        "TimeSync NTP server IP: {}",
        defmt::Debug2Format(server_addr)
    );

    let mut rx_meta = [udp::PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0u8; 128];
    let mut tx_meta = [udp::PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0u8; 128];
    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(0).map_err(|_| "Socket bind failed")?;

    let mut ntp_request = [0u8; 48];
    ntp_request[0] = 0x1B; // LI=0, VN=3, Mode=3 (client)
    socket
        .send_to(&ntp_request, (*server_addr, NTP_PORT))
        .await
        .map_err(|_| "NTP send failed")?;

    let mut response = [0u8; 48];
    let (response_len, _from) =
        embassy_time::with_timeout(Duration::from_secs(5), socket.recv_from(&mut response))
            .await
            .map_err(|_| "NTP receive timeout")?
            .map_err(|_| "NTP receive failed")?;

    if response_len < 48 {
        return Err("NTP response too short");
    }

    let ntp_seconds = u32::from_be_bytes([response[40], response[41], response[42], response[43]]);

    UnixSeconds::from_ntp_seconds(ntp_seconds).ok_or("Invalid NTP timestamp")
}
