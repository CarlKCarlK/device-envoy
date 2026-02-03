//! A device abstraction that connects a Pico with WiFi to the Internet and, when needed,
//! creates a temporary WiFi network to enter credentials.
//!
//! See [`WifiAuto`] for the main struct and usage examples.

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
use embassy_time::{Duration, Instant, Timer, with_timeout};
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

pub use stack::WifiPio;
pub(crate) use stack::{Wifi, WifiEvent};

pub use portal::WifiAutoField;

/// Events emitted while connecting. See [`WifiAuto::connect`](crate::wifi_auto::WifiAuto::connect)
/// for usage examples.
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
    /// Connection failed after all attempts, device will reset.
    ConnectionFailed,
}

const MAX_CONNECT_ATTEMPTS: u8 = 4;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(40);
const RETRY_BASE_DELAY: Duration = Duration::from_secs(3);
const RETRY_JITTER_MAX: Duration = Duration::from_millis(500);

pub(crate) type WifiAutoEvents = Signal<CriticalSectionRawMutex, WifiAutoEvent>;

const MAX_WIFI_AUTO_FIELDS: usize = 8;

/// Static for [`WifiAuto`]. See [`WifiAuto`] for usage example.
pub(crate) struct WifiAutoStatic {
    events: WifiAutoEvents,
    wifi: InnerWifiStatic,
    wifi_auto_cell: StaticCell<WifiAutoInner>,
    force_captive_portal: AtomicBool,
    defaults: Mutex<CriticalSectionRawMutex, RefCell<Option<InnerWifiCredentials>>>,
    button: Mutex<CriticalSectionRawMutex, RefCell<Option<Button<'static>>>>,
    fields_storage: StaticCell<Vec<&'static dyn WifiAutoField, MAX_WIFI_AUTO_FIELDS>>,
}
/// A device abstraction that connects a Pico with WiFi to the Internet and, when needed,
/// creates a temporary WiFi network to enter credentials.
///
/// `WifiAuto` handles WiFi connections end-to-end. It normally connects using
/// a saved WiFi network name (SSID) and password. If those values are missing
/// or invalid, it temporarily creates its own WiFi network (a “captive
/// portal”) and hosts a web form where the user can enter the local WiFi
/// ssid and password.
///
/// `WifiAuto` works on the Pico 1 W and Pico 2 W, which include the CYW43 WiFi chip.
///
/// The typical usage pattern is:
///
/// 0. Ensure your hardware includes a button wired to a GPIO. The button can be used during boot to force captive-portal mode.
/// 1. Construct a [`FlashArray`](crate::flash_array::FlashArray) to store WiFi credentials.
/// 2. Use [`WifiAuto::new`] to construct a `WifiAuto`.
/// 3. Use [`WifiAuto::connect`] to connect to WiFi while optionally showing status.
///
/// The [`WifiAuto::connect`] method returns a network stack and the button, and it consumes
/// the `WifiAuto`. See its documentation for examples and details.
///
/// Let’s look at an example. Following the example, we’ll explain the details.
/// (For additional examples, see the [wifi_auto::fields module example](crate::wifi_auto::fields)
/// and the [`WifiAuto::connect`] docs.)
///
/// ## Example: Connect with logging
///
/// This example connects to WiFi and logs progress.
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_envoy::{
///     Result,
///     button::PressedTo,
///     flash_array::FlashArray,
///     wifi_auto::{WifiAuto, WifiAutoEvent},
/// };
/// use embassy_time::Duration;
///
/// async fn connect_wifi(
///     spawner: embassy_executor::Spawner,
///     p: embassy_rp::Peripherals,
/// ) -> Result<()> {
///     // Set up flash storage for WiFi credentials
///     let [wifi_flash] = FlashArray::<1>::new(p.FLASH)?;
///
///     // Construct WifiAuto
///     let wifi_auto = WifiAuto::new(
///         p.PIN_23,          // CYW43 power
///         p.PIN_24,          // CYW43 clock
///         p.PIN_25,          // CYW43 chip select
///         p.PIN_29,          // CYW43 data
///         p.PIO0,            // WiFi PIO
///         p.DMA_CH0,         // WiFi DMA
///         wifi_flash,
///         p.PIN_13,          // Button for reconfiguration
///         PressedTo::Ground,
///         "PicoAccess",      // Captive-portal SSID
///         [],                // Any extra fields
///         spawner,
///     )?;
///
///     // Connect (logging status as we go)
///     let (stack, _button) = wifi_auto
///         .connect(|event| async move {
///             match event {
///                 WifiAutoEvent::CaptivePortalReady =>
///                     defmt::info!("Captive portal ready"),
///                 WifiAutoEvent::Connecting { .. } =>
///                     defmt::info!("Connecting to WiFi"),
///                 WifiAutoEvent::ConnectionFailed =>
///                     defmt::info!("WiFi connection failed"),
///             }
///             Ok(())
///         })
///         .await?;
///
///     defmt::info!("WiFi connected");
///
///     loop {
///         if let Ok(addresses) = stack.dns_query("google.com", embassy_net::dns::DnsQueryType::A).await {
///             defmt::info!("google.com: {:?}", addresses);
///         } else {
///             defmt::info!("google.com: lookup failed");
///         }
///         embassy_time::Timer::after(Duration::from_secs(15)).await;
///     }
/// }
/// ```
///
/// ## What happens during connection
///
/// While `connect` is running:
///
/// - The WiFi chip may reset as it switches between normal WiFi operation and
///   hosting its own temporary WiFi network.
/// - Your code should tolerate these resets.
///   Initializing LEDs or displays before WiFi is fine; just be aware they may be
///   momentarily disrupted during mode changes.
///
/// ## WiFi limitations
///
/// - Only standard SSID/password 2.4 Ghz WiFi networks are supported.
///
/// ## Performance and code size
///
/// You may choose any PIO instance and any DMA channel for WiFi.
/// With **Thin LTO enabled**, this flexibility should have no impact on
/// code size.
///
/// Recommended release profile:
///
/// ```toml
/// [profile.release]
/// # debug = 2    # uncomment for better backtraces, at the cost of code size
/// lto = "thin"
/// codegen-units = 1
/// panic = "abort"
/// ```
///
/// (Your application could also enable linker garbage collection (`--gc-sections`)
/// for embedded targets. We enable it in our `rustflags`, but in recent builds
/// it had no measurable effect on size. See the
/// [rustc linker argument docs](https://doc.rust-lang.org/rustc/codegen-options/index.html#link-arg)
/// and the
/// [Cargo rustflags docs](https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags).)
///
/// ## Hardware model
///
/// On the Pico W, the CYW43 WiFi chip is wired to fixed GPIOs. You must
/// also provide a PIO instance and a DMA channel for the WiFi driver.
///
/// These are supplied explicitly to [`WifiAuto::new`]. The chosen PIO/DMA
/// pair cannot be shared with other uses; the compiler enforces this.
pub struct WifiAuto {
    wifi_auto: &'static WifiAutoInner,
}

struct WifiAutoInner {
    events: &'static WifiAutoEvents,
    wifi: &'static Wifi,
    spawner: Spawner,
    force_captive_portal: &'static AtomicBool,
    defaults: &'static Mutex<CriticalSectionRawMutex, RefCell<Option<InnerWifiCredentials>>>,
    button: &'static Mutex<CriticalSectionRawMutex, RefCell<Option<Button<'static>>>>,
    fields: &'static [&'static dyn WifiAutoField],
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
    /// Initialize WiFi auto-provisioning with custom configuration fields.
    ///
    /// # Parameters
    ///
    /// - `pin_23`, `pin_24`, `pin_25`, `pin_29`: the internal GPIO pins for the CYW43 WiFi chip.
    /// - `pio`: PIO resource used for WiFi.
    /// - `dma`: DMA resource for WiFi.
    /// - `wifi_credentials_flash_block`: [`FlashBlock`] reserved
    ///   for WiFi credentials.
    /// - `button_pin`: Button pin used to force setup mode on boot.
    /// - `button_pressed_to`: Wiring for the button (ground or VCC).
    /// - `captive_portal_ssid`: SSID shown when the device starts setup mode.
    /// - `custom_fields`: Extra fields collected in the setup page. See the
    ///   [wifi_auto::fields module example](crate::wifi_auto::fields) for usage.
    /// - `spawner`: Embassy task spawner for background work.
    ///
    /// See the [WifiAuto struct example](Self) for a complete example.
    #[allow(clippy::too_many_arguments)]
    pub fn new<const N: usize, PIO: WifiPio, DMA: Channel>(
        pin_23: Peri<'static, PIN_23>,
        pin_24: Peri<'static, PIN_24>,
        pin_25: Peri<'static, PIN_25>,
        pin_29: Peri<'static, PIN_29>,
        pio: Peri<'static, PIO>,
        dma: Peri<'static, DMA>,
        mut wifi_credentials_flash_block: FlashBlock,
        button_pin: Peri<'static, impl Pin>,
        button_pressed_to: PressedTo,
        captive_portal_ssid: &'static str,
        custom_fields: [&'static dyn WifiAutoField; N],
        spawner: Spawner,
    ) -> Result<Self> {
        static WIFI_AUTO_STATIC: WifiAutoStatic = WifiAutoInner::new_static();
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

        // Allow the pull-up to stabilize after reset before sampling the button.
        let button = Button::new(button_pin, button_pressed_to);
        let button_reset_stabilize_cycles: u32 = 300_000;
        cortex_m::asm::delay(button_reset_stabilize_cycles);
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
            pin_24,
            pin_25,
            pin_29,
            pio,
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

        let instance = wifi_auto_static.wifi_auto_cell.init(WifiAutoInner {
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

        Ok(Self {
            wifi_auto: instance,
        })
    }

    /// Connects to WiFi (if possible), reports status, and returns the
    /// network stack and button, consuming the `WifiAuto`.
    ///
    /// See the [WifiAuto struct example](Self) for a usage example.
    ///
    /// This method does not return until WiFi is connected. It may briefly
    /// restart the Pico while switching between normal WiFi operation
    /// and hosting its temporary setup network.
    ///
    /// This `connect` method reports progress by calling a user-provided async
    /// handler whenever the WiFi state changes.
    /// The handler receives a [`WifiAutoEvent`].
    /// The handler is called sequentially for each event and may `await`.
    ///
    /// The three events are:
    /// - `CaptivePortalReady`: The device is hosting a captive portal and waiting for user input.
    /// - `Connecting`: The device is attempting to connect to the WiFi network.
    /// - `ConnectionFailed`: All connection attempts failed. The device
    ///   will reset and re-enter setup mode (for example, if the password
    ///   is incorrect).
    ///
    /// The first example uses a handler that does nothing.
    /// The second example shows how to use an LED panel to display status messages.
    /// The example on the [`WifiAuto`] struct shows simple logging.
    ///
    /// # Example 1: No-op event handler
    /// ```rust,no_run
    /// # // Based on examples/wifiauto2.rs.
    /// # #![no_std]
    /// # #![no_main]
    /// # use panic_probe as _;
    /// # use device_envoy::{
    /// #     Result,
    /// #     button::PressedTo,
    /// #     flash_array::FlashArray,
    /// #     wifi_auto::WifiAuto,
    /// # };
    /// # use embassy_executor::Spawner;
    /// # use embassy_rp::Peripherals;
    /// # async fn example(spawner: Spawner, p: Peripherals) -> Result<()> {
    /// # let [wifi_flash] = FlashArray::<1>::new(p.FLASH)?;
    /// # let wifi_auto = WifiAuto::new(
    /// #     p.PIN_23,
    /// #     p.PIN_24,
    /// #     p.PIN_25,
    /// #     p.PIN_29,
    /// #     p.PIO0,
    /// #     p.DMA_CH0,
    /// #     wifi_flash,
    /// #     p.PIN_13,
    /// #     PressedTo::Ground,
    /// #     "PicoAccess",
    /// #     [],
    /// #     spawner,
    /// # )?;
    /// let (_stack, _button) = wifi_auto
    ///     .connect(|_event| async move { Ok(()) })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example 2: Using a display to show status
    /// ```rust,no_run
    /// # // Based on demos/f_wifi_auto/f1_dns.rs.
    /// # #![no_std]
    /// # #![no_main]
    /// # use panic_probe as _;
    /// # use device_envoy::{
    /// #     Result,
    /// #     button::PressedTo,
    /// #     flash_array::FlashArray,
    /// #     led_strip::colors,
    /// #     wifi_auto::{WifiAuto, WifiAutoEvent},
    /// # };
    /// # use smart_leds::RGB8;
    /// # use embassy_executor::Spawner;
    /// # use embassy_rp::Peripherals;
    /// # struct Led8x12;
    /// # impl Led8x12 {
    /// #     async fn write_text(&self, _text: &str, _colors: &[RGB8]) -> Result<()> { Ok(()) }
    /// # }
    /// # async fn show_animated_dots(_led8x12: &Led8x12) -> Result<()> { Ok(()) }
    /// # const COLORS: &[RGB8] = &[colors::WHITE];
    /// # async fn example(spawner: Spawner, p: Peripherals) -> Result<()> {
    /// # let [wifi_flash] = FlashArray::<1>::new(p.FLASH)?;
    /// # let wifi_auto = WifiAuto::new(
    /// #     p.PIN_23,
    /// #     p.PIN_24,
    /// #     p.PIN_25,
    /// #     p.PIN_29,
    /// #     p.PIO0,
    /// #     p.DMA_CH0,
    /// #     wifi_flash,
    /// #     p.PIN_13,
    /// #     PressedTo::Ground,
    /// #     "PicoAccess",
    /// #     [],
    /// #     spawner,
    /// # )?;
    /// # let led8x12 = Led8x12;
    /// // Keep a reference so the handler can reuse the display across events.
    /// let led8x12_ref = &led8x12;
    /// let (stack, button) = wifi_auto
    ///     .connect(|event| async move {
    ///         match event {
    ///             WifiAutoEvent::CaptivePortalReady => {
    ///                 led8x12_ref.write_text("JO\nIN", COLORS).await?;
    ///             }
    ///             WifiAutoEvent::Connecting { .. } => {
    ///                 show_animated_dots(led8x12_ref).await?;
    ///             }
    ///             WifiAutoEvent::ConnectionFailed => {
    ///                 led8x12_ref.write_text("FA\nIL", COLORS).await?;
    ///             }
    ///         }
    ///         Ok(())
    ///     })
    ///     .await?;
    /// # let _stack = stack;
    /// # let _button = button;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect<Fut, F>(
        self,
        on_event: F,
    ) -> Result<(&'static Stack<'static>, Button<'static>)>
    where
        F: FnMut(WifiAutoEvent) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        self.wifi_auto.connect(on_event).await
    }
}

impl WifiAutoInner {
    #[must_use]
    const fn new_static() -> WifiAutoStatic {
        WifiAutoStatic::new()
    }

    fn force_captive_portal(&self) {
        self.force_captive_portal.store(true, Ordering::Relaxed);
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

    async fn connect<Fut, F>(
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
                    return Ok(());
                }
                warn!("WifiAuto: connection attempt {} timed out", attempt);
                let retry_delay = retry_delay_with_jitter(attempt - 1);
                info!(
                    "WifiAuto: retrying after {} ms (attempt {})",
                    retry_delay.as_millis(),
                    attempt
                );
                Timer::after(retry_delay).await;
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

fn retry_delay_with_jitter(attempt_index: u8) -> Duration {
    let base_ms = RETRY_BASE_DELAY.as_millis();
    assert!(base_ms > 0, "RETRY_BASE_DELAY must be positive");
    let jitter_max_ms = RETRY_JITTER_MAX.as_millis();
    let multiplier = 1u64
        .checked_shl(u32::from(attempt_index))
        .expect("attempt_index must fit in shift");
    let delay_ms = base_ms
        .checked_mul(multiplier)
        .expect("retry delay must fit in millis");
    let jitter_ms = if jitter_max_ms == 0 {
        0
    } else {
        Instant::now().as_millis() % (jitter_max_ms + 1)
    };
    let total_ms = delay_ms
        .checked_add(jitter_ms)
        .expect("retry delay with jitter must fit in millis");
    Duration::from_millis(total_ms)
}
