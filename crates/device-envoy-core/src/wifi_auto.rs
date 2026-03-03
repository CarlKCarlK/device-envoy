//! A device abstraction for common Wi-Fi auto-provisioning types and portal helpers.
//!
//! See [`WifiCredentials`] for the primary shared data type.

use core::future::Future;

pub mod portal;

pub use portal::{FormData, HtmlBuffer, WifiAutoField, generate_config_page, parse_post};

/// Events emitted while driving a Wi-Fi setup flow.
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

/// Return whether startup should enter captive-portal mode.
#[must_use]
pub const fn should_enter_captive_portal(
    wifi_start_mode: WifiStartMode,
    force_captive_portal: bool,
    has_persisted_credentials: bool,
    custom_fields_satisfied: bool,
) -> bool {
    force_captive_portal
        || !custom_fields_satisfied
        || !has_persisted_credentials
        || matches!(wifi_start_mode, WifiStartMode::CaptivePortal)
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

/// Persisted Wi-Fi auto state shared across platform ports.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WifiAutoPersistedState {
    /// Persisted credentials, if available.
    pub wifi_credentials: Option<WifiCredentials>,
    /// Preferred startup mode for next boot.
    pub wifi_start_mode: WifiStartMode,
}

impl Default for WifiAutoPersistedState {
    fn default() -> Self {
        Self {
            wifi_credentials: None,
            wifi_start_mode: WifiStartMode::Client,
        }
    }
}

/// Resolve Wi-Fi credentials for connection, optionally running captive-portal setup.
///
/// This function centralizes the common startup flow used by platform ports:
///
/// 1. Inspect persisted mode/credentials and custom-field readiness.
/// 2. Enter captive portal when required.
/// 3. Persist submitted credentials and switch startup mode back to client mode.
///
/// The returned credentials are guaranteed to be present unless callback logic violates
/// the expected contract (for example, `run_captive_portal` returns but does not provide
/// credentials).
pub async fn resolve_wifi_credentials<
    Error,
    OnEvent,
    OnEventFuture,
    LoadStartMode,
    CustomFieldsSatisfied,
    LoadPersistedCredentials,
    PersistCredentials,
    SetStartMode,
    RunCaptivePortalFuture,
>(
    force_captive_portal: bool,
    on_event: &mut OnEvent,
    mut load_start_mode: LoadStartMode,
    mut custom_fields_satisfied: CustomFieldsSatisfied,
    mut load_persisted_credentials: LoadPersistedCredentials,
    mut persist_credentials: PersistCredentials,
    mut set_start_mode: SetStartMode,
    run_captive_portal: RunCaptivePortalFuture,
) -> Result<WifiCredentials, Error>
where
    OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
    OnEventFuture: Future<Output = Result<(), Error>>,
    LoadStartMode: FnMut() -> Result<WifiStartMode, Error>,
    CustomFieldsSatisfied: FnMut() -> Result<bool, Error>,
    LoadPersistedCredentials: FnMut() -> Result<Option<WifiCredentials>, Error>,
    PersistCredentials: FnMut(&WifiCredentials) -> Result<(), Error>,
    SetStartMode: FnMut(WifiStartMode) -> Result<(), Error>,
    RunCaptivePortalFuture: Future<Output = Result<WifiCredentials, Error>>,
{
    let wifi_start_mode = load_start_mode()?;
    let custom_fields_satisfied = custom_fields_satisfied()?;
    let mut wifi_credentials = load_persisted_credentials()?;
    let has_persisted_credentials = wifi_credentials.is_some();

    let enter_captive_portal = should_enter_captive_portal(
        wifi_start_mode,
        force_captive_portal,
        has_persisted_credentials,
        custom_fields_satisfied,
    );
    if enter_captive_portal {
        on_event(WifiAutoEvent::CaptivePortalReady).await?;
        let portal_wifi_credentials = run_captive_portal.await?;
        persist_credentials(&portal_wifi_credentials)?;
        set_start_mode(WifiStartMode::Client)?;
        wifi_credentials = Some(portal_wifi_credentials);
    }

    let wifi_credentials =
        wifi_credentials.expect("wifi credentials should exist after captive portal fallback");
    Ok(wifi_credentials)
}

/// Hook trait for one connection attempt in [`run_connect_retries`].
pub trait ConnectAttemptHook<Error> {
    /// Run a single attempt identified by a zero-based `try_index`.
    ///
    /// Return `Ok(true)` when a connection is established.
    fn on_attempt(&mut self, try_index: u8) -> impl Future<Output = Result<bool, Error>> + '_;
}

/// Run a shared connect-retry loop and emit standard Wi-Fi auto events.
///
/// Returns `Ok(true)` when connected and `Ok(false)` after exhausting all retries
/// (after emitting `ConnectionFailed`).
pub async fn run_connect_retries<Error, OnEvent, OnEventFuture, Hook>(
    try_count: u8,
    on_event: &mut OnEvent,
    hook: &mut Hook,
) -> Result<bool, Error>
where
    OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
    OnEventFuture: Future<Output = Result<(), Error>>,
    Hook: ConnectAttemptHook<Error>,
{
    for try_index in 0..try_count {
        on_event(WifiAutoEvent::Connecting {
            try_index,
            try_count,
        })
        .await?;
        if hook.on_attempt(try_index).await? {
            return Ok(true);
        }
    }

    on_event(WifiAutoEvent::ConnectionFailed).await?;
    Ok(false)
}

/// Hook trait for post-setup connect flow orchestration.
pub trait ConnectFlowHook<Error>: ConnectAttemptHook<Error> {
    /// Run captive portal and return submitted credentials.
    fn run_captive_portal(
        &mut self,
    ) -> impl Future<Output = Result<WifiCredentials, Error>> + '_;

    /// Configure platform networking after credentials are resolved and before retries.
    fn on_resolved_credentials(
        &mut self,
        wifi_credentials: &WifiCredentials,
    ) -> impl Future<Output = Result<(), Error>> + '_;
}

/// Shared connect orchestration: setup decision + credential resolution + retry loop.
///
/// Returns `Ok(true)` on successful connection and `Ok(false)` after exhausted retries.
pub async fn connect_with_auto_setup<
    Error,
    OnEvent,
    OnEventFuture,
    LoadStartMode,
    CustomFieldsSatisfied,
    LoadPersistedCredentials,
    PersistCredentials,
    SetStartMode,
    Hook,
>(
    force_captive_portal: bool,
    on_event: &mut OnEvent,
    try_count: u8,
    mut load_start_mode: LoadStartMode,
    mut custom_fields_satisfied: CustomFieldsSatisfied,
    mut load_persisted_credentials: LoadPersistedCredentials,
    mut persist_credentials: PersistCredentials,
    mut set_start_mode: SetStartMode,
    hook: &mut Hook,
) -> Result<bool, Error>
where
    OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
    OnEventFuture: Future<Output = Result<(), Error>>,
    LoadStartMode: FnMut() -> Result<WifiStartMode, Error>,
    CustomFieldsSatisfied: FnMut() -> Result<bool, Error>,
    LoadPersistedCredentials: FnMut() -> Result<Option<WifiCredentials>, Error>,
    PersistCredentials: FnMut(&WifiCredentials) -> Result<(), Error>,
    SetStartMode: FnMut(WifiStartMode) -> Result<(), Error>,
    Hook: ConnectFlowHook<Error>,
{
    let wifi_start_mode = load_start_mode()?;
    let custom_fields_satisfied = custom_fields_satisfied()?;
    let mut wifi_credentials = load_persisted_credentials()?;
    let has_persisted_credentials = wifi_credentials.is_some();
    if should_enter_captive_portal(
        wifi_start_mode,
        force_captive_portal,
        has_persisted_credentials,
        custom_fields_satisfied,
    ) {
        on_event(WifiAutoEvent::CaptivePortalReady).await?;
        let portal_wifi_credentials = hook.run_captive_portal().await?;
        persist_credentials(&portal_wifi_credentials)?;
        set_start_mode(WifiStartMode::Client)?;
        wifi_credentials = Some(portal_wifi_credentials);
    }
    let wifi_credentials =
        wifi_credentials.expect("wifi credentials should exist after captive portal fallback");
    hook.on_resolved_credentials(&wifi_credentials).await?;
    run_connect_retries(try_count, on_event, hook).await
}
