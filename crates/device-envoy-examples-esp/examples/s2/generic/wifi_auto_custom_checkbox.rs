//! This example shows how to define a general checkbox field that can prompt the user for a boolean choice.
//!
//! Wiring:
//! - GPIO0 <-> button <-> GND
//! - Use internal pull-up (`PressedTo::Ground`).

#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;
use core::{convert::Infallible, future::pending};

use esp_backtrace as _;

use device_envoy_core::wifi_auto::{FormData, HtmlBuffer};
use device_envoy_esp::{
    Error, Result,
    button::{ButtonEsp, PressedTo},
    flash_block::{FlashBlock as _, FlashBlockEsp},
    init_and_start,
    wifi_auto::{WifiAuto as _, WifiAutoEsp, WifiAutoEvent, WifiAutoField},
};

esp_bootloader_esp_idf::esp_app_desc!();

struct CheckboxField {
    // Interior mutability keeps WifiAutoField methods ergonomic (`&self`) while
    // still allowing flash-backed load/save operations.
    checkbox_flash_block: RefCell<FlashBlockEsp>,
    // HTML form key used in generated input markup and POST parsing.
    field_name: &'static str,
    // User-visible checkbox label shown in the captive portal.
    label: &'static str,
    // Render-time fallback when no value has ever been stored.
    // This affects only initial UI state, not setup completion.
    default_checked: bool,
}

impl CheckboxField {
    fn new(
        checkbox_flash_block: FlashBlockEsp,
        field_name: &'static str,
        label: &'static str,
        default_checked: bool,
    ) -> Self {
        Self {
            checkbox_flash_block: RefCell::new(checkbox_flash_block),
            field_name,
            label,
            default_checked,
        }
    }

    // Tri-state load: None = never configured, Some(true/false) = user made a choice.
    fn checked(&self) -> Result<Option<bool>> {
        self.checkbox_flash_block.borrow_mut().load::<bool>()
    }

    // UI fallback only; this does not mean configuration is complete.
    fn checked_or_default(&self) -> Result<bool> {
        Ok(self.checked()?.unwrap_or(self.default_checked))
    }

    fn format_error() -> Error {
        Error::from(device_envoy_core::Error::WifiAutoFormat)
    }
}

impl WifiAutoField for CheckboxField {
    type Error = Error;

    fn render(&self, page: &mut HtmlBuffer) -> core::result::Result<(), Self::Error> {
        // render() controls what appears on the captive portal form.
        let checked_attribute = if self.checked_or_default()? {
            " checked"
        } else {
            ""
        };
        write!(
            page,
            "<p><label><input type=\"checkbox\" name=\"{}\" value=\"1\"{}> {}</label></p>",
            self.field_name, checked_attribute, self.label
        )
        .map_err(|_| Self::format_error())?;
        Ok(())
    }

    fn parse(&self, form: &FormData<'_>) -> core::result::Result<(), Self::Error> {
        // parse() reads submitted form data and persists this field value.
        let checked = matches!(
            form.get(self.field_name),
            Some("1") | Some("on") | Some("true")
        );
        self.checkbox_flash_block.borrow_mut().save(&checked)
    }

    fn is_satisfied(&self) -> core::result::Result<bool, Self::Error> {
        // Setup is incomplete until the user makes an explicit choice once.
        Ok(self.checked()?.is_some())
    }
}

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let [wifi_auto_flash_block, checkbox_flash_block] = FlashBlockEsp::new_array::<2>(p.FLASH)?;
    let checkbox_field = CheckboxField::new(
        checkbox_flash_block,
        "share_telemetry",
        "Share anonymous telemetry",
        false,
    );
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);

    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        "DeviceEnvoySetup",
        [&checkbox_field],
        spawner,
    )?;

    let _stack = wifi_auto
        .connect(
            &mut button,
            async |wifi_auto_event| -> Result<(), device_envoy_esp::Error> {
                match wifi_auto_event {
                    WifiAutoEvent::CaptivePortalReady => log::info!("Captive portal ready"),
                    WifiAutoEvent::Connecting { .. } => log::info!("Connecting"),
                    WifiAutoEvent::ConnectionFailed => log::warn!("Connection failed"),
                }
                Ok(())
            },
        )
        .await?;

    log::info!("share_telemetry: {:?}", checkbox_field.checked()?);

    pending().await
}
