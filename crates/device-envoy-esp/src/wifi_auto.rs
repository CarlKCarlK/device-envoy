//! A device abstraction for automatic Wi-Fi credential collection workflows.
//! See [`WifiAuto`] for the main struct and usage.

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
use crate::button::{Button as _, ButtonEsp, PressedTo};
#[cfg(target_os = "none")]
use crate::flash_block::{FlashBlock as _, FlashBlockEsp};
use crate::Result;
#[cfg(target_os = "none")]
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};
#[cfg(target_os = "none")]
use esp_hal::gpio::InputPin;
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

pub use device_envoy_core::wifi_auto::{
    FormData, HtmlBuffer, WifiAutoError, WifiAutoEvent, WifiAutoField, WifiAutoPersistedState,
    WifiCredentials, WifiStartMode,
};

const MAX_WIFI_AUTO_FIELDS: usize = 8;

enum WifiAutoStorage {
    #[cfg(target_os = "none")]
    Flash(RefCell<FlashBlockEsp>),
    Memory(RefCell<WifiAutoPersistedState>),
}

/// Captive-portal workflow configuration and parsing helpers.
///
/// This is the ESP32 port of the Pico `wifi_auto` setup surface for parsing
/// form submissions and rendering setup pages. Networking/backend connection
/// orchestration is intentionally separate from this type.
pub struct WifiAuto<'a> {
    captive_portal_ssid: &'static str,
    fields: Vec<&'a dyn WifiAutoField<Error = crate::Error>, MAX_WIFI_AUTO_FIELDS>,
    storage: WifiAutoStorage,
    #[cfg(target_os = "none")]
    wifi: RefCell<Option<esp_hal::peripherals::WIFI<'static>>>,
    #[cfg(target_os = "none")]
    spawner: embassy_executor::Spawner,
    #[cfg(target_os = "none")]
    force_captive_portal: bool,
    #[cfg(target_os = "none")]
    button: RefCell<Option<ButtonEsp<'static>>>,
}

impl<'a> WifiAuto<'a> {
    /// Create a new setup flow configuration.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
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

    /// Create an in-memory setup flow for embedded targets with runtime Wi-Fi resources.
    ///
    /// This constructor is available on embedded targets (`target_os = "none"`).
    ///
    /// # Errors
    ///
    /// Returns an error if custom field validation fails while resolving startup mode.
    #[cfg(target_os = "none")]
    #[must_use]
    pub fn new_in_memory<const N: usize>(
        captive_portal_ssid: &'static str,
        wifi: esp_hal::peripherals::WIFI<'static>,
        button_pin: impl InputPin + 'static,
        button_pressed_to: PressedTo,
        custom_fields: [&'a dyn WifiAutoField<Error = crate::Error>; N],
        spawner: embassy_executor::Spawner,
    ) -> Result<Self> {
        assert!(
            N <= MAX_WIFI_AUTO_FIELDS,
            "custom_fields supports up to {MAX_WIFI_AUTO_FIELDS} entries"
        );
        let fields =
            Vec::from_slice(&custom_fields).expect("custom_fields length was validated above");
        let button = ButtonEsp::new(button_pin, button_pressed_to);
        let force_captive_portal = button.is_pressed();
        let wifi_auto = Self {
            captive_portal_ssid,
            fields,
            storage: WifiAutoStorage::Memory(RefCell::new(WifiAutoPersistedState {
                wifi_credentials: None,
                wifi_start_mode: WifiStartMode::Client,
            })),
            wifi: RefCell::new(Some(wifi)),
            spawner,
            force_captive_portal,
            button: RefCell::new(Some(button)),
        };

        let wifi_start_mode = wifi_auto.start_mode()?;
        let custom_fields_satisfied = wifi_auto.custom_fields_satisfied()?;
        let has_persisted_credentials = wifi_auto.load_persisted_credentials()?.is_some();
        if device_envoy_core::wifi_auto::should_enter_captive_portal(
            wifi_start_mode,
            force_captive_portal,
            has_persisted_credentials,
            custom_fields_satisfied,
        ) {
            wifi_auto.set_start_mode(WifiStartMode::CaptivePortal)?;
        }

        Ok(wifi_auto)
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
        button_pin: impl InputPin + 'static,
        button_pressed_to: PressedTo,
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
        let button = ButtonEsp::new(button_pin, button_pressed_to);
        let force_captive_portal = button.is_pressed();
        let wifi_auto = Self {
            captive_portal_ssid,
            fields,
            storage: WifiAutoStorage::Flash(RefCell::new(wifi_auto_flash_block)),
            wifi: RefCell::new(Some(wifi)),
            spawner,
            force_captive_portal,
            button: RefCell::new(Some(button)),
        };

        let wifi_start_mode = wifi_auto.start_mode()?;
        let custom_fields_satisfied = wifi_auto.custom_fields_satisfied()?;
        let has_persisted_credentials = wifi_auto.load_persisted_credentials()?.is_some();
        if device_envoy_core::wifi_auto::should_enter_captive_portal(
            wifi_start_mode,
            force_captive_portal,
            has_persisted_credentials,
            custom_fields_satisfied,
        ) {
            wifi_auto.set_start_mode(WifiStartMode::CaptivePortal)?;
        }

        Ok(wifi_auto)
    }

    /// Return the SSID shown in captive-portal mode.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    #[must_use]
    pub const fn captive_portal_ssid(&self) -> &'static str {
        self.captive_portal_ssid
    }

    /// Render the captive-portal HTML page.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    #[must_use]
    pub fn generate_config_page(&self, defaults: Option<&WifiCredentials>) -> HtmlBuffer {
        device_envoy_core::wifi_auto::generate_config_page(defaults, self.fields.as_slice())
    }

    /// Parse a raw HTTP POST request into credentials and apply field parsers.
    ///
    /// Returns `None` when the request body is malformed, the SSID is missing,
    /// or any custom field parser reports an error.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    #[must_use]
    pub fn parse_post(&self, request: &str) -> Option<WifiCredentials> {
        device_envoy_core::wifi_auto::parse_post(request, self.fields.as_slice())
    }

    /// Load persisted Wi-Fi credentials from storage.
    ///
    /// Returns `None` when no credentials have been persisted yet.
    pub fn load_persisted_credentials(&self) -> Result<Option<WifiCredentials>> {
        let wifi_auto_persisted_state = self.load_persisted_state()?;
        Ok(wifi_auto_persisted_state.wifi_credentials)
    }

    /// Persist Wi-Fi credentials to storage.
    pub fn persist_credentials(&self, wifi_credentials: &WifiCredentials) -> Result<()> {
        let mut wifi_auto_persisted_state = self.load_persisted_state()?;
        wifi_auto_persisted_state.wifi_credentials = Some(wifi_credentials.clone());
        self.store_persisted_state(&wifi_auto_persisted_state)
    }

    /// Clear persisted Wi-Fi credentials from storage.
    pub fn clear_persisted_credentials(&self) -> Result<()> {
        let mut wifi_auto_persisted_state = self.load_persisted_state()?;
        wifi_auto_persisted_state.wifi_credentials = None;
        self.store_persisted_state(&wifi_auto_persisted_state)
    }

    /// Load the persisted startup mode.
    pub fn start_mode(&self) -> Result<WifiStartMode> {
        let wifi_auto_persisted_state = self.load_persisted_state()?;
        Ok(wifi_auto_persisted_state.wifi_start_mode)
    }

    /// Persist the startup mode.
    pub fn set_start_mode(&self, wifi_start_mode: WifiStartMode) -> Result<()> {
        let mut wifi_auto_persisted_state = self.load_persisted_state()?;
        wifi_auto_persisted_state.wifi_start_mode = wifi_start_mode;
        self.store_persisted_state(&wifi_auto_persisted_state)
    }

    /// Force captive-portal mode when a sampled press state is `true`.
    ///
    /// Returns `true` if startup mode was changed to [`WifiStartMode::CaptivePortal`].
    pub fn force_captive_portal_if_pressed_state(&self, is_pressed: bool) -> Result<bool> {
        if is_pressed {
            self.set_start_mode(WifiStartMode::CaptivePortal)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Force captive-portal mode if the button is currently pressed.
    ///
    /// Returns `true` if startup mode was changed to [`WifiStartMode::CaptivePortal`].
    #[cfg(target_os = "none")]
    pub fn force_captive_portal_if_pressed(&self, button: &ButtonEsp<'_>) -> Result<bool> {
        self.force_captive_portal_if_pressed_state(button.is_pressed())
    }

    /// Check whether every custom field reports a satisfied state.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    pub fn custom_fields_satisfied(&self) -> Result<bool> {
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
            WifiAutoStorage::Memory(stored_state) => {
                *stored_state.borrow_mut() = wifi_auto_persisted_state.clone();
                Ok(())
            }
        }
    }

    #[cfg(target_os = "none")]
    device_envoy_core::__impl_wifi_auto_connect! {
    /// Connect using persisted credentials or captive-portal setup flow.
    fn connect(&self as wifi_auto, on_event) -> Result<(&'static Stack<'static>, ButtonEsp<'static>)> {
        Self::initialize_wifi_heap_once();
        let wifi = wifi_auto
            .wifi
            .borrow_mut()
            .take()
            .ok_or_else(|| crate::Error::from(WifiAutoError::StorageCorrupted))?;
        let button = wifi_auto
            .button
            .borrow_mut()
            .take()
            .ok_or_else(|| crate::Error::from(WifiAutoError::StorageCorrupted))?;
        let spawner = wifi_auto.spawner;
        let force_captive_portal = wifi_auto.force_captive_portal;

        static ESP_RADIO_CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
        let esp_radio_controller = ESP_RADIO_CONTROLLER.init(esp_radio::init()?);
        let (mut wifi_controller, interfaces) = esp_radio::wifi::new(
            esp_radio_controller,
            wifi,
            esp_radio::wifi::Config::default(),
        )?;

        const TRY_COUNT: u8 = 10;
        struct EspWifiAutoBackend<'a, 'b> {
            wifi_auto: &'b WifiAuto<'b>,
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

            fn persist_credentials(
                &self,
                wifi_credentials: &WifiCredentials,
            ) -> crate::Result<()> {
                self.wifi_auto.persist_credentials(wifi_credentials)
            }

            fn set_start_mode(&self, wifi_start_mode: WifiStartMode) -> crate::Result<()> {
                self.wifi_auto.set_start_mode(wifi_start_mode)
            }

            fn run_captive_portal(&mut self) -> impl Future<Output = crate::Result<WifiCredentials>> + '_ {
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
            wifi_auto,
            wifi_controller: &mut wifi_controller,
            access_point_device: Some(interfaces.ap),
            station_device: Some(interfaces.sta),
            spawner,
            connected_stack: None,
            force_captive_portal,
        };
        let connected =
            device_envoy_core::wifi_auto::connect_with_backend(&mut wifi_auto_backend, &mut on_event)
                .await?;
        if !connected {
            warn!(
                "wifi_auto failed to connect after {} attempts; switching startup mode to CaptivePortal and resetting",
                TRY_COUNT
            );
            wifi_auto.set_start_mode(WifiStartMode::CaptivePortal)?;
            info!("wifi_auto wrote startup mode CaptivePortal to storage");
            let _ = wifi_auto_backend.wifi_controller.stop_async().await;
            info!("wifi_auto resetting in 1 second");
            Timer::after(Duration::from_secs(1)).await;
            esp_hal::system::software_reset();
        }

        let stack = wifi_auto_backend
            .connected_stack
            .expect("stack should be initialized after successful connect");
        wifi_auto.set_start_mode(WifiStartMode::Client)?;

        // Keep the Wi-Fi controller alive for the lifetime of the returned stack.
        // Dropping it would shut Wi-Fi down.
        core::mem::forget(wifi_controller);

        Ok((stack, button))
    }
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
                if let Some(wifi_credentials) = self.parse_post(request) {
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
#[embassy_executor::task]
async fn wifi_auto_net_task(
    mut runner: embassy_net::Runner<'static, esp_radio::wifi::WifiDevice<'static>>,
) -> ! {
    runner.run().await
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::{WifiAuto, WifiStartMode};

    #[test]
    fn force_captive_portal_when_pressed_sets_mode() {
        let wifi_auto = WifiAuto::new("PortalSsid", &[]);
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
        let wifi_auto = WifiAuto::new("PortalSsid", &[]);
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
