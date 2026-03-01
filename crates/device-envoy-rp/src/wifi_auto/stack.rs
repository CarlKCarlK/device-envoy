//! A device abstraction for WiFi connectivity used internally by [`crate::wifi_auto::WifiAuto`].
//!
//! See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.

#![allow(clippy::future_not_send, reason = "single-threaded")]
#![allow(
    unsafe_code,
    reason = "StackStorage uses UnsafeCell in single-threaded context"
)]

use core::cell::{RefCell, UnsafeCell};
use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::peripherals;
use embassy_rp::pio::Pio;
use embassy_rp::{
    Peri,
    dma::{AnyChannel, Channel},
    gpio::{Level, Output},
    peripherals::{PIN_23, PIN_24, PIN_25, PIN_29},
};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use portable_atomic::{AtomicBool, Ordering};
use static_cell::StaticCell;

use super::credentials::WifiCredentials;
use super::dhcp::dhcp_server_task;
use crate::flash_array::FlashBlock;

// ============================================================================
// Types
// ============================================================================

/// Events emitted by the WiFi device.
pub enum WifiEvent {
    /// Network stack is initialized in captive portal mode, ready for configuration
    CaptivePortalReady,
    /// Network stack is initialized in client mode and DHCP is configured
    ClientReady,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum WifiStartMode {
    CaptivePortal,
    Client,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct WifiStoredState {
    credentials: Option<WifiCredentials>,
    start_mode: WifiStartMode,
}

impl Default for WifiStoredState {
    fn default() -> Self {
        Self {
            credentials: None,
            start_mode: WifiStartMode::CaptivePortal,
        }
    }
}

/// Internal WiFi operating mode used during startup.
#[derive(Clone, PartialEq, Eq)]
#[doc(hidden)]
pub enum WifiMode {
    /// Start in captive portal mode for configuration (no credentials needed)
    CaptivePortal,
    /// Connect to WiFi network using provisioned credentials
    ClientConfigured(WifiCredentials),
}

/// Single-threaded once-storage for network stack
///
/// SAFETY: This is safe in single-threaded Embassy context
pub struct StackStorage {
    initialized: AtomicBool,
    ready: Signal<CriticalSectionRawMutex, ()>,
    value: UnsafeCell<Option<&'static Stack<'static>>>,
}

// SAFETY: We're in a single-threaded context (Embassy on Pico)
unsafe impl Sync for StackStorage {}

impl StackStorage {
    /// Create a new empty StackStorage
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            ready: Signal::new(),
            value: UnsafeCell::new(None),
        }
    }

    /// Initialize the stack storage (can only be called once)
    pub fn init(&self, stack: &'static Stack<'static>) {
        // SAFETY: This is called once from WiFi device loop
        unsafe {
            *self.value.get() = Some(stack);
        }
        self.initialized.store(true, Ordering::Release);
        self.ready.signal(());
    }

    /// Wait for the stack to be initialized and return it
    pub async fn get(&self) -> &'static Stack<'static> {
        if !self.initialized.load(Ordering::Acquire) {
            self.ready.wait().await;
        }
        // SAFETY: initialized is true, so value is set
        unsafe { (*self.value.get()).unwrap() }
    }
}

// ============================================================================
// WiFi Virtual Device
// ============================================================================

/// Signal type for WiFi events.
pub type WifiEvents = Signal<CriticalSectionRawMutex, WifiEvent>;

/// PIO peripheral used by the WiFi driver.
#[doc(hidden)]
pub trait WifiPio: crate::pio_irqs::PioIrqMap {
    fn spawn_wifi_task(
        spawner: Spawner,
        runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, Self, 0, AnyChannel>>,
    );

    fn spawn_device_loop(
        spawner: Spawner,
        pin_23: Peri<'static, PIN_23>,
        pin_24: Peri<'static, PIN_24>,
        pin_25: Peri<'static, PIN_25>,
        pin_29: Peri<'static, PIN_29>,
        pio: Peri<'static, Self>,
        dma: Peri<'static, AnyChannel>,
        mode: WifiMode,
        captive_portal_ssid: &'static str,
        wifi_events: &'static WifiEvents,
        stack_storage: &'static StackStorage,
    );
}

/// Resources needed by the WiFi device.
pub struct WifiStatic {
    events: WifiEvents,
    stack: StackStorage,
    wifi_cell: StaticCell<Wifi>,
}

/// A device abstraction that manages WiFi connectivity and network stack in both captive portal and client modes.
///
/// See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.
pub struct Wifi {
    events: &'static WifiEvents,
    stack: &'static StackStorage,
    credential_store: Mutex<CriticalSectionRawMutex, RefCell<FlashBlock>>,
}

impl Wifi {
    /// Create WiFi resources (events + storage).
    ///
    /// This must be called once to create a static `WifiStatic` that will be passed to
    /// [`Wifi::new_with_captive_portal_ssid`].
    ///
    /// See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.
    #[must_use]
    pub const fn new_static() -> WifiStatic {
        WifiStatic {
            events: Signal::new(),
            stack: StackStorage::new(),
            wifi_cell: StaticCell::new(),
        }
    }

    /// Wait for the network stack to be ready and return a reference to it.
    ///
    /// This provides access to the Embassy network stack for TCP/UDP operations.
    /// The stack will be configured differently depending on the WiFi mode:
    /// - In captive portal mode: static IP 192.168.4.1
    /// - In client mode: DHCP-assigned IP
    ///
    /// See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.
    pub async fn wait_for_stack(&self) -> &'static Stack<'static> {
        self.stack.get().await
    }

    /// Wait for and return the next WiFi event.
    ///
    /// This will block until the next [`WifiEvent`] occurs, such as:
    /// - [`WifiEvent::CaptivePortalReady`] when captive portal mode is initialized
    /// - [`WifiEvent::ClientReady`] when connected to WiFi and DHCP is configured
    ///
    /// See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.
    pub async fn wait_for_wifi_event(&self) -> WifiEvent {
        self.events.wait().await
    }

    pub fn new_with_captive_portal_ssid<PIO: WifiPio, DMA: Channel>(
        wifi_static: &'static WifiStatic,
        pin_23: Peri<'static, PIN_23>,
        pin_24: Peri<'static, PIN_24>,
        pin_25: Peri<'static, PIN_25>,
        pin_29: Peri<'static, PIN_29>,
        pio: Peri<'static, PIO>,
        dma: Peri<'static, DMA>,
        credential_store: FlashBlock,
        captive_portal_ssid: &'static str,
        spawner: Spawner,
    ) -> &'static Self {
        let mut store_block = credential_store;
        let stored_state = load_state_from_block(&mut store_block);
        let mode = match stored_state.start_mode {
            WifiStartMode::CaptivePortal => WifiMode::CaptivePortal,
            WifiStartMode::Client => {
                if let Some(creds) = stored_state.credentials.clone() {
                    WifiMode::ClientConfigured(creds)
                } else {
                    WifiMode::CaptivePortal
                }
            }
        };
        let dma = dma.into();
        PIO::spawn_device_loop(
            spawner,
            pin_23,
            pin_24,
            pin_25,
            pin_29,
            pio,
            dma,
            mode,
            captive_portal_ssid,
            &wifi_static.events,
            &wifi_static.stack,
        );
        wifi_static.wifi_cell.init(Self {
            events: &wifi_static.events,
            stack: &wifi_static.stack,
            credential_store: Mutex::new(RefCell::new(store_block)),
        })
    }

    fn update_state<F>(&self, f: F) -> Result<(), &'static str>
    where
        F: FnOnce(&mut WifiStoredState),
    {
        self.credential_store.lock(|cell| {
            let mut block = cell.borrow_mut();
            let mut state = load_state_from_block(&mut block);
            f(&mut state);
            save_state_to_block(&mut block, &state)
        })
    }

    fn read_state<R>(&self, f: impl FnOnce(&WifiStoredState) -> R) -> R {
        self.credential_store.lock(|cell| {
            let mut block = cell.borrow_mut();
            let state = load_state_from_block(&mut block);
            f(&state)
        })
    }

    /// Persist credentials into the configured flash store.
    pub fn persist_credentials(&self, credentials: &WifiCredentials) -> Result<(), &'static str> {
        let cloned = credentials.clone();
        self.update_state(|state| {
            state.credentials = Some(cloned.clone());
            state.start_mode = WifiStartMode::Client;
        })
    }

    /// Return whether credentials currently exist in flash.
    pub fn has_persisted_credentials(&self) -> bool {
        self.read_state(|state| state.credentials.is_some())
    }

    /// Load stored credentials if available.
    pub fn load_persisted_credentials(&self) -> Option<WifiCredentials> {
        self.read_state(|state| state.credentials.clone())
    }

    /// Return the currently configured start mode.
    pub fn current_start_mode(&self) -> WifiStartMode {
        self.read_state(|state| state.start_mode)
    }

    /// Change the stored start mode flag.
    pub fn set_start_mode(&self, mode: WifiStartMode) -> Result<(), &'static str> {
        self.update_state(|state| {
            state.start_mode = mode;
        })
    }

    /// Update the start mode flag in a raw flash block before WiFi initialization.
    pub fn prepare_start_mode(
        block: &mut FlashBlock,
        mode: WifiStartMode,
    ) -> Result<(), &'static str> {
        let mut state = load_state_from_block(block);
        state.start_mode = mode;
        save_state_to_block(block, &state)
    }

    /// Peek stored credentials directly from a flash block.
    pub fn peek_credentials(block: &mut FlashBlock) -> Option<WifiCredentials> {
        load_state_from_block(block).credentials
    }

    /// Peek the stored start mode directly from a flash block.
    pub fn peek_start_mode(block: &mut FlashBlock) -> WifiStartMode {
        load_state_from_block(block).start_mode
    }
}

fn load_state_from_block(block: &mut FlashBlock) -> WifiStoredState {
    match block.load::<WifiStoredState>() {
        Ok(Some(state)) => state,
        Ok(None) => WifiStoredState::default(),
        Err(_) => {
            warn!("Failed to load stored WiFi state");
            WifiStoredState::default()
        }
    }
}

fn save_state_to_block(
    block: &mut FlashBlock,
    state: &WifiStoredState,
) -> Result<(), &'static str> {
    block
        .save(state)
        .map_err(|_| "Failed to save WiFi state to flash")
}

async fn wifi_task_impl<PIO: WifiPio>(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO, 0, AnyChannel>>,
) -> ! {
    runner.run().await
}

async fn wifi_device_loop_impl<PIO: WifiPio>(
    pin_23: Peri<'static, PIN_23>,
    pin_24: Peri<'static, PIN_24>,
    pin_25: Peri<'static, PIN_25>,
    pin_29: Peri<'static, PIN_29>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, AnyChannel>,
    mode: WifiMode,
    captive_portal_ssid: &'static str,
    wifi_events: &'static WifiEvents,
    stack_storage: &'static StackStorage,
    spawner: Spawner,
) -> ! {
    match mode {
        WifiMode::CaptivePortal => {
            wifi_device_loop_captive_portal(
                pin_23,
                pin_24,
                pin_25,
                pin_29,
                pio,
                dma,
                captive_portal_ssid,
                wifi_events,
                stack_storage,
                spawner,
            )
            .await
        }
        WifiMode::ClientConfigured(credentials) => {
            wifi_device_loop_client_configured(
                pin_23,
                pin_24,
                pin_25,
                pin_29,
                pio,
                dma,
                wifi_events,
                stack_storage,
                spawner,
                credentials,
            )
            .await
        }
    }
}

/// WiFi device loop for captive portal mode
async fn wifi_device_loop_captive_portal<PIO: WifiPio>(
    pin_23: Peri<'static, PIN_23>,
    pin_24: Peri<'static, PIN_24>,
    pin_25: Peri<'static, PIN_25>,
    pin_29: Peri<'static, PIN_29>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, AnyChannel>,
    captive_portal_ssid: &'static str,
    wifi_events: &'static WifiEvents,
    stack_storage: &'static StackStorage,
    spawner: Spawner,
) -> ! {
    info!(
        "WiFi device initializing in captive portal mode ({})",
        captive_portal_ssid
    );

    // Initialize WiFi hardware
    let fw = cyw43_firmware::CYW43_43439A0;
    let clm = cyw43_firmware::CYW43_43439A0_CLM;

    let pwr = Output::new(pin_23, Level::Low);
    let cs = Output::new(pin_25, Level::High);
    let mut pio = Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        pin_24,
        pin_29,
        dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    PIO::spawn_wifi_task(spawner, runner);

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Start captive portal mode
    const CAPTIVE_PORTAL_PASSWORD: &str = ""; // Open network

    info!("Starting captive portal mode: {}", captive_portal_ssid);

    // Configure static IP for captive portal mode (we are the gateway)
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 4, 1), 24),
        gateway: Some(embassy_net::Ipv4Address::new(192, 168, 4, 1)),
        dns_servers: heapless::Vec::from_slice(&[embassy_net::Ipv4Address::new(192, 168, 4, 1)])
            .unwrap(),
    });

    let seed = 0x0bad_cafe_dead_beef;

    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    static STACK: StaticCell<Stack<'static>> = StaticCell::new();
    let (stack_val, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<5>::new()),
        seed,
    );
    let stack = STACK.init(stack_val);

    unwrap!(spawner.spawn(net_task(runner)));

    // Start captive portal network
    if CAPTIVE_PORTAL_PASSWORD.is_empty() {
        control.start_ap_open(captive_portal_ssid, 1).await;
    } else {
        control
            .start_ap_wpa2(captive_portal_ssid, CAPTIVE_PORTAL_PASSWORD, 1)
            .await;
    }

    info!("Captive portal mode started! SSID: {}", captive_portal_ssid);

    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("Captive portal IP Address: {}", config.address);
    }

    // Start DHCP server for captive portal mode
    let server_ip = embassy_net::Ipv4Address::new(192, 168, 4, 1);
    let netmask = embassy_net::Ipv4Address::new(255, 255, 255, 0);
    let pool_start = embassy_net::Ipv4Address::new(192, 168, 4, 2);
    let pool_size = 253; // 192.168.4.2 - 192.168.4.254

    unwrap!(spawner.spawn(dhcp_server_task(
        stack, server_ip, netmask, pool_start, pool_size,
    )));

    info!("DHCP server started (pool: 192.168.4.2-254)");
    info!(
        "WiFi captive portal ready - connect to '{}'",
        captive_portal_ssid
    );

    // Store stack reference and emit CaptivePortalReady event
    stack_storage.init(stack);
    wifi_events.signal(WifiEvent::CaptivePortalReady);

    // Keep task alive
    loop {
        Timer::after_secs(3600).await;
    }
}

/// WiFi device loop for client mode with provisioned credentials
async fn wifi_device_loop_client_configured<PIO: WifiPio>(
    pin_23: Peri<'static, PIN_23>,
    pin_24: Peri<'static, PIN_24>,
    pin_25: Peri<'static, PIN_25>,
    pin_29: Peri<'static, PIN_29>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, AnyChannel>,
    wifi_events: &'static WifiEvents,
    stack_storage: &'static StackStorage,
    spawner: Spawner,
    credentials: WifiCredentials,
) -> ! {
    let WifiCredentials { ssid, password } = credentials;

    wifi_device_loop_client_impl(
        pin_23,
        pin_24,
        pin_25,
        pin_29,
        pio,
        dma,
        wifi_events,
        stack_storage,
        spawner,
        ssid,
        password,
    )
    .await
}

/// Shared client-mode implementation.
async fn wifi_device_loop_client_impl<PIO: WifiPio>(
    pin_23: Peri<'static, PIN_23>,
    pin_24: Peri<'static, PIN_24>,
    pin_25: Peri<'static, PIN_25>,
    pin_29: Peri<'static, PIN_29>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, AnyChannel>,
    wifi_events: &'static WifiEvents,
    stack_storage: &'static StackStorage,
    spawner: Spawner,
    ssid: heapless::String<32>,
    password: heapless::String<64>,
) -> ! {
    info!("WiFi device initializing in client mode");

    let ssid_str = ssid;
    let password_str = password;

    // Initialize WiFi hardware
    let fw = cyw43_firmware::CYW43_43439A0;
    let clm = cyw43_firmware::CYW43_43439A0_CLM;

    let pwr = Output::new(pin_23, Level::Low);
    let cs = Output::new(pin_25, Level::High);
    let mut pio = Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        pin_24,
        pin_29,
        dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    PIO::spawn_wifi_task(spawner, runner);

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Initialize network stack
    let config = Config::dhcpv4(Default::default());
    let seed = 0x7c8f_3a2e_9d14_6b5a;

    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    static STACK: StaticCell<Stack<'static>> = StaticCell::new();
    let (stack_val, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<5>::new()),
        seed,
    );
    let stack = STACK.init(stack_val);

    unwrap!(spawner.spawn(net_task(runner)));

    // Connect to WiFi
    info!(
        "Connecting to WiFi: {} (password length: {})",
        ssid_str,
        password_str.len()
    );
    // TODO Pico2 WiFi driver has bugs - retrying after join failure causes IOCTL crashes
    // Just try once and let the higher-level timeout in wait_for_client_ready_with_timeout
    // handle retries by restarting the entire WiFi task
    match control
        .join(ssid_str.as_str(), JoinOptions::new(password_str.as_bytes()))
        .await
    {
        Ok(_) => {
            info!("WiFi join succeeded");
        }
        Err(err) => {
            info!("WiFi join failed: {:?}", defmt::Debug2Format(&err));
            info!("Not retrying - letting timeout handle this to avoid driver bugs");
            // Don't signal ClientReady - let the timeout in wait_for_client_ready_with_timeout
            // trigger and the higher-level retry logic will restart this entire task
            loop {
                Timer::after_secs(3600).await;
            }
        }
    }

    info!("WiFi connected! Waiting for DHCP...");
    let mut dhcp_wait_seconds = 0_u32;
    let config = loop {
        if let Some(config) = stack.config_v4() {
            break config;
        }
        if dhcp_wait_seconds % 5 == 0 {
            info!(
                "DHCP wait {}s (link_up={} config_up={})",
                dhcp_wait_seconds,
                stack.is_link_up(),
                stack.is_config_up()
            );
        }
        Timer::after_secs(1).await;
        dhcp_wait_seconds += 1;
    };
    info!("IP Address: {}", config.address);

    info!("WiFi client ready");

    // Store stack reference and emit ClientReady event
    stack_storage.init(stack);
    wifi_events.signal(WifiEvent::ClientReady);

    // Keep task alive (could monitor link status in future)
    loop {
        Timer::after_secs(3600).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

// ============================================================================
// WiFi Tasks
// ============================================================================

macro_rules! impl_wifi_pio {
    ($pio:ident, $suffix:ident) => {
        paste::paste! {
            impl WifiPio for peripherals::$pio {
                fn spawn_wifi_task(
                    spawner: Spawner,
                    runner: cyw43::Runner<
                        'static,
                        Output<'static>,
                        PioSpi<'static, Self, 0, AnyChannel>,
                    >,
                ) {
                    defmt::unwrap!(spawner.spawn([<wifi_task_ $suffix>](runner)));
                }

                fn spawn_device_loop(
                    spawner: Spawner,
                    pin_23: Peri<'static, PIN_23>,
                    pin_24: Peri<'static, PIN_24>,
                    pin_25: Peri<'static, PIN_25>,
                    pin_29: Peri<'static, PIN_29>,
                    pio: Peri<'static, Self>,
                    dma: Peri<'static, AnyChannel>,
                    mode: WifiMode,
                    captive_portal_ssid: &'static str,
                    wifi_events: &'static WifiEvents,
                    stack_storage: &'static StackStorage,
                ) {
                    defmt::unwrap!(spawner.spawn([<wifi_device_loop_ $suffix>](
                        pin_23,
                        pin_24,
                        pin_25,
                        pin_29,
                        pio,
                        dma,
                        mode,
                        captive_portal_ssid,
                        wifi_events,
                        stack_storage,
                        spawner,
                    )));
                }
            }

            #[embassy_executor::task]
            async fn [<wifi_device_loop_ $suffix>](
                pin_23: Peri<'static, PIN_23>,
                pin_24: Peri<'static, PIN_24>,
                pin_25: Peri<'static, PIN_25>,
                pin_29: Peri<'static, PIN_29>,
                pio: Peri<'static, peripherals::$pio>,
                dma: Peri<'static, AnyChannel>,
                mode: WifiMode,
                captive_portal_ssid: &'static str,
                wifi_events: &'static WifiEvents,
                stack_storage: &'static StackStorage,
                spawner: Spawner,
            ) -> ! {
                wifi_device_loop_impl::<peripherals::$pio>(
                    pin_23,
                    pin_24,
                    pin_25,
                    pin_29,
                    pio,
                    dma,
                    mode,
                    captive_portal_ssid,
                    wifi_events,
                    stack_storage,
                    spawner,
                )
                .await
            }

            #[embassy_executor::task]
            async fn [<wifi_task_ $suffix>](
                runner: cyw43::Runner<
                    'static,
                    Output<'static>,
                    PioSpi<'static, peripherals::$pio, 0, AnyChannel>,
                >,
            ) -> ! {
                wifi_task_impl::<peripherals::$pio>(runner).await
            }
        }
    };
}

impl_wifi_pio!(PIO0, pio0);
impl_wifi_pio!(PIO1, pio1);
#[cfg(feature = "pico2")]
impl_wifi_pio!(PIO2, pio2);
