//! A device abstraction for NTP time synchronization over Wi-Fi.
//! See [`crate::clock_sync::ClockSync`] for the high-level clock API.

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_net::{dns, udp, Stack};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use log::{info, warn};
use static_cell::StaticCell;
use time::{OffsetDateTime, UtcOffset};

use crate::{Error, Result};

/// Units-safe wrapper for Unix timestamps (seconds since 1970-01-01 UTC).
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }

    #[must_use]
    pub const fn from_ntp_seconds(ntp_seconds: u32) -> Option<Self> {
        const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
        let seconds = (ntp_seconds as i64) - NTP_TO_UNIX_SECONDS;
        if seconds >= 0 {
            Some(Self(seconds))
        } else {
            None
        }
    }

    #[must_use]
    pub fn to_offset_datetime(self, offset: UtcOffset) -> Option<OffsetDateTime> {
        OffsetDateTime::from_unix_timestamp(self.as_i64())
            .ok()
            .map(|datetime| datetime.to_offset(offset))
    }
}

#[derive(Debug)]
pub enum TimeSyncEvent {
    Ok(UnixSeconds),
    Err(&'static str),
}

type TimeSyncEvents = Signal<CriticalSectionRawMutex, TimeSyncEvent>;

pub struct TimeSyncStatic {
    events: TimeSyncEvents,
    time_sync_cell: StaticCell<TimeSync>,
}

pub struct TimeSync {
    events: &'static TimeSyncEvents,
}

impl TimeSync {
    #[must_use]
    pub const fn new_static() -> TimeSyncStatic {
        TimeSyncStatic {
            events: Signal::new(),
            time_sync_cell: StaticCell::new(),
        }
    }

    pub fn new(
        time_sync_static: &'static TimeSyncStatic,
        stack: &'static Stack<'static>,
        spawner: Spawner,
    ) -> &'static Self {
        spawner
            .spawn(time_sync_stack_loop(stack, &time_sync_static.events))
            .expect("time_sync task spawn should succeed");

        time_sync_static.time_sync_cell.init(Self {
            events: &time_sync_static.events,
        })
    }

    pub async fn wait_for_sync(&self) -> TimeSyncEvent {
        self.events.wait().await
    }
}

#[embassy_executor::task]
async fn time_sync_stack_loop(
    stack: &'static Stack<'static>,
    sync_events: &'static TimeSyncEvents,
) -> ! {
    let err = run_time_sync_loop(stack, sync_events).await.unwrap_err();
    panic!("{err:?}");
}

async fn run_time_sync_loop(
    stack: &'static Stack<'static>,
    sync_events: &'static TimeSyncEvents,
) -> Result<Infallible> {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match fetch_ntp_time(stack).await {
            Ok(unix_seconds) => {
                sync_events.signal(TimeSyncEvent::Ok(unix_seconds));
                break;
            }
            Err(error) => {
                if let Error::Ntp(message) = error {
                    sync_events.signal(TimeSyncEvent::Err(message));
                }
                let delay_seconds = if attempt == 1 {
                    10
                } else if attempt == 2 {
                    30
                } else if attempt == 3 {
                    60
                } else {
                    300
                };
                Timer::after_secs(delay_seconds).await;
            }
        }
    }

    let mut seconds_since_success = 0u64;
    loop {
        let wait_seconds = if seconds_since_success == 0 {
            3600
        } else {
            300
        };
        Timer::after_secs(wait_seconds).await;
        seconds_since_success = seconds_since_success.saturating_add(wait_seconds);

        match fetch_ntp_time(stack).await {
            Ok(unix_seconds) => {
                sync_events.signal(TimeSyncEvent::Ok(unix_seconds));
                seconds_since_success = 0;
            }
            Err(error) => {
                if let Error::Ntp(message) = error {
                    sync_events.signal(TimeSyncEvent::Err(message));
                }
            }
        }
    }
}

async fn fetch_ntp_time(stack: &Stack<'static>) -> Result<UnixSeconds> {
    use dns::DnsQueryType;
    use udp::UdpSocket;

    const NTP_SERVER: &str = "pool.ntp.org";
    const NTP_PORT: u16 = 123;

    info!("resolving NTP host: {}", NTP_SERVER);
    let dns_results = stack
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await
        .map_err(|error| {
            warn!("NTP DNS failed: {:?}", error);
            Error::Ntp("DNS lookup failed")
        })?;
    let server_ip = dns_results.first().ok_or(Error::Ntp("No DNS results"))?;

    let mut rx_metadata = [udp::PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0; 128];
    let mut tx_metadata = [udp::PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0; 128];
    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_metadata,
        &mut rx_buffer,
        &mut tx_metadata,
        &mut tx_buffer,
    );

    socket.bind(0).map_err(|error| {
        warn!("NTP bind failed: {:?}", error);
        Error::Ntp("Socket bind failed")
    })?;

    let mut ntp_request = [0u8; 48];
    ntp_request[0] = 0x1B;
    socket
        .send_to(&ntp_request, (*server_ip, NTP_PORT))
        .await
        .map_err(|error| {
            warn!("NTP send failed: {:?}", error);
            Error::Ntp("NTP send failed")
        })?;

    let mut ntp_response = [0u8; 48];
    let (response_len, _) =
        embassy_time::with_timeout(Duration::from_secs(5), socket.recv_from(&mut ntp_response))
            .await
            .map_err(|_| Error::Ntp("NTP receive timeout"))?
            .map_err(|_| Error::Ntp("NTP receive failed"))?;
    if response_len < 48 {
        return Err(Error::Ntp("NTP response too short"));
    }

    let ntp_seconds = u32::from_be_bytes([
        ntp_response[40],
        ntp_response[41],
        ntp_response[42],
        ntp_response[43],
    ]);
    UnixSeconds::from_ntp_seconds(ntp_seconds).ok_or(Error::Ntp("Invalid NTP timestamp"))
}
