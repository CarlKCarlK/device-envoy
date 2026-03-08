//! A device abstraction for common Wi-Fi auto-provisioning types and portal helpers.
//!
//! See [`WifiCredentials`] for the primary shared data type.

use core::future::Future;
use crate::button::Button;

pub mod fields;
pub mod portal;

pub use portal::{FormData, HtmlBuffer, WifiAutoField, generate_config_page, parse_post};

/// Canonical network stack type returned by [`WifiAuto::connect`].
pub type WifiStack = &'static embassy_net::Stack<'static>;

// This helper macro must be `pub` because downstream crates expand it in impl blocks.
#[doc(hidden)]
#[macro_export]
macro_rules! __impl_wifi_auto_connect {
    (
        $(#[$meta:meta])*
        fn $name:ident (&self as $self_ident:ident, $on_event:ident) -> $return_ty:ty $body:block
    ) => {
        $(#[$meta])*
        pub async fn $name<OnEvent, OnEventFuture>(
            &self,
            mut $on_event: OnEvent,
        ) -> $return_ty
        where
            OnEvent: FnMut($crate::wifi_auto::WifiAutoEvent) -> OnEventFuture,
            OnEventFuture: core::future::Future<Output = crate::Result<()>>,
        {
            let $self_ident = self;
            $body
        }
    };
    (
        $(#[$meta:meta])*
        fn $name:ident (self as $self_ident:ident, $on_event:ident) -> $return_ty:ty $body:block
    ) => {
        $(#[$meta])*
        pub async fn $name<OnEvent, OnEventFuture>(
            self,
            mut $on_event: OnEvent,
        ) -> $return_ty
        where
            OnEvent: FnMut($crate::wifi_auto::WifiAutoEvent) -> OnEventFuture,
            OnEventFuture: core::future::Future<Output = crate::Result<()>>,
        {
            let $self_ident = self;
            $body
        }
    };
}

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

/// Shared Wi-Fi auto-provisioning error variants used across platform ports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiAutoError {
    /// Captive-portal data or rendering format was invalid.
    FormatError,
    /// Stored Wi-Fi auto state is invalid for expected runtime flow.
    StorageCorrupted,
    /// A required custom field is missing from the Wi-Fi auto setup.
    MissingCustomWifiAutoField,
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

/// Platform-agnostic Wi-Fi auto-connect contract.
///
/// Platform crates implement this trait for their concrete runtime handles.
/// Constructors remain inherent on platform types.
///
/// `WifiAuto::connect` handles Wi-Fi setup end-to-end. It usually tries saved
/// credentials first, and if credentials are missing or invalid it can run a
/// captive portal so the user can submit new credentials.
///
/// While `connect` runs, `on_event` receives progress updates:
///
/// - [`WifiAutoEvent::CaptivePortalReady`]: captive portal is ready for user input.
/// - [`WifiAutoEvent::Connecting`]: a Wi-Fi connection attempt is in progress.
/// - [`WifiAutoEvent::ConnectionFailed`]: all connection attempts failed.
///
/// # Example
///
/// ```rust,no_run
/// use core::future::Future;
/// use core::convert::Infallible;
/// use device_envoy_core::{
///     button::Button,
///     wifi_auto::{WifiAuto, WifiAutoEvent, WifiStack},
/// };
///
/// async fn connect_with_status(
///     wifi_auto: impl WifiAuto<Error = Infallible>,
/// ) -> Result<(WifiStack, impl Button), Infallible> {
///     wifi_auto
///         .connect(|wifi_auto_event| async move {
///             match wifi_auto_event {
///                 WifiAutoEvent::CaptivePortalReady => {
///                     // Captive portal is ready for Wi-Fi credential entry.
///                 }
///                 WifiAutoEvent::Connecting { .. } => {
///                     // A Wi-Fi connection attempt is in progress.
///                 }
///                 WifiAutoEvent::ConnectionFailed => {
///                     // All connection attempts failed.
///                 }
///             }
///             Ok(())
///         })
///         .await
/// }
///
/// # struct DemoButton;
/// # impl Button for DemoButton {
/// #     fn is_pressed(&self) -> bool {
/// #         false
/// #     }
/// #     async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
/// # }
/// # struct DemoWifiAuto;
/// # impl WifiAuto for DemoWifiAuto {
/// #     type Error = Infallible;
/// #     async fn connect<
/// #         OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
/// #         OnEventFuture: Future<Output = Result<(), Self::Error>>,
/// #     >(
/// #         self,
/// #         mut on_event: OnEvent,
/// #     ) -> Result<(WifiStack, DemoButton), Self::Error>
/// #     {
/// #         on_event(WifiAutoEvent::Connecting {
/// #             try_index: 0,
/// #             try_count: 1,
/// #         })
/// #         .await?;
/// #         let _todo_result: Result<(WifiStack, DemoButton), Self::Error> = todo!();
/// #         _todo_result
/// #     }
/// # }
/// # fn main() {
/// #     let wifi_auto = DemoWifiAuto;
/// #     let _future = connect_with_status(wifi_auto);
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait WifiAuto {
    /// Platform-specific error type.
    type Error;

    /// Connect to Wi-Fi, emitting progress events to `on_event`.
    ///
    /// See the [WifiAuto trait documentation](Self) for usage examples.
    async fn connect<OnEvent, OnEventFuture>(
        self,
        on_event: OnEvent,
    ) -> Result<(WifiStack, impl Button), Self::Error>
    where
        OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
        OnEventFuture: Future<Output = Result<(), Self::Error>>;
}

/// Backend contract for platform-specific Wi-Fi auto-connect operations.
pub trait WifiAutoBackend {
    /// Platform-specific error type.
    type Error;

    /// Whether boot should force captive portal regardless of persisted state.
    fn force_captive_portal(&self) -> bool;

    /// Number of connection attempts before emitting `ConnectionFailed`.
    fn try_count(&self) -> u8;

    /// Load persisted startup mode.
    fn load_start_mode(&self) -> Result<WifiStartMode, Self::Error>;

    /// Check whether all custom fields are currently satisfied.
    fn custom_fields_satisfied(&self) -> Result<bool, Self::Error>;

    /// Load persisted credentials, if present.
    fn load_persisted_credentials(&self) -> Result<Option<WifiCredentials>, Self::Error>;

    /// Persist submitted credentials.
    fn persist_credentials(&self, wifi_credentials: &WifiCredentials) -> Result<(), Self::Error>;

    /// Persist startup mode.
    fn set_start_mode(&self, wifi_start_mode: WifiStartMode) -> Result<(), Self::Error>;

    /// Run captive portal and return submitted credentials.
    fn run_captive_portal(
        &mut self,
    ) -> impl Future<Output = Result<WifiCredentials, Self::Error>> + '_;

    /// Configure platform networking once credentials are resolved.
    fn on_resolved_credentials(
        &mut self,
        wifi_credentials: &WifiCredentials,
    ) -> impl Future<Output = Result<(), Self::Error>> + '_;

    /// Run one platform-specific connect attempt.
    fn on_connect_attempt(
        &mut self,
        try_index: u8,
    ) -> impl Future<Output = Result<bool, Self::Error>> + '_;
}

/// Run the shared Wi-Fi auto-connect flow through a platform backend.
pub async fn connect_with_backend<Backend, OnEvent, OnEventFuture>(
    backend: &mut Backend,
    on_event: &mut OnEvent,
) -> Result<bool, Backend::Error>
where
    Backend: WifiAutoBackend,
    OnEvent: FnMut(WifiAutoEvent) -> OnEventFuture,
    OnEventFuture: Future<Output = Result<(), Backend::Error>>,
{
    let wifi_start_mode = backend.load_start_mode()?;
    let custom_fields_satisfied = backend.custom_fields_satisfied()?;
    let mut wifi_credentials = backend.load_persisted_credentials()?;
    let has_persisted_credentials = wifi_credentials.is_some();

    if should_enter_captive_portal(
        wifi_start_mode,
        backend.force_captive_portal(),
        has_persisted_credentials,
        custom_fields_satisfied,
    ) {
        on_event(WifiAutoEvent::CaptivePortalReady).await?;
        let portal_wifi_credentials = backend.run_captive_portal().await?;
        backend.persist_credentials(&portal_wifi_credentials)?;
        backend.set_start_mode(WifiStartMode::Client)?;
        wifi_credentials = Some(portal_wifi_credentials);
    }

    let wifi_credentials =
        wifi_credentials.expect("wifi credentials should exist after captive portal fallback");
    backend.on_resolved_credentials(&wifi_credentials).await?;

    for try_index in 0..backend.try_count() {
        on_event(WifiAutoEvent::Connecting {
            try_index,
            try_count: backend.try_count(),
        })
        .await?;
        if backend.on_connect_attempt(try_index).await? {
            return Ok(true);
        }
    }

    on_event(WifiAutoEvent::ConnectionFailed).await?;
    Ok(false)
}
