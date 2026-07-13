//! A shared CYD DNS tester UI renderer.
//!
//! The renderer is platform-neutral: ESP, RP, WASM, and
//! [`CydMemory`](device_envoy_core::memory::CydMemory)
//! all consume the same TGA-backed background and dynamic-value layout.

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
    cyd::{
        CydDisplay,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
        touch::TouchEvent,
    },
};

const LANDSCAPE_BACKGROUND: Image565Fixed<320, 240, { 320 * 240 }> =
    tga!(concat!(env!("OUT_DIR"), "/dns_landscape.tga")).to_565();
const PORTRAIT_BACKGROUND: Image565Fixed<240, 320, { 240 * 320 }> =
    tga!(concat!(env!("OUT_DIR"), "/dns_portrait.tga")).to_565();

const VALUE_TEXT: Rgb888 = Rgb888::new(255, 255, 255); // white
const SUCCESS_TEXT: Rgb888 = Rgb888::new(121, 226, 164); // soft green
const FAILURE_TEXT: Rgb888 = Rgb888::new(255, 117, 110); // coral red
const PANEL_FILL: Rgb888 = Rgb888::new(15, 38, 55); // dark desaturated blue

/// A DNS lookup result supplied by a platform adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DnsResult {
    /// Whether the lookup returned at least one address.
    pub succeeded: bool,
    /// Measured lookup duration in milliseconds.
    pub latency_millis: u64,
}

/// Inputs accepted by the platform-neutral DNS Tester state machine.
#[derive(Clone, Copy, Debug)]
pub enum DnsTesterInput {
    /// A calibrated touch event in the current screen orientation.
    Touch(TouchEvent),
    /// Wi-Fi initialization has started.
    WifiConnecting,
    /// Wi-Fi setup is available.
    WifiSetup,
    /// Wi-Fi and DNS services are ready.
    WifiReady,
    /// A platform DNS adapter has completed a lookup.
    DnsFinished(DnsResult),
    /// The physical or simulated BOOT button was pressed.
    Boot,
}

/// Platform services requested by [`DnsTesterApp::input`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DnsTesterAction {
    /// No platform work is required.
    None,
    /// Start one DNS lookup and report its result with [`DnsTesterInput::DnsFinished`].
    StartDnsLookup,
    /// Clear calibration and restart the platform calibration flow.
    ClearCalibrationAndRestart,
    /// Clear Wi-Fi configuration and restart the Wi-Fi setup flow.
    ResetWifiAndRestart,
    /// Persist this orientation and restart the display adapter.
    SaveOrientationAndRestart(Orientation),
}

/// Physical control placement used by a DNS Tester platform adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DnsTesterLayout {
    /// Full-screen CYD artwork with fixed landscape/portrait controls.
    FullScreen,
    /// Compact text dashboard with three equal controls along the bottom edge.
    Compact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DnsTesterScreen {
    Splash,
    Connecting,
    Setup,
    Dashboard,
    Unavailable,
}

/// Shared DNS Tester state and transition logic used by hardware, WASM, and memory tests.
#[derive(Clone, Copy)]
pub struct DnsTesterApp {
    orientation: Orientation,
    target: &'static str,
    queries: u32,
    successes: u32,
    failures: u32,
    last_latency_millis: u64,
    screen: DnsTesterScreen,
    layout: DnsTesterLayout,
}

#[derive(Clone, Copy)]
enum Control {
    Calibration,
    Wifi,
    Orientation,
}

fn control_at(point: Point, size: Size, layout: DnsTesterLayout) -> Option<Control> {
    if layout == DnsTesterLayout::Compact {
        return [Control::Orientation, Control::Calibration, Control::Wifi]
            .into_iter()
            .find(|control| {
                let index = match control {
                    Control::Orientation => 0,
                    Control::Calibration => 1,
                    Control::Wifi => 2,
                };
                Rectangle::new(
                    Point::new(
                        size.width as i32 - size.width as i32 / 3 * (3 - index),
                        size.height as i32 - 20,
                    ),
                    Size::new(size.width / 3, 20),
                )
                .contains(point)
            });
    }
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

impl DnsTesterApp {
    /// Construct a DNS Tester in its startup splash state.
    #[must_use]
    pub const fn new(target: &'static str, orientation: Orientation) -> Self {
        Self {
            orientation,
            target,
            queries: 0,
            successes: 0,
            failures: 0,
            last_latency_millis: 0,
            screen: DnsTesterScreen::Splash,
            layout: DnsTesterLayout::FullScreen,
        }
    }

    /// Construct a DNS Tester with an explicit platform control layout.
    #[must_use]
    pub const fn new_with_layout(
        target: &'static str,
        orientation: Orientation,
        layout: DnsTesterLayout,
    ) -> Self {
        let mut app = Self::new(target, orientation);
        app.layout = layout;
        app
    }

    /// Select the display orientation used while touch calibration is running.
    ///
    /// Calibration coordinates are defined in landscape. A saved application
    /// orientation is used only when a valid calibration already exists.
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

    /// Return the currently selected screen orientation.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Set the adapter's current orientation after platform initialization or persistence.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    /// Return the current dynamic dashboard values.
    #[must_use]
    pub const fn ui_state(&self) -> DnsTesterUiState {
        DnsTesterUiState {
            target: self.target,
            queries: self.queries,
            successes: self.successes,
            failures: self.failures,
            last_latency_millis: self.last_latency_millis,
        }
    }

    /// Apply one platform event and return any requested platform operation.
    pub fn input(&mut self, input: DnsTesterInput) -> DnsTesterAction {
        match input {
            DnsTesterInput::WifiConnecting => {
                self.screen = DnsTesterScreen::Connecting;
                DnsTesterAction::None
            }
            DnsTesterInput::WifiSetup => {
                self.screen = DnsTesterScreen::Setup;
                DnsTesterAction::None
            }
            DnsTesterInput::WifiReady => {
                self.screen = DnsTesterScreen::Dashboard;
                DnsTesterAction::None
            }
            DnsTesterInput::DnsFinished(result) => {
                self.queries = self.queries.saturating_add(1);
                self.last_latency_millis = result.latency_millis;
                if result.succeeded {
                    self.successes = self.successes.saturating_add(1);
                } else {
                    self.failures = self.failures.saturating_add(1);
                }
                self.screen = DnsTesterScreen::Dashboard;
                DnsTesterAction::None
            }
            DnsTesterInput::Boot => DnsTesterAction::ClearCalibrationAndRestart,
            DnsTesterInput::Touch(TouchEvent::Down { point }) => {
                if let Some(control) = control_at(point, self.orientation.size(), self.layout) {
                    match control {
                        Control::Calibration => DnsTesterAction::ClearCalibrationAndRestart,
                        Control::Wifi => {
                            self.screen = DnsTesterScreen::Unavailable;
                            DnsTesterAction::ResetWifiAndRestart
                        }
                        Control::Orientation => {
                            let orientation = self.orientation.next();
                            self.orientation = orientation;
                            DnsTesterAction::SaveOrientationAndRestart(orientation)
                        }
                    }
                } else {
                    DnsTesterAction::StartDnsLookup
                }
            }
            DnsTesterInput::Touch(TouchEvent::Move { .. } | TouchEvent::Up) => {
                DnsTesterAction::None
            }
        }
    }

    /// Whether the app currently displays its browser-unavailable notice.
    #[must_use]
    pub const fn is_unavailable(&self) -> bool {
        matches!(self.screen, DnsTesterScreen::Unavailable)
    }

    /// Return the pre-dashboard notice, when one should be rendered.
    #[must_use]
    pub const fn notice(&self) -> Option<DnsTesterUiNotice> {
        match self.screen {
            DnsTesterScreen::Splash => Some(DnsTesterUiNotice::Splash),
            DnsTesterScreen::Connecting => Some(DnsTesterUiNotice::WifiConnecting),
            DnsTesterScreen::Setup => Some(DnsTesterUiNotice::WifiSetup),
            DnsTesterScreen::Unavailable => Some(DnsTesterUiNotice::WifiUnavailable),
            DnsTesterScreen::Dashboard => None,
        }
    }
}

/// The changing values shown by the DNS tester UI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DnsTesterUiState {
    /// DNS name for the current test.
    pub target: &'static str,
    /// Number of DNS tests started.
    pub queries: u32,
    /// Number of successful DNS tests.
    pub successes: u32,
    /// Number of failed DNS tests.
    pub failures: u32,
    /// Most recent lookup latency in milliseconds.
    pub last_latency_millis: u64,
}

/// A full-screen DNS tester notice shown before the test dashboard is usable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DnsTesterUiNotice {
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
pub enum DnsTesterUiError<F> {
    /// The fixed-size text buffer was too small.
    Text(core::fmt::Error),
    /// The display failed to flush.
    Display(F),
}

/// Render the shared DNS tester screen.
pub async fn render<D>(
    display: &mut D,
    orientation: Orientation,
    state: DnsTesterUiState,
) -> Result<(), DnsTesterUiError<D::Error>>
where
    D: CydDisplay,
{
    let background = Background::for_orientation(orientation);
    let screen_size = orientation.size();
    display
        .draw_items::<1>(
            Rectangle::new(Point::zero(), screen_size),
            display.background_565(),
            [DrawItem::Bitmap {
                view: background.view(),
                top_left: Point::zero(),
            }],
        )
        .map_err(DnsTesterUiError::Display)?;

    let status = if state.queries == 0 {
        "TAP"
    } else if state.failures > 0 {
        "FAIL"
    } else {
        "OK"
    };
    let mut latency = heapless::String::<16>::new();
    if state.queries == 0 {
        latency
            .push_str("--")
            .map_err(|_| DnsTesterUiError::Text(core::fmt::Error))?;
    } else {
        write!(latency, "{} ms", state.last_latency_millis).map_err(DnsTesterUiError::Text)?;
    }
    let mut queries = heapless::String::<12>::new();
    let mut successes = heapless::String::<12>::new();
    let mut failures = heapless::String::<12>::new();
    write!(queries, "{}", state.queries).map_err(DnsTesterUiError::Text)?;
    write!(successes, "{}", state.successes).map_err(DnsTesterUiError::Text)?;
    write!(failures, "{}", state.failures).map_err(DnsTesterUiError::Text)?;

    if orientation.width() > orientation.height() {
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(22, 76), Size::new(150, 20)),
            state.target,
            &FONT_10X20,
            Alignment::Left,
            VALUE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(244, 82), Size::new(56, 20)),
            status,
            &FONT_10X20,
            Alignment::Center,
            if state.failures > 0 {
                FAILURE_TEXT
            } else {
                SUCCESS_TEXT
            },
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(100, 100), Size::new(120, 29)),
            latency.as_str(),
            &PROFONT_24_POINT,
            Alignment::Center,
            VALUE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(27, 156), Size::new(50, 20)),
            queries.as_str(),
            &FONT_10X20,
            Alignment::Center,
            VALUE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(135, 156), Size::new(50, 20)),
            successes.as_str(),
            &FONT_10X20,
            Alignment::Center,
            SUCCESS_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(243, 156), Size::new(50, 20)),
            failures.as_str(),
            &FONT_10X20,
            Alignment::Center,
            FAILURE_TEXT,
        )
        .await?;
    } else {
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(22, 68), Size::new(190, 20)),
            state.target,
            &FONT_10X20,
            Alignment::Left,
            VALUE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(60, 119), Size::new(120, 29)),
            latency.as_str(),
            &PROFONT_24_POINT,
            Alignment::Center,
            VALUE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(160, 180), Size::new(50, 20)),
            queries.as_str(),
            &FONT_10X20,
            Alignment::Right,
            VALUE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(160, 202), Size::new(50, 20)),
            successes.as_str(),
            &FONT_10X20,
            Alignment::Right,
            SUCCESS_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(160, 220), Size::new(50, 20)),
            failures.as_str(),
            &FONT_10X20,
            Alignment::Right,
            FAILURE_TEXT,
        )
        .await?;
    }
    Ok(())
}

/// Render a full-screen operational notice over the DNS tester artwork.
pub async fn render_notice<D>(
    display: &mut D,
    orientation: Orientation,
    notice: DnsTesterUiNotice,
) -> Result<(), DnsTesterUiError<D::Error>>
where
    D: CydDisplay,
{
    let background = Background::for_orientation(orientation);
    display
        .draw_items::<1>(
            Rectangle::new(Point::zero(), orientation.size()),
            display.background_565(),
            [DrawItem::Bitmap {
                view: background.view(),
                top_left: Point::zero(),
            }],
        )
        .map_err(DnsTesterUiError::Display)?;

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
        DnsTesterUiNotice::Splash => ("DNS", "STARTING"),
        DnsTesterUiNotice::WifiConnecting => ("WI-FI", "CONNECTING"),
        DnsTesterUiNotice::WifiSetup => ("WI-FI", "SETUP"),
        DnsTesterUiNotice::WifiUnavailable => ("WI-FI", "UNAVAILABLE"),
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

/// Render the screen selected by the shared DNS Tester state.
pub async fn render_app<D>(
    display: &mut D,
    app: &DnsTesterApp,
) -> Result<(), DnsTesterUiError<D::Error>>
where
    D: CydDisplay,
{
    match app.notice() {
        Some(notice) => render_notice(display, app.orientation(), notice).await,
        None => render(display, app.orientation(), app.ui_state()).await,
    }
}

async fn fill_panel<D>(
    display: &mut D,
    rectangle: Rectangle,
) -> Result<(), DnsTesterUiError<D::Error>>
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
        frame.flush().await.map_err(DnsTesterUiError::Display)?;
        offset_y += height;
    }
    Ok(())
}

async fn draw_notice_text<D>(
    display: &mut D,
    rectangle: Rectangle,
    text: &str,
    font: &MonoFont<'_>,
) -> Result<(), DnsTesterUiError<D::Error>>
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
    frame.flush().await.map_err(DnsTesterUiError::Display)
}

#[derive(Clone, Copy)]
enum Background {
    Landscape,
    Portrait,
}

impl Background {
    const fn for_orientation(orientation: Orientation) -> Self {
        match orientation {
            Orientation::Landscape | Orientation::LandscapeInverted => Self::Landscape,
            Orientation::Portrait | Orientation::PortraitInverted => Self::Portrait,
        }
    }

    const fn view(self) -> Image565View {
        match self {
            Self::Landscape => LANDSCAPE_BACKGROUND.view(),
            Self::Portrait => PORTRAIT_BACKGROUND.view(),
        }
    }

    const fn view_rect(self, rectangle: Rectangle) -> Image565View {
        match self {
            Self::Landscape => LANDSCAPE_BACKGROUND.view_rect(rectangle),
            Self::Portrait => PORTRAIT_BACKGROUND.view_rect(rectangle),
        }
    }
}

async fn draw_text<D>(
    display: &mut D,
    background: Background,
    rectangle: Rectangle,
    text: &str,
    font: &MonoFont<'_>,
    alignment: Alignment,
    color: Rgb888,
) -> Result<(), DnsTesterUiError<D::Error>>
where
    D: CydDisplay,
{
    let mut frame = display.frame_mut(rectangle);
    DrawItem::Bitmap {
        view: background.view_rect(rectangle),
        top_left: Point::zero(),
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
    frame.flush().await.map_err(DnsTesterUiError::Display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_control_cycles_through_all_presentations() {
        let mut app = DnsTesterApp::new("example.com", Orientation::Landscape);
        for expected_orientation in [
            Orientation::Portrait,
            Orientation::LandscapeInverted,
            Orientation::PortraitInverted,
            Orientation::Landscape,
        ] {
            let action = app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: Point::new(
                    if app.orientation().width() > app.orientation().height() {
                        260
                    } else {
                        193
                    },
                    if app.orientation().width() > app.orientation().height() {
                        216
                    } else {
                        294
                    },
                ),
            }));
            assert_eq!(
                action,
                DnsTesterAction::SaveOrientationAndRestart(expected_orientation)
            );
            assert_eq!(app.orientation(), expected_orientation);
        }
    }

    #[test]
    fn dns_results_update_shared_counters() {
        let mut app = DnsTesterApp::new("example.com", Orientation::Landscape);
        app.input(DnsTesterInput::DnsFinished(DnsResult {
            succeeded: true,
            latency_millis: 12,
        }));
        app.input(DnsTesterInput::DnsFinished(DnsResult {
            succeeded: false,
            latency_millis: 18,
        }));
        assert_eq!(app.ui_state().queries, 2);
        assert_eq!(app.ui_state().successes, 1);
        assert_eq!(app.ui_state().failures, 1);
        assert_eq!(app.ui_state().last_latency_millis, 18);
    }

    #[test]
    fn compact_layout_routes_settings_controls_before_dns_lookup() {
        let mut app = DnsTesterApp::new_with_layout(
            "example.com",
            Orientation::Landscape,
            DnsTesterLayout::Compact,
        );
        assert_eq!(
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: Point::new(50, 230),
            })),
            DnsTesterAction::SaveOrientationAndRestart(Orientation::Portrait)
        );
        let mut wifi_app = DnsTesterApp::new_with_layout(
            "example.com",
            Orientation::Landscape,
            DnsTesterLayout::Compact,
        );
        assert_eq!(
            wifi_app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: Point::new(250, 230),
            })),
            DnsTesterAction::ResetWifiAndRestart
        );
    }
}
