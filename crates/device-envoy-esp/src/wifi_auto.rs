//! A device abstraction that connects ESP devices with Wi-Fi to the Internet and, when needed,
//! creates a temporary Wi-Fi network to enter credentials.
//!
//! See [`WifiAutoEsp`] for the main struct and usage examples.

#[cfg(target_os = "none")]
mod dhcp;
#[cfg(target_os = "none")]
mod dns;
pub mod fields;

use core::cell::RefCell;
#[cfg(target_os = "none")]
use core::future::Future;
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "none")]
use crate::button::Button;
#[cfg(target_os = "none")]
use crate::flash_block::{FlashBlock as _, FlashBlockEsp};
use crate::Result;
#[cfg(target_os = "none")]
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};
use heapless::Vec;
#[cfg(target_os = "none")]
use log::{info, warn};
#[cfg(target_os = "none")]
use static_cell::StaticCell;

#[cfg(target_os = "none")]
extern crate alloc;
#[cfg(target_os = "none")]
use alloc::string::String;
#[cfg(target_os = "none")]
use embassy_futures::select::{select4, Either4};

#[cfg(target_os = "none")]
use device_envoy_core::wifi_auto::WifiAutoError;
use device_envoy_core::wifi_auto::{
    HtmlBuffer, WifiAutoPersistedState, WifiCredentials, WifiStartMode,
};
pub use device_envoy_core::wifi_auto::{WifiAuto, WifiAutoEvent, WifiAutoField, WifiStack};

const MAX_WIFI_AUTO_FIELDS: usize = 8;

#[cfg_attr(feature = "host", allow(dead_code))]
enum WifiAutoStorage {
    #[cfg(target_os = "none")]
    Flash(RefCell<FlashBlockEsp>),
    #[cfg(not(target_os = "none"))]
    Memory(RefCell<WifiAutoPersistedState>),
}

/// A device abstraction that connects ESP devices with WiFi to the Internet and, when needed,
/// creates a temporary WiFi network to enter credentials.
///
/// `WifiAutoEsp` handles WiFi connections end-to-end. It normally connects using
/// a saved WiFi network name (SSID) and password. If those values are missing
/// or invalid, it temporarily creates its own WiFi network (a captive
/// portal) and hosts a web form where the user can enter local WiFi
/// ssid and password.
///
/// The typical usage pattern is:
///
/// 1. Ensure your hardware includes a button wired to a GPIO. The button can be used during boot to force captive-portal mode.
/// 2. Construct a [`ButtonEsp`](crate::button::ButtonEsp) to control the physical button.
/// 3. Construct a [`FlashBlockEsp`] to store WiFi credentials.
/// 4. Use [`WifiAutoEsp::new`] to construct a `WifiAutoEsp`.
/// 5. Use [`WifiAuto::connect`] to connect to WiFi while optionally showing status.
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
/// use device_envoy_esp::{
///     Result,
///     button::{ButtonEsp, PressedTo},
///     flash_block::FlashBlockEsp,
///     wifi_auto::{WifiAuto as _, WifiAutoEvent, WifiAutoEsp},
/// };
/// use embassy_time::Duration;
/// use log::info;
///
/// async fn connect_wifi(
///     spawner: embassy_executor::Spawner,
///     p: esp_hal::peripherals::Peripherals,
/// ) -> Result<core::convert::Infallible> {
///     // Set up ButtonEsp to control the physical button.
///     let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
///
///     // Set up flash storage for WiFi credentials.
///     let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
///
///     // Construct WifiAutoEsp.
///     let wifi_auto = WifiAutoEsp::new(
///         p.WIFI,
///         wifi_auto_flash_block,
///         "DeviceEnvoySetup", // Captive-portal SSID
///         [],                 // Any extra fields
///         spawner,
///     )?;
///
///     // Connect (logging status as we go).
///     let stack = wifi_auto
///         .connect(&mut button6, |wifi_auto_event| async move {
///             match wifi_auto_event {
///                 WifiAutoEvent::CaptivePortalReady => {
///                     info!("Captive portal ready");
///                 }
///                 WifiAutoEvent::Connecting { .. } => {
///                     info!("Connecting to WiFi");
///                 }
///                 WifiAutoEvent::ConnectionFailed => {
///                     info!("WiFi connection failed");
///                 }
///             }
///             Ok(())
///         })
///         .await?;
///
///     info!("WiFi connected");
///
///     loop {
///         if let Ok(addresses) = stack.dns_query("google.com", embassy_net::dns::DnsQueryType::A).await {
///             info!("google.com: {:?}", addresses);
///         } else {
///             info!("google.com: lookup failed");
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
/// - The WiFi subsystem may reset as it switches between normal WiFi operation and
///   hosting its own temporary WiFi network.
/// - Your code should tolerate these resets.
///   Initializing LEDs or displays before WiFi is fine; just be aware they may be
///   momentarily disrupted during mode changes.
///
/// ## WiFi limitations
///
/// - Only standard SSID/password 2.4 GHz WiFi networks are supported.
///
/// ## Hardware model
///
/// On ESP devices, the WiFi radio is integrated on-chip.
#[cfg_attr(feature = "host", allow(dead_code))]
pub struct WifiAutoEsp<'a> {
    captive_portal_ssid: &'static str,
    fields: Vec<&'a dyn WifiAutoField<Error = crate::Error>, MAX_WIFI_AUTO_FIELDS>,
    storage: WifiAutoStorage,
    #[cfg(target_os = "none")]
    wifi: RefCell<Option<esp_hal::peripherals::WIFI<'static>>>,
    #[cfg(target_os = "none")]
    spawner: embassy_executor::Spawner,
}

#[cfg_attr(feature = "host", allow(dead_code))]
impl<'a> WifiAutoEsp<'a> {
    /// Create a new setup flow configuration.
    ///
    /// See the [WifiAutoEsp struct example](Self) for usage.
    #[cfg(not(target_os = "none"))]
    #[must_use]
    pub fn new(
        captive_portal_ssid: &'static str,
        custom_fields: &'a [&'a dyn WifiAutoField<Error = crate::Error>],
    ) -> Self {
        let mut fields = Vec::new();
        let mut field_index = 0;
        while field_index < custom_fields.len() {
            assert!(
                fields.push(custom_fields[field_index]).is_ok(),
                "custom_fields supports up to {MAX_WIFI_AUTO_FIELDS} entries"
            );
            field_index += 1;
        }
        Self {
            captive_portal_ssid,
            fields,
            storage: WifiAutoStorage::Memory(RefCell::new(WifiAutoPersistedState {
                wifi_credentials: None,
                wifi_start_mode: WifiStartMode::Client,
            })),
        }
    }

    /// Create a setup flow backed by persistent flash storage.
    ///
    /// This constructor is available on embedded targets (`target_os = "none"`).
    ///
    /// # Errors
    ///
    /// Returns an error if stored state cannot be loaded/saved while resolving startup mode.
    #[cfg(target_os = "none")]
    #[must_use]
    pub fn new<const N: usize>(
        wifi: esp_hal::peripherals::WIFI<'static>,
        wifi_auto_flash_block: FlashBlockEsp,
        captive_portal_ssid: &'static str,
        custom_fields: [&'a dyn WifiAutoField<Error = crate::Error>; N],
        spawner: embassy_executor::Spawner,
    ) -> Result<Self> {
        assert!(
            N <= MAX_WIFI_AUTO_FIELDS,
            "custom_fields supports up to {MAX_WIFI_AUTO_FIELDS} entries"
        );
        let fields =
            Vec::from_slice(&custom_fields).expect("custom_fields length was validated above");
        let wifi_auto = Self {
            captive_portal_ssid,
            fields,
            storage: WifiAutoStorage::Flash(RefCell::new(wifi_auto_flash_block)),
            wifi: RefCell::new(Some(wifi)),
            spawner,
        };

        let wifi_start_mode = wifi_auto.start_mode()?;
        let custom_fields_satisfied = wifi_auto.custom_fields_satisfied()?;
        let has_persisted_credentials = wifi_auto.load_persisted_credentials()?.is_some();
        if device_envoy_core::wifi_auto::should_enter_captive_portal(
            wifi_start_mode,
            false,
            has_persisted_credentials,
            custom_fields_satisfied,
        ) {
            wifi_auto.set_start_mode(WifiStartMode::CaptivePortal)?;
        }

        Ok(wifi_auto)
    }

    /// Return the SSID shown in captive-portal mode.
    ///
    /// See the [WifiAutoEsp struct example](Self) for usage.
    #[must_use]
    pub(crate) const fn captive_portal_ssid(&self) -> &'static str {
        self.captive_portal_ssid
    }

    /// Render the captive-portal HTML page.
    ///
    /// See the [WifiAutoEsp struct example](Self) for usage.
    #[must_use]
    pub(crate) fn generate_config_page(&self, defaults: Option<&WifiCredentials>) -> HtmlBuffer {
        device_envoy_core::wifi_auto::generate_config_page(defaults, self.fields.as_slice())
    }

    /// Parse a raw HTTP POST request into credentials and apply field parsers.
    ///
    /// Returns `None` when the request body is malformed, the SSID is missing,
    /// or any custom field parser reports an error.
    ///
    /// See the [WifiAutoEsp struct example](Self) for usage.
    #[must_use]
    pub(crate) fn parse_post(
        &self,
        request: &str,
        defaults: Option<&WifiCredentials>,
    ) -> Option<WifiCredentials> {
        device_envoy_core::wifi_auto::parse_post(request, defaults, self.fields.as_slice())
    }

    /// Load persisted Wi-Fi credentials from storage.
    ///
    /// Returns `None` when no credentials have been persisted yet.
    pub(crate) fn load_persisted_credentials(&self) -> Result<Option<WifiCredentials>> {
        let wifi_auto_persisted_state = self.load_persisted_state()?;
        Ok(wifi_auto_persisted_state.wifi_credentials)
    }

    /// Persist Wi-Fi credentials to storage.
    pub(crate) fn persist_credentials(&self, wifi_credentials: &WifiCredentials) -> Result<()> {
        let mut wifi_auto_persisted_state = self.load_persisted_state()?;
        wifi_auto_persisted_state.wifi_credentials = Some(wifi_credentials.clone());
        self.store_persisted_state(&wifi_auto_persisted_state)
    }

    /// Load the persisted startup mode.
    pub(crate) fn start_mode(&self) -> Result<WifiStartMode> {
        let wifi_auto_persisted_state = self.load_persisted_state()?;
        Ok(wifi_auto_persisted_state.wifi_start_mode)
    }

    /// Persist the startup mode.
    pub(crate) fn set_start_mode(&self, wifi_start_mode: WifiStartMode) -> Result<()> {
        let mut wifi_auto_persisted_state = self.load_persisted_state()?;
        wifi_auto_persisted_state.wifi_start_mode = wifi_start_mode;
        self.store_persisted_state(&wifi_auto_persisted_state)
    }

    /// Force captive-portal mode when a sampled press state is `true`.
    ///
    /// Returns `true` if startup mode was changed to [`WifiStartMode::CaptivePortal`].
    pub(crate) fn force_captive_portal_if_pressed_state(&self, is_pressed: bool) -> Result<bool> {
        if is_pressed {
            self.set_start_mode(WifiStartMode::CaptivePortal)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check whether every custom field reports a satisfied state.
    ///
    /// See the [WifiAutoEsp struct example](Self) for usage.
    pub(crate) fn custom_fields_satisfied(&self) -> Result<bool> {
        for field in self.fields.as_slice() {
            if !field.is_satisfied()? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn load_persisted_state(&self) -> Result<WifiAutoPersistedState> {
        match &self.storage {
            #[cfg(target_os = "none")]
            WifiAutoStorage::Flash(wifi_auto_flash_block) => {
                let wifi_auto_persisted_state = wifi_auto_flash_block
                    .borrow_mut()
                    .load::<WifiAutoPersistedState>()?
                    .unwrap_or_default();
                Ok(wifi_auto_persisted_state)
            }
            #[cfg(not(target_os = "none"))]
            WifiAutoStorage::Memory(wifi_auto_persisted_state) => {
                Ok(wifi_auto_persisted_state.borrow().clone())
            }
        }
    }

    fn store_persisted_state(
        &self,
        wifi_auto_persisted_state: &WifiAutoPersistedState,
    ) -> Result<()> {
        match &self.storage {
            #[cfg(target_os = "none")]
            WifiAutoStorage::Flash(wifi_auto_flash_block) => wifi_auto_flash_block
                .borrow_mut()
                .save(wifi_auto_persisted_state),
            #[cfg(not(target_os = "none"))]
            WifiAutoStorage::Memory(stored_state) => {
                *stored_state.borrow_mut() = wifi_auto_persisted_state.clone();
                Ok(())
            }
        }
    }

    #[cfg(target_os = "none")]
    async fn connect_inner<OnEvent, OnEventFuture>(
        &self,
        force_captive_portal: bool,
        mut on_event: OnEvent,
    ) -> Result<WifiStack>
    where
        OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
        OnEventFuture: Future<Output = Result<()>>,
    {
        Self::initialize_wifi_heap_once();
        let wifi = self
            .wifi
            .borrow_mut()
            .take()
            .ok_or_else(|| crate::Error::from(WifiAutoError::StorageCorrupted))?;
        let spawner = self.spawner;

        static ESP_RADIO_CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
        let esp_radio_controller = ESP_RADIO_CONTROLLER.init(esp_radio::init()?);
        let (mut wifi_controller, interfaces) = esp_radio::wifi::new(
            esp_radio_controller,
            wifi,
            esp_radio::wifi::Config::default(),
        )?;

        const TRY_COUNT: u8 = 10;
        struct EspWifiAutoBackend<'a, 'b> {
            wifi_auto: &'b WifiAutoEsp<'b>,
            wifi_controller: &'a mut esp_radio::wifi::WifiController<'static>,
            access_point_device: Option<esp_radio::wifi::WifiDevice<'static>>,
            station_device: Option<esp_radio::wifi::WifiDevice<'static>>,
            spawner: embassy_executor::Spawner,
            connected_stack: Option<&'static Stack<'static>>,
            force_captive_portal: bool,
        }
        impl device_envoy_core::wifi_auto::WifiAutoBackend for EspWifiAutoBackend<'_, '_> {
            type Error = crate::Error;

            fn force_captive_portal(&self) -> bool {
                self.force_captive_portal
            }

            fn try_count(&self) -> u8 {
                TRY_COUNT
            }

            fn load_start_mode(&self) -> crate::Result<WifiStartMode> {
                self.wifi_auto.start_mode()
            }

            fn custom_fields_satisfied(&self) -> crate::Result<bool> {
                self.wifi_auto.custom_fields_satisfied()
            }

            fn load_persisted_credentials(&self) -> crate::Result<Option<WifiCredentials>> {
                self.wifi_auto.load_persisted_credentials()
            }

            fn persist_credentials(&self, wifi_credentials: &WifiCredentials) -> crate::Result<()> {
                self.wifi_auto.persist_credentials(wifi_credentials)
            }

            fn set_start_mode(&self, wifi_start_mode: WifiStartMode) -> crate::Result<()> {
                self.wifi_auto.set_start_mode(wifi_start_mode)
            }

            fn run_captive_portal(
                &mut self,
            ) -> impl Future<Output = crate::Result<WifiCredentials>> + '_ {
                async move {
                    let access_point_device = self
                        .access_point_device
                        .take()
                        .expect("captive portal should run at most once");
                    self.wifi_auto
                        .run_captive_portal(self.wifi_controller, access_point_device)
                        .await
                }
            }

            fn on_connect_attempt(
                &mut self,
                try_index: u8,
            ) -> impl Future<Output = crate::Result<bool>> + '_ {
                async move {
                    info!("wifi_auto connect attempt {}/{}", try_index + 1, TRY_COUNT);
                    match self.wifi_controller.connect_async().await {
                        Ok(()) => {
                            info!("wifi_auto client connected on try {}", try_index + 1);
                            let station_device = self
                                .station_device
                                .take()
                                .expect("station device should be consumed only once");
                            static STA_STACK_RESOURCES: StaticCell<StackResources<4>> =
                                StaticCell::new();
                            let (stack, runner) = embassy_net::new(
                                station_device,
                                Config::dhcpv4(Default::default()),
                                STA_STACK_RESOURCES.init(StackResources::new()),
                                0xD1D1_C1C1_5151_4242,
                            );
                            static STA_STACK: StaticCell<Stack<'static>> = StaticCell::new();
                            let stack = STA_STACK.init(stack);
                            self.spawner.spawn(wifi_auto_net_task(runner))?;
                            self.connected_stack = Some(stack);
                            Ok(true)
                        }
                        Err(error) => {
                            warn!(
                                "wifi_auto connect failed on try {}: {:?}",
                                try_index + 1,
                                error
                            );
                            warn!(
                                "wifi_auto hint: verify AP is 2.4GHz and security mode is compatible (for example WPA2/WPA3 mixed, not WPA3-only)"
                            );
                            Timer::after(Duration::from_millis(800)).await;
                            Ok(false)
                        }
                    }
                }
            }

            fn on_resolved_credentials(
                &mut self,
                wifi_credentials: &WifiCredentials,
            ) -> impl Future<Output = crate::Result<()>> + '_ {
                let wifi_credentials = wifi_credentials.clone();
                async move {
                    let wifi_client_config = esp_radio::wifi::ClientConfig::default()
                        .with_ssid(String::from(wifi_credentials.ssid.as_str()))
                        .with_password(String::from(wifi_credentials.password.as_str()));
                    self.wifi_controller
                        .set_config(&esp_radio::wifi::ModeConfig::Client(wifi_client_config))?;
                    info!(
                        "wifi_auto client config set: ssid='{}' password_len={}",
                        wifi_credentials.ssid.as_str(),
                        wifi_credentials.password.len()
                    );
                    self.wifi_controller.start_async().await?;
                    Ok(())
                }
            }
        }

        let mut wifi_auto_backend = EspWifiAutoBackend {
            wifi_auto: self,
            wifi_controller: &mut wifi_controller,
            access_point_device: Some(interfaces.ap),
            station_device: Some(interfaces.sta),
            spawner,
            connected_stack: None,
            force_captive_portal,
        };
        let connected = device_envoy_core::wifi_auto::connect_with_backend(
            &mut wifi_auto_backend,
            &mut on_event,
        )
        .await?;
        if !connected {
            warn!(
                "wifi_auto failed to connect after {} attempts; switching startup mode to CaptivePortal and resetting",
                TRY_COUNT
            );
            self.set_start_mode(WifiStartMode::CaptivePortal)?;
            info!("wifi_auto wrote startup mode CaptivePortal to storage");
            let _ = wifi_auto_backend.wifi_controller.stop_async().await;
            info!("wifi_auto resetting in 1 second");
            Timer::after(Duration::from_secs(1)).await;
            esp_hal::system::software_reset();
        }

        let stack = wifi_auto_backend
            .connected_stack
            .expect("stack should be initialized after successful connect");
        self.set_start_mode(WifiStartMode::Client)?;

        // Keep the Wi-Fi controller alive for the lifetime of the returned stack.
        // Dropping it would shut Wi-Fi down.
        core::mem::forget(wifi_controller);

        Ok(stack)
    }

    #[cfg(target_os = "none")]
    async fn run_captive_portal(
        &self,
        wifi_controller: &mut esp_radio::wifi::WifiController<'static>,
        ap_device: esp_radio::wifi::WifiDevice<'static>,
    ) -> Result<WifiCredentials> {
        let access_point_config = esp_radio::wifi::AccessPointConfig::default()
            .with_ssid(String::from(self.captive_portal_ssid()));
        wifi_controller.set_config(&esp_radio::wifi::ModeConfig::AccessPoint(
            access_point_config,
        ))?;
        wifi_controller.start_async().await?;

        static AP_STACK_RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
        let mut dns_servers = heapless::Vec::new();
        dns_servers
            .push(Ipv4Address::new(192, 168, 4, 1))
            .expect("single DNS entry must fit");

        let (ap_stack, mut ap_runner) = embassy_net::new(
            ap_device,
            Config::ipv4_static(StaticConfigV4 {
                address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 4, 1), 24),
                gateway: Some(Ipv4Address::new(192, 168, 4, 1)),
                dns_servers,
            }),
            AP_STACK_RESOURCES.init(StackResources::new()),
            0xA0A0_C1C1_5151_4242,
        );

        info!(
            "wifi_auto captive portal ready: connect to '{}' and open http://192.168.4.1",
            self.captive_portal_ssid()
        );

        let wait_for_submission = async {
            let wifi_credentials = self.wait_for_portal_submission(ap_stack).await?;
            Ok::<WifiCredentials, crate::Error>(wifi_credentials)
        };
        let dhcp_server = dhcp::dhcp_server_task(
            ap_stack,
            Ipv4Address::new(192, 168, 4, 1),
            Ipv4Address::new(255, 255, 255, 0),
            Ipv4Address::new(192, 168, 4, 2),
            32,
        );
        let dns_server = dns::dns_server_task(ap_stack, Ipv4Address::new(192, 168, 4, 1));

        let portal_wifi_credentials = match select4(
            ap_runner.run(),
            wait_for_submission,
            dhcp_server,
            dns_server,
        )
        .await
        {
            Either4::First(never) => match never {},
            Either4::Second(result) => result?,
            Either4::Third(never) => match never {},
            Either4::Fourth(never) => match never {},
        };

        let _ = wifi_controller.stop_async().await;
        Ok(portal_wifi_credentials)
    }

    #[cfg(target_os = "none")]
    async fn wait_for_portal_submission(&self, stack: Stack<'static>) -> Result<WifiCredentials> {
        loop {
            let mut receive_buffer = [0u8; 2048];
            let mut transmit_buffer = [0u8; 2048];
            let mut socket =
                embassy_net::tcp::TcpSocket::new(stack, &mut receive_buffer, &mut transmit_buffer);

            if socket.accept(80).await.is_err() {
                continue;
            }

            let mut request_buffer = [0u8; 2048];
            let read_len = match socket.read(&mut request_buffer).await {
                Ok(read_len) => read_len,
                Err(_) => {
                    socket.close();
                    continue;
                }
            };
            if read_len == 0 {
                socket.close();
                continue;
            }

            let request = match core::str::from_utf8(&request_buffer[..read_len]) {
                Ok(request) => request,
                Err(_) => {
                    socket.close();
                    continue;
                }
            };

            if request.starts_with("POST ") {
                let defaults_wifi_credentials = self.load_persisted_credentials()?;
                if let Some(wifi_credentials) =
                    self.parse_post(request, defaults_wifi_credentials.as_ref())
                {
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nSaved credentials. Device is connecting now.\r\n";
                    let _ = Self::socket_write_all(&mut socket, response.as_bytes()).await;
                    socket.close();
                    return Ok(wifi_credentials);
                }
            }

            let defaults_wifi_credentials = self.load_persisted_credentials()?;
            let page = self.generate_config_page(defaults_wifi_credentials.as_ref());
            let _ = Self::socket_write_all(&mut socket, page.as_bytes()).await;
            socket.close();
        }
    }

    #[cfg(target_os = "none")]
    async fn socket_write_all(
        socket: &mut embassy_net::tcp::TcpSocket<'_>,
        bytes: &[u8],
    ) -> core::result::Result<(), embassy_net::tcp::Error> {
        let mut write_index = 0usize;
        while write_index < bytes.len() {
            let written_count = socket.write(&bytes[write_index..]).await?;
            if written_count == 0 {
                break;
            }
            write_index += written_count;
        }
        socket.flush().await
    }

    #[cfg(target_os = "none")]
    fn initialize_wifi_heap_once() {
        static WIFI_HEAP_INITIALIZED: AtomicBool = AtomicBool::new(false);
        if WIFI_HEAP_INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            const WIFI_HEAP_BYTES: usize = 72 * 1024;
            esp_alloc::heap_allocator!(size: WIFI_HEAP_BYTES);
        }
    }
}

#[cfg(target_os = "none")]
impl device_envoy_core::wifi_auto::WifiAuto for WifiAutoEsp<'_> {
    type Error = crate::Error;

    /// Connects to WiFi (if possible), reports status, and returns the
    /// network stack, consuming the `WifiAutoEsp`.
    ///
    /// See the [WifiAutoEsp struct example](Self) for a usage example.
    ///
    /// This method does not return until WiFi is connected. It may briefly
    /// restart WiFi while switching between normal WiFi operation
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
    /// The example on the [`WifiAutoEsp`] struct shows simple logging.
    ///
    /// # Example 1: No-op event handler
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// # use device_envoy_esp::{
    /// #     Result,
    /// #     button::{ButtonEsp, PressedTo},
    /// #     flash_block::FlashBlockEsp,
    /// #     wifi_auto::{WifiAuto as _, WifiAutoEsp},
    /// # };
    /// # async fn example(
    /// #     spawner: embassy_executor::Spawner,
    /// #     p: esp_hal::peripherals::Peripherals,
    /// # ) -> Result<()> {
    /// # let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    /// # let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    /// # let wifi_auto = WifiAutoEsp::new(
    /// #     p.WIFI,
    /// #     wifi_auto_flash_block,
    /// #     "DeviceEnvoySetup",
    /// #     [],
    /// #     spawner,
    /// # )?;
    /// let _stack = wifi_auto
    ///     .connect(&mut button6, |_event| async move { Ok(()) })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example 2: Using a display to show status
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// # use device_envoy_esp::{
    /// #     Result,
    /// #     button::{ButtonEsp, PressedTo},
    /// #     flash_block::FlashBlockEsp,
    /// #     led_strip::colors,
    /// #     wifi_auto::{WifiAuto as _, WifiAutoEvent, WifiAutoEsp},
    /// # };
    /// # use smart_leds::RGB8;
    /// # async fn example(
    /// #     spawner: embassy_executor::Spawner,
    /// #     p: esp_hal::peripherals::Peripherals,
    /// # ) -> Result<()> {
    /// # let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    /// # let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    /// # let wifi_auto = WifiAutoEsp::new(
    /// #     p.WIFI,
    /// #     wifi_auto_flash_block,
    /// #     "DeviceEnvoySetup",
    /// #     [],
    /// #     spawner,
    /// # )?;
    /// # struct Led8x12;
    /// # impl Led8x12 {
    /// #     async fn write_text(&self, _text: &str, _colors: &[RGB8]) -> Result<()> { Ok(()) }
    /// # }
    /// # async fn show_animated_dots(_led8x12: &Led8x12) -> Result<()> { Ok(()) }
    /// # const COLORS: &[RGB8] = &[colors::WHITE];
    /// # let led8x12 = Led8x12;
    /// // Keep a reference so the handler can reuse the display across events.
    /// let led8x12_ref = &led8x12;
    /// let stack = wifi_auto
    ///     .connect(&mut button6, |wifi_auto_event| async move {
    ///         match wifi_auto_event {
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
    /// # Ok(())
    /// # }
    /// ```
    async fn connect<OnEvent, OnEventFuture>(
        self,
        button: &mut impl Button,
        on_event: OnEvent,
    ) -> Result<WifiStack>
    where
        OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
        OnEventFuture: Future<Output = Result<()>>,
    {
        let force_captive_portal = button.is_pressed();
        if self.force_captive_portal_if_pressed_state(force_captive_portal)? {
            info!("wifi_auto force-captive-portal requested via button");
        }
        self.connect_inner(force_captive_portal, on_event).await
    }
}

#[cfg(target_os = "none")]
#[embassy_executor::task]
async fn wifi_auto_net_task(
    mut runner: embassy_net::Runner<'static, esp_radio::wifi::WifiDevice<'static>>,
) -> ! {
    runner.run().await
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::{WifiAutoEsp, WifiStartMode};

    #[test]
    fn force_captive_portal_when_pressed_sets_mode() {
        let wifi_auto = WifiAutoEsp::new("PortalSsid", &[]);
        let changed = wifi_auto
            .force_captive_portal_if_pressed_state(true)
            .expect("force should succeed");
        assert!(changed);
        assert_eq!(
            wifi_auto.start_mode().expect("start mode should load"),
            WifiStartMode::CaptivePortal
        );
    }

    #[test]
    fn force_captive_portal_when_not_pressed_keeps_mode() {
        let wifi_auto = WifiAutoEsp::new("PortalSsid", &[]);
        let changed = wifi_auto
            .force_captive_portal_if_pressed_state(false)
            .expect("force should succeed");
        assert!(!changed);
        assert_eq!(
            wifi_auto.start_mode().expect("start mode should load"),
            WifiStartMode::Client
        );
    }
}
