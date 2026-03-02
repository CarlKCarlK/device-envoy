//! A device abstraction for automatic Wi-Fi credential collection workflows.
//! See [`WifiAuto`] for the main struct and usage.

#[cfg(target_os = "none")]
mod dhcp;
#[cfg(target_os = "none")]
mod dns;
pub mod fields;
mod portal;

use core::cell::RefCell;
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "none")]
use crate::flash_array::FlashBlock;
use crate::Result;
#[cfg(target_os = "none")]
use crate::button::Button;
#[cfg(target_os = "none")]
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};
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

pub use portal::{FormData, HtmlBuffer, WifiAutoField};

/// Events emitted by [`WifiAuto`] while driving a setup flow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiAutoEvent {
    /// Captive portal is ready and waiting for user input.
    CaptivePortalReady,
    /// Device is attempting to connect using saved credentials.
    Connecting {
        /// Current attempt number (0-based).
        try_index: u8,
        /// Total number of attempts planned.
        try_count: u8,
    },
    /// Connection failed after all attempts.
    ConnectionFailed,
}

/// Preferred Wi-Fi startup mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WifiStartMode {
    /// Start directly in Wi-Fi client mode using saved credentials.
    Client,
    /// Start in captive-portal mode for reconfiguration.
    CaptivePortal,
}

impl Default for WifiStartMode {
    fn default() -> Self {
        Self::Client
    }
}

/// Wi-Fi credentials collected from the captive portal.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WifiCredentials {
    /// Network name (SSID).
    pub ssid: heapless::String<32>,
    /// Network password.
    pub password: heapless::String<64>,
}

impl WifiCredentials {
    /// Create credentials from string slices.
    #[must_use]
    pub fn new(ssid: &str, password: &str) -> Self {
        assert!(!ssid.is_empty(), "ssid must not be empty");
        let mut ssid_string = heapless::String::<32>::new();
        ssid_string
            .push_str(ssid)
            .expect("ssid exceeds 32 characters");
        let mut password_string = heapless::String::<64>::new();
        password_string
            .push_str(password)
            .expect("password exceeds 64 characters");
        Self {
            ssid: ssid_string,
            password: password_string,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct WifiAutoPersistedState {
    wifi_credentials: Option<WifiCredentials>,
    wifi_start_mode: WifiStartMode,
}

impl Default for WifiAutoPersistedState {
    fn default() -> Self {
        Self {
            wifi_credentials: None,
            wifi_start_mode: WifiStartMode::Client,
        }
    }
}

enum WifiAutoStorage {
    #[cfg(target_os = "none")]
    Flash(RefCell<FlashBlock>),
    Memory(RefCell<WifiAutoPersistedState>),
}

/// Captive-portal workflow configuration and parsing helpers.
///
/// This is the ESP32 port of the Pico `wifi_auto` setup surface for parsing
/// form submissions and rendering setup pages. Networking/backend connection
/// orchestration is intentionally separate from this type.
pub struct WifiAuto<'a> {
    captive_portal_ssid: &'static str,
    fields: &'a [&'a dyn WifiAutoField],
    storage: WifiAutoStorage,
}

impl<'a> WifiAuto<'a> {
    /// Create a new setup flow configuration.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    #[must_use]
    pub const fn new(
        captive_portal_ssid: &'static str,
        custom_fields: &'a [&'a dyn WifiAutoField],
    ) -> Self {
        Self {
            captive_portal_ssid,
            fields: custom_fields,
            storage: WifiAutoStorage::Memory(RefCell::new(WifiAutoPersistedState {
                wifi_credentials: None,
                wifi_start_mode: WifiStartMode::Client,
            })),
        }
    }

    /// Create a setup flow backed by persistent flash storage.
    ///
    /// This constructor is available on embedded targets (`target_os = "none"`).
    #[cfg(target_os = "none")]
    #[must_use]
    pub fn new_with_flash(
        captive_portal_ssid: &'static str,
        wifi_auto_flash_block: FlashBlock,
        custom_fields: &'a [&'a dyn WifiAutoField],
    ) -> Self {
        Self {
            captive_portal_ssid,
            fields: custom_fields,
            storage: WifiAutoStorage::Flash(RefCell::new(wifi_auto_flash_block)),
        }
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
        portal::generate_config_page(defaults, self.fields)
    }

    /// Parse a raw HTTP POST request into credentials and apply field parsers.
    ///
    /// Returns `None` when the request body is malformed, the SSID is missing,
    /// or any custom field parser reports an error.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    #[must_use]
    pub fn parse_post(&self, request: &str) -> Option<WifiCredentials> {
        portal::parse_post(request, self.fields)
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
    pub fn force_captive_portal_if_pressed(&self, button: &Button<'_>) -> Result<bool> {
        self.force_captive_portal_if_pressed_state(button.is_pressed())
    }

    /// Check whether every custom field reports a satisfied state.
    ///
    /// See the [WifiAuto struct example](Self) for usage.
    pub fn custom_fields_satisfied(&self) -> Result<bool> {
        for field in self.fields {
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
    /// Connect using persisted credentials or captive-portal setup flow.
    ///
    /// If `force_captive_portal` is `true`, startup enters AP setup mode even
    /// when saved credentials exist.
    pub async fn connect<OnEvent, OnEventFuture>(
        &self,
        wifi: esp_hal::peripherals::WIFI<'static>,
        spawner: embassy_executor::Spawner,
        force_captive_portal: bool,
        mut on_event: OnEvent,
    ) -> Result<Stack<'static>>
    where
        OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
        OnEventFuture: core::future::Future<Output = Result<()>>,
    {
        Self::initialize_wifi_heap_once();

        static ESP_RADIO_CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
        let esp_radio_controller = ESP_RADIO_CONTROLLER.init(esp_radio::init()?);
        let (mut wifi_controller, interfaces) = esp_radio::wifi::new(
            esp_radio_controller,
            wifi,
            esp_radio::wifi::Config::default(),
        )?;

        let mut wifi_start_mode = self.start_mode()?;
        if force_captive_portal {
            wifi_start_mode = WifiStartMode::CaptivePortal;
        }
        let custom_fields_satisfied = self.custom_fields_satisfied()?;
        if !custom_fields_satisfied {
            wifi_start_mode = WifiStartMode::CaptivePortal;
        }

        let mut wifi_credentials = self.load_persisted_credentials()?;
        let has_persisted_credentials = wifi_credentials.is_some();
        if !has_persisted_credentials {
            wifi_start_mode = WifiStartMode::CaptivePortal;
        }
        info!(
            "wifi_auto start mode={:?} force_captive_portal={} custom_fields_satisfied={} has_persisted_credentials={}",
            wifi_start_mode,
            force_captive_portal,
            custom_fields_satisfied,
            has_persisted_credentials
        );

        if matches!(wifi_start_mode, WifiStartMode::CaptivePortal) {
            on_event(WifiAutoEvent::CaptivePortalReady).await?;
            let portal_wifi_credentials = self
                .run_captive_portal(&mut wifi_controller, interfaces.ap)
                .await?;
            self.persist_credentials(&portal_wifi_credentials)?;
            self.set_start_mode(WifiStartMode::Client)?;
            wifi_credentials = Some(portal_wifi_credentials);
        }

        let wifi_credentials =
            wifi_credentials.expect("wifi credentials should exist after captive portal fallback");
        let stack = self
            .connect_client_with_retries(
                &mut wifi_controller,
                interfaces.sta,
                &wifi_credentials,
                spawner,
                &mut on_event,
            )
            .await?;
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
    async fn connect_client_with_retries<OnEvent, OnEventFuture>(
        &self,
        wifi_controller: &mut esp_radio::wifi::WifiController<'static>,
        sta_device: esp_radio::wifi::WifiDevice<'static>,
        wifi_credentials: &WifiCredentials,
        spawner: embassy_executor::Spawner,
        on_event: &mut OnEvent,
    ) -> Result<Stack<'static>>
    where
        OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
        OnEventFuture: core::future::Future<Output = Result<()>>,
    {
        let wifi_client_config = esp_radio::wifi::ClientConfig::default()
            .with_ssid(String::from(wifi_credentials.ssid.as_str()))
            .with_password(String::from(wifi_credentials.password.as_str()));
        wifi_controller.set_config(&esp_radio::wifi::ModeConfig::Client(wifi_client_config))?;
        info!(
            "wifi_auto client config set: ssid='{}' password_len={}",
            wifi_credentials.ssid.as_str(),
            wifi_credentials.password.len()
        );
        wifi_controller.start_async().await?;

        const TRY_COUNT: u8 = 10;
        for try_index in 0..TRY_COUNT {
            info!(
                "wifi_auto connect attempt {}/{} for ssid='{}'",
                try_index + 1,
                TRY_COUNT,
                wifi_credentials.ssid.as_str()
            );
            on_event(WifiAutoEvent::Connecting {
                try_index,
                try_count: TRY_COUNT,
            })
            .await?;

            match wifi_controller.connect_async().await {
                Ok(()) => {
                    info!("wifi_auto client connected on try {}", try_index + 1);
                    static STA_STACK_RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
                    let (stack, runner) = embassy_net::new(
                        sta_device,
                        Config::dhcpv4(Default::default()),
                        STA_STACK_RESOURCES.init(StackResources::new()),
                        0xD1D1_C1C1_5151_4242,
                    );
                    spawner.spawn(wifi_auto_net_task(runner))?;
                    return Ok(stack);
                }
                Err(error) => {
                    warn!(
                        "wifi_auto connect failed on try {} for ssid='{}': {:?}",
                        try_index + 1,
                        wifi_credentials.ssid.as_str(),
                        error
                    );
                    warn!(
                        "wifi_auto hint: verify AP is 2.4GHz and security mode is compatible (for example WPA2/WPA3 mixed, not WPA3-only)"
                    );
                    Timer::after(Duration::from_millis(800)).await;
                }
            }
        }

        on_event(WifiAutoEvent::ConnectionFailed).await?;
        warn!(
            "wifi_auto failed to connect after {} attempts; switching startup mode to CaptivePortal and resetting",
            TRY_COUNT
        );
        self.set_start_mode(WifiStartMode::CaptivePortal)?;
        info!("wifi_auto wrote startup mode CaptivePortal to storage");
        let _ = wifi_controller.stop_async().await;
        info!("wifi_auto resetting in 1 second");
        Timer::after(Duration::from_secs(1)).await;
        esp_hal::system::software_reset()
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
