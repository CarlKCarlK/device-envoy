//! A shared CYD DNS tester game loop and UI.
//!
//! [`dns_tester`] is the platform-neutral game loop: ESP, RP, WASM, and
//! [`CydMemory`](device_envoy_core::memory::CydMemory)
//! provide resources and events while it owns state transitions, commands,
//! rendering, and the redraw schedule. All platforms consume the same
//! TGA-backed bitmap and dynamic-value layout.

use core::fmt::Write;

use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoFont, MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::{Rgb565, Rgb888},
    primitives::Rectangle,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use profont::PROFONT_24_POINT;

use device_envoy_core::{
    UnwrapInfallible,
    button::Button,
    cyd::{
        Cyd, CydDisplay, CydTouch,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
        touch::TouchEvent,
    },
    dns_lookup::DnsLookup,
};
use embassy_futures::yield_now;

const LANDSCAPE_BITMAP_FIXED: Image565Fixed<320, 240, { 320 * 240 }> =
    tga!(concat!(env!("OUT_DIR"), "/dns_landscape.tga")).to_565();
const PORTRAIT_BITMAP_FIXED: Image565Fixed<240, 320, { 240 * 320 }> =
    tga!(concat!(env!("OUT_DIR"), "/dns_portrait.tga")).to_565();
const LANDSCAPE_BITMAP: Image565View = LANDSCAPE_BITMAP_FIXED.view();
const PORTRAIT_BITMAP: Image565View = PORTRAIT_BITMAP_FIXED.view();

const VALUE_TEXT: Rgb888 = Rgb888::new(255, 255, 255); // white
const SUCCESS_TEXT: Rgb888 = Rgb888::new(121, 226, 164); // soft green
const FAILURE_TEXT: Rgb888 = Rgb888::new(255, 117, 110); // coral red
const PANEL_FILL: Rgb888 = Rgb888::new(15, 38, 55); // dark desaturated blue

/// Run the shared DNS Tester loop on calibrated platform resources.
///
/// Platform setup follows the Linkage Blaze pattern: construct and calibrate
/// the display and touch resources, show the splash, connect Wi-Fi, then call
/// this function. The loop owns input policy, application state, DNS result
/// accounting, and rendering. Call [`dns_tester_splash`] during platform
/// setup before entering this loop. The loop returns platform control
/// requests instead of knowing how persistence or reboot works.
pub async fn dns_tester<CydDevice, ButtonDevice, DnsLookupDevice>(
    cyd: &mut CydDevice,
    button: &mut ButtonDevice,
    target: &'static str,
    dns_lookup: &mut DnsLookupDevice,
) -> Result<Exit, Error<CydDevice::Error, DnsLookupDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    DnsLookupDevice: DnsLookup,
{
    let mut orientation = cyd.orientation();
    let (display, touch) = cyd.parts();
    let bitmap = match orientation {
        Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_BITMAP,
        Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_BITMAP,
    };
    display
        .fill_contiguous_full(bitmap.rgb565_iter())
        .map_err(Error::display)?;
    let mut queries: u32 = 0;
    let mut successes: u32 = 0;
    let mut failures: u32 = 0;
    let mut last_latency_millis = 0;
    loop {
        // todo0000 review these
        let status = if queries == 0 {
            "TAP"
        } else if failures > 0 {
            "FAIL"
        } else {
            "OK"
        };
        let mut latency = heapless::String::<16>::new();
        if queries == 0 {
            latency
                .push_str("--")
                .map_err(|_| Error::Display(UiError::Text(core::fmt::Error)))?;
        } else {
            write!(latency, "{} ms", last_latency_millis)?;
        }
        let mut query_text = heapless::String::<12>::new();
        let mut success_text = heapless::String::<12>::new();
        let mut failure_text = heapless::String::<12>::new();
        write!(query_text, "{}", queries)?;
        write!(success_text, "{}", successes)?;
        write!(failure_text, "{}", failures)?;

        match orientation {
            Orientation::Landscape | Orientation::LandscapeInverted => {
                // Draw the DNS hostname being tested.
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(22, 76), Size::new(150, 20)),
                    target,
                    &FONT_10X20,
                    Alignment::Left,
                    VALUE_TEXT,
                )
                .await?;
                // Draw the current test status.
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(244, 82), Size::new(56, 20)),
                    status,
                    &FONT_10X20,
                    Alignment::Center,
                    if failures > 0 {
                        FAILURE_TEXT
                    } else {
                        SUCCESS_TEXT
                    },
                )
                .await?;
                // Draw the most recent lookup latency.
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(100, 100), Size::new(120, 29)),
                    latency.as_str(),
                    &PROFONT_24_POINT,
                    Alignment::Center,
                    VALUE_TEXT,
                )
                .await?;
                // Draw the total number of DNS queries.
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(27, 156), Size::new(50, 20)),
                    query_text.as_str(),
                    &FONT_10X20,
                    Alignment::Center,
                    VALUE_TEXT,
                )
                .await?;
                // Draw the number of successful DNS queries.
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(135, 156), Size::new(50, 20)),
                    success_text.as_str(),
                    &FONT_10X20,
                    Alignment::Center,
                    SUCCESS_TEXT,
                )
                .await?;
                // Draw the number of failed DNS queries.
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(243, 156), Size::new(50, 20)),
                    failure_text.as_str(),
                    &FONT_10X20,
                    Alignment::Center,
                    FAILURE_TEXT,
                )
                .await?;
            }
            Orientation::Portrait | Orientation::PortraitInverted => {
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(22, 68), Size::new(190, 20)),
                    target,
                    &FONT_10X20,
                    Alignment::Left,
                    VALUE_TEXT,
                )
                .await?;
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(60, 119), Size::new(120, 29)),
                    latency.as_str(),
                    &PROFONT_24_POINT,
                    Alignment::Center,
                    VALUE_TEXT,
                )
                .await?;
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(160, 180), Size::new(50, 20)),
                    query_text.as_str(),
                    &FONT_10X20,
                    Alignment::Right,
                    VALUE_TEXT,
                )
                .await?;
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(160, 202), Size::new(50, 20)),
                    success_text.as_str(),
                    &FONT_10X20,
                    Alignment::Right,
                    SUCCESS_TEXT,
                )
                .await?;
                draw_text(
                    display,
                    bitmap,
                    Rectangle::new(Point::new(160, 220), Size::new(50, 20)),
                    failure_text.as_str(),
                    &FONT_10X20,
                    Alignment::Right,
                    FAILURE_TEXT,
                )
                .await?;
            }
        }

        if button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }

        if let Some(touch_event) = touch.read().map_err(Error::Touch)? {
            let touch_event = match touch_event {
                TouchEvent::Down { point } => TouchEvent::Down {
                    point: orientation.map_landscape_point(point),
                },
                touch_event => touch_event,
            };
            let action = match touch_event {
                TouchEvent::Down { point } => match control_at(point, orientation.size()) {
                    Some(Control::Calibration) => Action::ClearCalibrationAndRestart,
                    Some(Control::Wifi) => Action::ResetWifiAndRestart,
                    Some(Control::Orientation) => {
                        orientation = orientation.next();
                        Action::SaveOrientationAndRestart(orientation)
                    }
                    None => Action::StartDnsLookup,
                },
                TouchEvent::Move { .. } | TouchEvent::Up => Action::None,
            };
            let exit = match action {
                Action::None => None,
                Action::StartDnsLookup => {
                    let result = dns_lookup
                        .lookup(target)
                        .await
                        .map_err(Error::Dns)?;
                    queries = queries.saturating_add(1);
                    last_latency_millis = result.latency_millis;
                    if result.succeeded {
                        successes = successes.saturating_add(1);
                    } else {
                        failures = failures.saturating_add(1);
                    }
                    None
                }
                Action::ClearCalibrationAndRestart => {
                    Some(Exit::CalibrationRequested)
                }
                Action::ResetWifiAndRestart => Some(Exit::WifiResetRequested),
                Action::SaveOrientationAndRestart(next_orientation) => {
                    Some(Exit::OrientationChanged(next_orientation))
                }
            };
            if let Some(exit) = exit {
                return Ok(exit);
            }
        }
        yield_now().await;
    }
}

/// Show the DNS Tester splash immediately after display initialization.
pub async fn dns_tester_splash<D>(
    display: &mut D,
    orientation: Orientation,
) -> Result<(), UiError<D::Error>>
where
    D: CydDisplay,
{
    render_notice(display, orientation, UiNotice::Splash).await
}

/// Errors returned by the shared DNS Tester loop.
#[derive(Debug)]
pub enum Error<CydError, DnsError> {
    /// Rendering failed.
    Display(UiError<CydError>),
    /// Reading calibrated touch failed.
    Touch(CydError),
    /// The DNS lookup failed at the platform boundary.
    Dns(DnsError),
}

impl<CydError, DnsError> Error<CydError, DnsError> {
    fn display(error: CydError) -> Self {
        Self::Display(UiError::Display(error))
    }
}

impl<CydError, DnsError> From<UiError<CydError>> for Error<CydError, DnsError> {
    fn from(error: UiError<CydError>) -> Self {
        Self::Display(error)
    }
}

impl<CydError, DnsError> From<core::fmt::Error> for Error<CydError, DnsError> {
    fn from(error: core::fmt::Error) -> Self {
        Self::Display(UiError::Text(error))
    }
}

/// Result returned when the shared DNS Tester loop needs platform handling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Exit {
    /// Clear calibration and restart the platform calibration flow.
    CalibrationRequested,
    /// Clear Wi-Fi credentials and restart the platform setup flow.
    WifiResetRequested,
    /// Persist this orientation and restart the display adapter.
    OrientationChanged(Orientation),
}

/// Platform control request selected by the game loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    /// No platform work is required.
    None,
    /// Start one DNS lookup.
    StartDnsLookup,
    /// Clear calibration and restart the platform calibration flow.
    ClearCalibrationAndRestart,
    /// Clear Wi-Fi configuration and restart the Wi-Fi setup flow.
    ResetWifiAndRestart,
    /// Persist this orientation and restart the display adapter.
    SaveOrientationAndRestart(Orientation),
}

#[derive(Clone, Copy)]
enum Control {
    Calibration,
    Wifi,
    Orientation,
}

fn control_at(point: Point, size: Size) -> Option<Control> {
    let landscape = size.width > size.height;
    [
        (
            Control::Calibration,
            if landscape {
                (10, 202, 100, 28)
            } else {
                (10, 276, 73, 36)
            },
        ),
        (
            Control::Wifi,
            if landscape {
                (110, 202, 100, 28)
            } else {
                (83, 276, 74, 36)
            },
        ),
        (
            Control::Orientation,
            if landscape {
                (210, 202, 100, 28)
            } else {
                (157, 276, 73, 36)
            },
        ),
    ]
    .into_iter()
    .find_map(|(control, (x, y, width, height))| {
        Rectangle::new(Point::new(x, y), Size::new(width, height))
            .contains(point)
            .then_some(control)
    })
}

/// Select the display orientation used while touch calibration is running.
#[must_use]
pub const fn display_orientation_for_calibration(
    saved_orientation: Orientation,
    calibration_is_available: bool,
) -> Orientation {
    if calibration_is_available {
        saved_orientation
    } else {
        Orientation::Landscape
    }
}

/// Return the application orientation after calibration has completed.
#[must_use]
pub const fn orientation_after_calibration(saved_orientation: Orientation) -> Orientation {
    saved_orientation
}

/// A full-screen DNS tester notice shown before the test dashboard is usable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiNotice {
    /// The display has initialized and networking has not started yet.
    Splash,
    /// The device is joining the configured Wi-Fi network.
    WifiConnecting,
    /// Wi-Fi setup is available through the captive portal.
    WifiSetup,
    /// Browser WebAssembly cannot perform arbitrary DNS queries.
    WifiUnavailable,
}

/// Errors from rendering the shared DNS tester UI.
#[derive(Debug)]
pub enum UiError<F> {
    /// The fixed-size text buffer was too small.
    Text(core::fmt::Error),
    /// The display failed to flush.
    Display(F),
}

impl<F> From<core::fmt::Error> for UiError<F> {
    fn from(error: core::fmt::Error) -> Self {
        Self::Text(error)
    }
}

/// Render a full-screen operational notice over the DNS tester artwork.
pub async fn render_notice<D>(
    display: &mut D,
    orientation: Orientation,
    notice: UiNotice,
) -> Result<(), UiError<D::Error>>
where
    D: CydDisplay,
{
    match orientation {
        Orientation::Landscape | Orientation::LandscapeInverted => display
            .fill_contiguous_full(LANDSCAPE_BITMAP.rgb565_iter())
            .map_err(UiError::Display)?,
        Orientation::Portrait | Orientation::PortraitInverted => display
            .fill_contiguous_full(PORTRAIT_BITMAP.rgb565_iter())
            .map_err(UiError::Display)?,
    }

    let (rectangle, heading_y, detail_y) = if orientation.width() > orientation.height() {
        (
            Rectangle::new(Point::new(20, 54), Size::new(280, 112)),
            70,
            110,
        )
    } else {
        (
            Rectangle::new(Point::new(20, 96), Size::new(200, 80)),
            104,
            145,
        )
    };
    let (heading, detail) = match notice {
        UiNotice::Splash => ("DNS", "STARTING"),
        UiNotice::WifiConnecting => ("WI-FI", "CONNECTING"),
        UiNotice::WifiSetup => ("WI-FI", "SETUP"),
        UiNotice::WifiUnavailable => ("WI-FI", "UNAVAILABLE"),
    };
    fill_panel(display, rectangle).await?;
    draw_notice_text(
        display,
        Rectangle::new(Point::new(60, heading_y), Size::new(200, 29)),
        heading,
        &PROFONT_24_POINT,
    )
    .await?;
    draw_notice_text(
        display,
        Rectangle::new(Point::new(60, detail_y), Size::new(200, 20)),
        detail,
        &FONT_10X20,
    )
    .await
}

async fn fill_panel<D>(
    display: &mut D,
    rectangle: Rectangle,
) -> Result<(), UiError<D::Error>>
where
    D: CydDisplay,
{
    let mut offset_y = 0;
    while offset_y < rectangle.size.height {
        let height = (rectangle.size.height - offset_y).min(20);
        let mut frame = display.frame_mut(Rectangle::new(
            rectangle.top_left + Point::new(0, offset_y as i32),
            Size::new(rectangle.size.width, height),
        ));
        frame.fill(Rgb565::from(PANEL_FILL));
        frame.flush().await.map_err(UiError::Display)?;
        offset_y += height;
    }
    Ok(())
}

async fn draw_notice_text<D>(
    display: &mut D,
    rectangle: Rectangle,
    text: &str,
    font: &MonoFont<'_>,
) -> Result<(), UiError<D::Error>>
where
    D: CydDisplay,
{
    let mut frame = display.frame_mut(rectangle);
    frame.fill(Rgb565::from(PANEL_FILL));
    let text_style = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    Text::with_text_style(
        text,
        Point::new((rectangle.size.width / 2) as i32, 0),
        MonoTextStyle::new(font, Rgb565::from(VALUE_TEXT)),
        text_style,
    )
    .draw(&mut frame)
    .unwrap_infallible();
        frame.flush().await.map_err(UiError::Display)
}

async fn draw_text<D>(
    display: &mut D,
    bitmap: Image565View,
    rectangle: Rectangle,
    text: &str,
    font: &MonoFont<'_>,
    alignment: Alignment,
    color: Rgb888,
) -> Result<(), UiError<D::Error>>
where
    D: CydDisplay,
{
    let mut frame = display.frame_mut(rectangle);
    DrawItem::Bitmap {
        view: bitmap,
        top_left: Point::new(-rectangle.top_left.x, -rectangle.top_left.y),
    }
    .draw(&mut frame);
    let position_x = match alignment {
        Alignment::Left => 0,
        Alignment::Center => (rectangle.size.width / 2) as i32,
        Alignment::Right => rectangle.size.width as i32,
    };
    Text::with_text_style(
        text,
        Point::new(position_x, 0),
        MonoTextStyle::new(font, Rgb565::from(color)),
        TextStyleBuilder::new()
            .alignment(alignment)
            .baseline(Baseline::Top)
            .build(),
    )
    .draw(&mut frame)
    .unwrap_infallible();
    frame.flush().await.map_err(UiError::Display)
}
