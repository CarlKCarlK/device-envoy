//! A device abstraction for WiFi auto-provisioning with captive portal fallback.
//!
//! See [`WifiAuto`] for the main struct and usage examples.
// cmk we we really need both wifi and wifi_auto modules? If so, give better names and descriptions.
// cmk understand all top-level files and folder in the gitproject (is barlink there)

#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{cell::RefCell, convert::Infallible, future::Future};
use cortex_m::peripheral::SCB;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Stack};
use embassy_rp::{
    Peri,
    dma::Channel,
    gpio::Pin,
    peripherals::{PIN_23, PIN_24, PIN_25, PIN_29},
};
use embassy_sync::{
    blocking_mutex::{Mutex, raw::CriticalSectionRawMutex},
    signal::Signal,
};
use embassy_time::{Duration, Timer, with_timeout};
use heapless::Vec;
use portable_atomic::{AtomicBool, Ordering};
use static_cell::StaticCell;

use crate::button::{Button, PressedTo};
use crate::flash_array::FlashBlock;
use crate::{Error, Result};

mod credentials;
mod dhcp;
mod dns;
pub mod fields;
mod portal;
mod stack;

use credentials::WifiCredentials as InnerWifiCredentials;
use dns::dns_server_task;
use stack::{WifiStartMode, WifiStatic as InnerWifiStatic};

pub use credentials::WifiCredentials;
pub use stack::{Wifi, WifiEvent, WifiPio, WifiStatic};

pub use portal::WifiAutoField;

/// Events emitted while provisioning or connecting.
#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum WifiAutoEvent {
    /// Captive portal is ready and waiting for user configuration.
    CaptivePortalReady,
    /// Attempting to connect to WiFi network.
    Connecting {
        /// Current attempt number (0-based).
        try_index: u8,
        /// Total number of attempts that will be made.
        try_count: u8,
    },
    /// Successfully connected to WiFi network.
    Connected,
    /// Connection failed after all attempts, device will reset.
    ConnectionFailed,
}

const MAX_CONNECT_ATTEMPTS: u8 = 4;
// cmk0 reduced from 30s since WiFi join now fails immediately instead of retrying
const CONNECT_TIMEOUT: Duration = Duration::from_secs(40);
const RETRY_DELAY: Duration = Duration::from_secs(3);

pub type WifiAutoEvents = Signal<CriticalSectionRawMutex, WifiAutoEvent>;

const MAX_WIFI_AUTO_FIELDS: usize = 8;

/// Static for [`WifiAuto`]. See [`WifiAuto`] for usage example.
pub struct WifiAutoStatic {
    events: WifiAutoEvents,
    wifi: InnerWifiStatic,
    wifi_auto_cell: StaticCell<WifiAuto>,
    force_captive_portal: AtomicBool,
    defaults: Mutex<CriticalSectionRawMutex, RefCell<Option<InnerWifiCredentials>>>,
    button: Mutex<CriticalSectionRawMutex, RefCell<Option<Button<'static>>>>,
    fields_storage: StaticCell<Vec<&'static dyn WifiAutoField, MAX_WIFI_AUTO_FIELDS>>,
}

/// A device abstraction for WiFi auto-provisioning on the Pico W.
///
/// `WifiAuto` manages WiFi connectivity end-to-end, including:
/// - automatic connection using stored credentials
/// - captive-portal fallback when credentials are missing or invalid
/// - optional collection of additional configuration fields
/// - button-triggered reconfiguration
///
/// It is designed for **Embassy-based, no_std** applications and owns the
/// WiFi event stream while it is running.
///
/// ## Hardware model
///
/// On the Pico W, the CYW43 WiFi chip uses a fixed internal wiring:
///
/// - **PIO0** or **PIO1** can be used by the WiFi driver
/// - a dedicated **DMA channel** is used for WiFi SPI
///
/// The PIO and DMA selection comes from the peripherals passed to [`WifiAuto::new`].
///
/// Other peripherals (LEDs, displays, etc.) must use *different* PIO/DMA
/// resources (for example, `PIO1` / `DMA_CH1`).
///
/// ## Event-driven connection
///
/// WiFi connection is initiated with [`WifiAutoHandle::connect_with`], which:
///
/// - blocks until WiFi is connected
/// - emits high-level [`WifiAutoEvent`] values
/// - invokes a user-supplied async handler for UI updates or logging
///
/// The handler:
///
/// - is called **sequentially**
/// - may `await`
/// - returns `Result<()>`
/// - aborts connection immediately if it returns an error
///
/// ### Capturing external state
///
/// The handler closure must return an `async move { ... }` block.
/// If you want to use external state (such as a display), bind a reference
/// before the closure and capture that reference instead of moving the value.
///
/// See the [struct-level example](WifiAuto) for the full pattern.
///
/// ## Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_kit::{
///     button::PressedTo,
///     flash_array::{FlashArray, FlashArrayStatic},
///     wifi_auto::{WifiAuto, WifiAutoEvent},
/// };
///
/// async fn example(
///     spawner: embassy_executor::Spawner,
///     p: embassy_rp::Peripherals,
/// ) -> Result<(), device_kit::Error> {
///     // Flash storage for WiFi credentials
///     static FLASH_STATIC: FlashArrayStatic = FlashArray::<1>::new_static();
///     let [wifi_flash] = FlashArray::new(&FLASH_STATIC, p.FLASH)?;
///
///     let wifi_auto = WifiAuto::new(
///         p.PIN_23,          // CYW43 power
///         p.PIN_25,          // CYW43 chip select
///         p.PIO0,            // WiFi PIO (fixed)
///         p.PIN_24,          // CYW43 clock
///         p.PIN_29,          // CYW43 data
///         p.DMA_CH0,         // WiFi DMA
///         wifi_flash,
///         p.PIN_13,          // Button for reconfiguration
///         PressedTo::Ground,
///         "PicoAccess",      // Captive-portal SSID
///         [],                // No extra fields
///         spawner,
///     )?;
///
///     let (stack, _button) = wifi_auto
///         .connect_with(|event| async move {
///             match event {
///                 WifiAutoEvent::CaptivePortalReady => {
///                     defmt::info!("Captive portal ready");
///                 }
///                 WifiAutoEvent::Connecting { try_index, try_count } => {
///                     defmt::info!(
///                         "Connecting (attempt {} of {})",
///                         try_index + 1,
///                         try_count
///                     );
///                 }
///                 WifiAutoEvent::Connected => {
///                     defmt::info!("WiFi connected");
///                 }
///                 WifiAutoEvent::ConnectionFailed => {
///                     defmt::info!("Connection failed");
///                 }
///             }
///             Ok(())
///         })
///         .await?;
///
///     // `stack` is now ready for network use
///     Ok(())
/// }
/// ```
///
/// ## Design notes
///
/// - `WifiAuto` intentionally **does not expose low-level WiFi events**
///   while running; it owns the event loop.
/// - The API favors correctness over configurability.
/// - DMA and PIO choices for WiFi are fixed to match Pico W hardware
///   and avoid subtle runtime failures.
pub struct WifiAuto {
    events: &'static WifiAutoEvents,
    wifi: &'static Wifi,
    spawner: Spawner,
    force_captive_portal: &'static AtomicBool,
    defaults: &'static Mutex<CriticalSectionRawMutex, RefCell<Option<InnerWifiCredentials>>>,
    button: &'static Mutex<CriticalSectionRawMutex, RefCell<Option<Button<'static>>>>,
    fields: &'static [&'static dyn WifiAutoField],
}

/// Handle for [`WifiAuto`]. See [`WifiAuto`] for usage example.
pub struct WifiAutoHandle {
    wifi_auto: &'static WifiAuto,
}

impl WifiAutoStatic {
    #[must_use]
    pub const fn new() -> Self {
        WifiAutoStatic {
            events: Signal::new(),
            wifi: Wifi::new_static(),
            wifi_auto_cell: StaticCell::new(),
            force_captive_portal: AtomicBool::new(false),
            defaults: Mutex::new(RefCell::new(None)),
            button: Mutex::new(RefCell::new(None)),
            fields_storage: StaticCell::new(),
        }
    }

    fn force_captive_portal_flag(&'static self) -> &'static AtomicBool {
        &self.force_captive_portal
    }

    fn defaults(
        &'static self,
    ) -> &'static Mutex<CriticalSectionRawMutex, RefCell<Option<InnerWifiCredentials>>> {
        &self.defaults
    }

    fn button(
        &'static self,
    ) -> &'static Mutex<CriticalSectionRawMutex, RefCell<Option<Button<'static>>>> {
        &self.button
    }
}

impl WifiAuto {
    /// Create static resources for [`WifiAuto`].
    ///
    /// See [`WifiAuto`] for a complete example.
    #[must_use]
    pub const fn new_static() -> WifiAutoStatic {
        WifiAutoStatic::new()
    }

    /// Initialize WiFi auto-provisioning with custom configuration fields.
    ///
    /// See [`WifiAuto`] for a complete example.
    #[allow(clippy::too_many_arguments)]
    pub fn new<const N: usize, PIO: WifiPio, DMA: Channel>(
        pin_23: Peri<'static, PIN_23>,
        pin_25: Peri<'static, PIN_25>,
        pio: Peri<'static, PIO>,
        pin_24: Peri<'static, PIN_24>,
        pin_29: Peri<'static, PIN_29>,
        dma: Peri<'static, DMA>,
        mut wifi_credentials_flash_block: FlashBlock,
        button_pin: Peri<'static, impl Pin>,
        button_pressed_to: PressedTo,
        captive_portal_ssid: &'static str,
        custom_fields: [&'static dyn WifiAutoField; N],
        spawner: Spawner,
    ) -> Result<WifiAutoHandle> {
        static WIFI_AUTO_STATIC: WifiAutoStatic = WifiAuto::new_static();
        let wifi_auto_static = &WIFI_AUTO_STATIC;

        let stored_credentials = Wifi::peek_credentials(&mut wifi_credentials_flash_block);
        let stored_start_mode = Wifi::peek_start_mode(&mut wifi_credentials_flash_block);
        if matches!(stored_start_mode, WifiStartMode::CaptivePortal) {
            if let Some(creds) = stored_credentials.clone() {
                wifi_auto_static.defaults.lock(|cell| {
                    *cell.borrow_mut() = Some(creds);
                });
            }
        }

        let button = Button::new(button_pin, button_pressed_to);
        let force_captive_portal = button.is_pressed();

        // Check if custom fields are satisfied
        let extras_ready = custom_fields
            .iter()
            .all(|field| field.is_satisfied().unwrap_or(false));

        if force_captive_portal || !extras_ready {
            if let Some(creds) = stored_credentials.clone() {
                wifi_auto_static.defaults.lock(|cell| {
                    *cell.borrow_mut() = Some(creds);
                });
            }
            Wifi::prepare_start_mode(
                &mut wifi_credentials_flash_block,
                WifiStartMode::CaptivePortal,
            )
            .map_err(|_| Error::StorageCorrupted)?;
        }

        let wifi = Wifi::new_with_captive_portal_ssid(
            &wifi_auto_static.wifi,
            pin_23,
            pin_25,
            pio,
            pin_24,
            pin_29,
            dma,
            wifi_credentials_flash_block,
            captive_portal_ssid,
            spawner,
        );

        wifi_auto_static.button.lock(|cell| {
            *cell.borrow_mut() = Some(button);
        });

        // Store fields array and convert to slice
        let fields_ref: &'static [&'static dyn WifiAutoField] = if N > 0 {
            assert!(
                N <= MAX_WIFI_AUTO_FIELDS,
                "WifiAuto supports at most {} custom fields",
                MAX_WIFI_AUTO_FIELDS
            );
            let mut storage: Vec<&'static dyn WifiAutoField, MAX_WIFI_AUTO_FIELDS> = Vec::new();
            for field in custom_fields {
                storage.push(field).unwrap_or_else(|_| unreachable!());
            }
            let stored_vec = wifi_auto_static.fields_storage.init(storage);
            stored_vec.as_slice()
        } else {
            &[]
        };

        let instance = wifi_auto_static.wifi_auto_cell.init(Self {
            events: &wifi_auto_static.events,
            wifi,
            spawner,
            force_captive_portal: wifi_auto_static.force_captive_portal_flag(),
            defaults: wifi_auto_static.defaults(),
            button: wifi_auto_static.button(),
            fields: fields_ref,
        });

        if force_captive_portal {
            instance.force_captive_portal();
        }

        Ok(WifiAutoHandle {
            wifi_auto: instance,
        })
    }

    fn force_captive_portal(&self) {
        self.force_captive_portal.store(true, Ordering::Relaxed);
    }

    /// Return the underlying WiFi handle for advanced operations such as clearing
    /// credentials. Avoid waiting on WiFi events while [`WifiAuto`] is running, as it
    /// already owns the event stream.
    pub fn wifi(&self) -> &'static Wifi {
        self.wifi
    }

    fn take_button(&self) -> Option<Button<'static>> {
        self.button.lock(|cell| cell.borrow_mut().take())
    }

    fn extra_fields_ready(&self) -> Result<bool> {
        for field in self.fields {
            let satisfied = field.is_satisfied().map_err(|_| Error::StorageCorrupted)?;
            if !satisfied {
                info!("WifiAuto: custom field not satisfied, forcing captive portal");
                return Ok(false);
            }
        }
        info!(
            "WifiAuto: all {} custom fields satisfied",
            self.fields.len()
        );
        Ok(true)
    }

    async fn connect_with<Fut, F>(
        &self,
        mut on_event: F,
    ) -> Result<(&'static Stack<'static>, Button<'static>)>
    where
        F: FnMut(WifiAutoEvent) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        self.ensure_connected_with(&mut on_event).await?;
        let stack = self.wifi.wait_for_stack().await;
        let button = self.take_button().ok_or(Error::StorageCorrupted)?;
        Ok((stack, button))
    }

    async fn signal_event_with<Fut, F>(&self, on_event: &mut F, event: WifiAutoEvent) -> Result<()>
    where
        F: FnMut(WifiAutoEvent) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        self.events.signal(event);
        on_event(event).await?;
        Ok(())
    }

    async fn ensure_connected_with<Fut, F>(&self, on_event: &mut F) -> Result<()>
    where
        F: FnMut(WifiAutoEvent) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        loop {
            let force_captive_portal = self.force_captive_portal.swap(false, Ordering::AcqRel);
            let start_mode = self.wifi.current_start_mode();
            let has_creds = self.wifi.has_persisted_credentials();
            let extras_ready = self.extra_fields_ready()?;
            info!(
                "WifiAuto: force={} has_creds={} extras_ready={}",
                force_captive_portal, has_creds, extras_ready
            );
            if force_captive_portal
                || matches!(start_mode, WifiStartMode::CaptivePortal)
                || !has_creds
                || !extras_ready
            {
                if has_creds {
                    if let Some(creds) = self.wifi.load_persisted_credentials() {
                        self.defaults.lock(|cell| {
                            *cell.borrow_mut() = Some(creds);
                        });
                    }
                }
                self.signal_event_with(on_event, WifiAutoEvent::CaptivePortalReady)
                    .await?;
                self.run_captive_portal().await?;
                unreachable!("Device should reset after captive portal submission");
            }

            for attempt in 1..=MAX_CONNECT_ATTEMPTS {
                info!(
                    "WifiAuto: connection attempt {}/{}",
                    attempt, MAX_CONNECT_ATTEMPTS
                );
                self.signal_event_with(
                    on_event,
                    WifiAutoEvent::Connecting {
                        try_index: attempt - 1,
                        try_count: MAX_CONNECT_ATTEMPTS,
                    },
                )
                .await?;
                if self
                    .wait_for_client_ready_with_timeout(CONNECT_TIMEOUT)
                    .await
                {
                    self.signal_event_with(on_event, WifiAutoEvent::Connected)
                        .await?;
                    return Ok(());
                }
                warn!("WifiAuto: connection attempt {} timed out", attempt);
                Timer::after(RETRY_DELAY).await;
            }

            info!(
                "WifiAuto: failed to connect after {} attempts, returning to captive portal",
                MAX_CONNECT_ATTEMPTS
            );
            info!("WifiAuto: signaling ConnectionFailed event");
            self.signal_event_with(on_event, WifiAutoEvent::ConnectionFailed)
                .await?;
            if let Some(creds) = self.wifi.load_persisted_credentials() {
                self.defaults.lock(|cell| {
                    *cell.borrow_mut() = Some(creds);
                });
            }
            info!("WifiAuto: writing CaptivePortal mode to flash");
            self.wifi
                .set_start_mode(WifiStartMode::CaptivePortal)
                .map_err(|_| Error::StorageCorrupted)?;
            info!("WifiAuto: flash write complete, waiting 1 second before reset");
            Timer::after_secs(1).await;
            info!("WifiAuto: resetting device now");
            SCB::sys_reset();
        }
    }

    async fn wait_for_client_ready_with_timeout(&self, timeout: Duration) -> bool {
        with_timeout(timeout, async {
            loop {
                match self.wifi.wait_for_wifi_event().await {
                    WifiEvent::ClientReady => break,
                    WifiEvent::CaptivePortalReady => {
                        info!(
                            "WifiAuto: received captive-portal-ready event while waiting for client mode"
                        );
                    }
                }
            }
        })
        .await
        .is_ok()
    }

    #[allow(unreachable_code)]
    async fn run_captive_portal(&self) -> Result<Infallible> {
        self.wifi.wait_for_wifi_event().await;
        let stack = self.wifi.wait_for_stack().await;

        let captive_portal_ip = Ipv4Address::new(192, 168, 4, 1);
        if let Err(err) = self
            .spawner
            .spawn(dns_server_task(stack, captive_portal_ip))
        {
            info!("WifiAuto: DNS server task spawn failed: {:?}", err);
        }

        let defaults_owned = self
            .defaults
            .lock(|cell| cell.borrow_mut().take())
            .or_else(|| self.wifi.load_persisted_credentials());
        let submission =
            portal::collect_credentials(stack, self.spawner, defaults_owned.as_ref(), self.fields)
                .await?;
        self.wifi.persist_credentials(&submission).map_err(|err| {
            warn!("{}", err);
            Error::StorageCorrupted
        })?;

        Timer::after_millis(750).await;
        SCB::sys_reset();
        loop {
            cortex_m::asm::nop();
        }
    }
}

impl WifiAutoHandle {
    /// Return the underlying WiFi handle for advanced operations such as clearing
    /// credentials. Avoid waiting on WiFi events while [`WifiAuto`] is running, as it
    /// already owns the event stream.
    ///
    /// See the [struct-level example](WifiAuto) for usage.
    pub fn wifi(&self) -> &'static Wifi {
        self.wifi_auto.wifi()
    }

    /// Ensures WiFi connection with UI callback for event-driven status updates.
    ///
    /// If the handler returns an error, connection is aborted and the error is returned.
    ///
    /// See the [struct-level example](WifiAuto) for usage.
    pub async fn connect_with<Fut, F>(
        self,
        on_event: F,
    ) -> Result<(&'static Stack<'static>, Button<'static>)>
    where
        F: FnMut(WifiAutoEvent) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        self.wifi_auto.connect_with(on_event).await
    }
}
