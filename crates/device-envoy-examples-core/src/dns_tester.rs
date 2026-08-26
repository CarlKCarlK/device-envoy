//! A shared CYD DNS tester game loop and UI.
//!
//! [`run`] is the platform-neutral game loop: ESP, RP, WASM, and
//! [`CydMemory`](device_envoy_core::memory::CydMemory)
//! provide resources and events while it owns state transitions, commands,
//! rendering, and the redraw schedule. All platforms consume the same
//! TGA-backed bitmap and dynamic-value layout.

use core::{
    convert::Infallible,
    fmt::{self, Write},
};

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
        display::{CydFrame, DrawItem, Image565View, Orientation, tga},
        touch::TouchEvent,
    },
    dns::Dns,
    wifi_auto::WifiAutoEvent,
};
use embassy_futures::yield_now;
use embassy_time::Instant;

const LANDSCAPE_BITMAP: Image565View =
    tga!(concat!(env!("OUT_DIR"), "/dns_landscape.tga"), 320, 240)
        .to_565()
        .view();
const PORTRAIT_BITMAP: Image565View = tga!(concat!(env!("OUT_DIR"), "/dns_portrait.tga"), 240, 320)
    .to_565()
    .view();

const VALUE_TEXT: Rgb888 = Rgb888::new(255, 255, 255); // white
const SUCCESS_TEXT: Rgb888 = Rgb888::new(121, 226, 164); // soft green
const FAILURE_TEXT: Rgb888 = Rgb888::new(255, 117, 110); // coral red
const PANEL_FILL: Rgb888 = Rgb888::new(15, 38, 55); // dark desaturated blue
const ARTWORK_PANEL_FILL: Rgb888 = Rgb888::new(10, 82, 120); // deep blue panel

pub const DNS_HOSTNAME: &str = "example.com";

/// Run the shared DNS Tester loop on calibrated platform resources.
///
/// Platform setup follows the Linkage Blaze pattern: construct and calibrate
/// the display and touch resources, show the splash, connect Wi-Fi, then call
/// this function. The loop owns input policy, application state, DNS result
/// accounting, and rendering. Call [`splash`] during platform setup
/// before entering this loop. The loop returns platform control
/// requests instead of knowing how persistence or reboot works.
pub async fn run<CydDevice, ButtonDevice, DnsDevice>(
    cyd: &mut CydDevice,
    button: &mut ButtonDevice,
    dns: &mut DnsDevice,
) -> Result<Exit, Error<CydDevice::Error, DnsDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    DnsDevice: Dns,
{
    run_inner(cyd, button, dns).await
}

async fn run_inner<CydDevice, ButtonDevice, DnsDevice>(
    cyd: &mut CydDevice,
    button: &mut ButtonDevice,
    dns: &mut DnsDevice,
) -> Result<Exit, Error<CydDevice::Error, DnsDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    DnsDevice: Dns,
{
    let mut queries: u32 = 0;
    let mut successes: u32 = 0;
    let mut failures: u32 = 0;
    let mut latency_millis = None;
    let mut status = Status::Tap;

    let orientation = cyd.orientation();
    let layout = match orientation {
        Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_LAYOUT,
        Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_LAYOUT,
    };
    let (display, touch) = cyd.parts();
    let mut ui = Ui::<_, 16>::new(display, layout, orientation);
    ui.fill_bitmap()?;
    ui.text(layout.hostname, DNS_HOSTNAME).await?;
    loop {
        yield_now().await;

        if button.is_pressed() {
            return Ok(Exit::Calibrate);
        }

        ui.status(layout.status, status, status.is_good()).await?;

        match latency_millis {
            Some(latency_millis) => {
                ui.value(layout.latency, format_args!("{latency_millis} ms"))
                    .await?
            }
            None => ui.value(layout.latency, format_args!("--")).await?,
        }

        ui.value(layout.queries, format_args!("{queries}")).await?;

        ui.value(layout.successes, format_args!("{successes}"))
            .await?;

        ui.value(layout.failures, format_args!("{failures}"))
            .await?;

        match ui.touch(touch.read()?) {
            TouchAction::None => {}
            TouchAction::StartDns => {
                let start = Instant::now();
                let addresses = dns.resolve(DNS_HOSTNAME).await.map_err(Error::Dns)?;
                latency_millis = Some(start.elapsed().as_millis());

                queries = queries.saturating_add(1);
                if addresses.is_empty() {
                    failures = failures.saturating_add(1);
                    status = Status::Fail;
                } else {
                    successes = successes.saturating_add(1);
                    if matches!(status, Status::Tap) {
                        status = Status::Ok;
                    }
                }
            }
            TouchAction::Control(Control::Calibration) => {
                return Ok(Exit::Calibrate);
            }
            TouchAction::Control(Control::Wifi) => {
                return Ok(Exit::ResetWifi);
            }
            TouchAction::Control(Control::Orientation) => {
                return Ok(Exit::Reorientate(orientation.next()));
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Layout {
    bitmap: Image565View,
    hostname: TextSlot,
    latency: TextSlot,
    status: StatusSlot,
    queries: TextSlot,
    successes: TextSlot,
    failures: TextSlot,
    taps: [TapRegion; 3],
}

#[derive(Clone, Copy)]
struct TextSlot {
    rectangle: Rectangle,
    font: TextFont,
    alignment: Alignment,
    color: Rgb888,
}

#[derive(Clone, Copy)]
struct StatusSlot {
    text: TextSlot,
    success_color: Rgb888,
    failure_color: Rgb888,
}

#[derive(Clone, Copy)]
struct TapRegion {
    rectangle: Rectangle,
    control: Control,
}

#[derive(Clone, Copy)]
enum TextFont {
    Body,
    Latency,
}

impl TextSlot {
    const fn new(
        rectangle: Rectangle,
        font: TextFont,
        alignment: Alignment,
        color: Rgb888,
    ) -> Self {
        Self {
            rectangle,
            font,
            alignment,
            color,
        }
    }
}

impl StatusSlot {
    const fn new(text: TextSlot, success_color: Rgb888, failure_color: Rgb888) -> Self {
        Self {
            text,
            success_color,
            failure_color,
        }
    }
}

impl TapRegion {
    const fn new(rectangle: Rectangle, control: Control) -> Self {
        Self { rectangle, control }
    }
}

const LANDSCAPE_LAYOUT: Layout = Layout {
    bitmap: LANDSCAPE_BITMAP,
    hostname: TextSlot::new(
        Rectangle::new(Point::new(22, 76), Size::new(150, 20)),
        TextFont::Body,
        Alignment::Left,
        VALUE_TEXT,
    ),
    latency: TextSlot::new(
        Rectangle::new(Point::new(100, 100), Size::new(120, 24)),
        TextFont::Latency,
        Alignment::Center,
        VALUE_TEXT,
    ),
    status: StatusSlot::new(
        TextSlot::new(
            Rectangle::new(Point::new(244, 82), Size::new(56, 20)),
            TextFont::Body,
            Alignment::Center,
            SUCCESS_TEXT,
        ),
        SUCCESS_TEXT,
        FAILURE_TEXT,
    ),
    queries: TextSlot::new(
        Rectangle::new(Point::new(27, 156), Size::new(50, 20)),
        TextFont::Body,
        Alignment::Center,
        VALUE_TEXT,
    ),
    successes: TextSlot::new(
        Rectangle::new(Point::new(135, 156), Size::new(50, 20)),
        TextFont::Body,
        Alignment::Center,
        SUCCESS_TEXT,
    ),
    failures: TextSlot::new(
        Rectangle::new(Point::new(243, 156), Size::new(50, 20)),
        TextFont::Body,
        Alignment::Center,
        FAILURE_TEXT,
    ),
    taps: [
        TapRegion::new(
            Rectangle::new(Point::new(10, 202), Size::new(100, 28)),
            Control::Calibration,
        ),
        TapRegion::new(
            Rectangle::new(Point::new(110, 202), Size::new(100, 28)),
            Control::Wifi,
        ),
        TapRegion::new(
            Rectangle::new(Point::new(210, 202), Size::new(100, 28)),
            Control::Orientation,
        ),
    ],
};

const PORTRAIT_LAYOUT: Layout = Layout {
    bitmap: PORTRAIT_BITMAP,
    hostname: TextSlot::new(
        Rectangle::new(Point::new(22, 68), Size::new(130, 20)),
        TextFont::Body,
        Alignment::Left,
        VALUE_TEXT,
    ),
    latency: TextSlot::new(
        Rectangle::new(Point::new(60, 119), Size::new(120, 24)),
        TextFont::Latency,
        Alignment::Center,
        VALUE_TEXT,
    ),
    status: StatusSlot::new(
        TextSlot::new(
            Rectangle::new(Point::new(170, 68), Size::new(50, 20)),
            TextFont::Body,
            Alignment::Center,
            SUCCESS_TEXT,
        ),
        SUCCESS_TEXT,
        FAILURE_TEXT,
    ),
    queries: TextSlot::new(
        Rectangle::new(Point::new(160, 180), Size::new(50, 20)),
        TextFont::Body,
        Alignment::Right,
        VALUE_TEXT,
    ),
    successes: TextSlot::new(
        Rectangle::new(Point::new(160, 202), Size::new(50, 20)),
        TextFont::Body,
        Alignment::Right,
        SUCCESS_TEXT,
    ),
    failures: TextSlot::new(
        Rectangle::new(Point::new(160, 220), Size::new(50, 20)),
        TextFont::Body,
        Alignment::Right,
        FAILURE_TEXT,
    ),
    taps: [
        TapRegion::new(
            Rectangle::new(Point::new(10, 276), Size::new(73, 36)),
            Control::Calibration,
        ),
        TapRegion::new(
            Rectangle::new(Point::new(83, 276), Size::new(74, 36)),
            Control::Wifi,
        ),
        TapRegion::new(
            Rectangle::new(Point::new(157, 276), Size::new(73, 36)),
            Control::Orientation,
        ),
    ],
};

/// Largest buffered frame used by the DNS tester UI.
///
/// Full-screen artwork is streamed directly; buffered fills are split into
/// twenty-pixel rows, with the landscape panel being the widest such frame.
pub const FRAME_PIXEL_COUNT: usize = 240 * 20;

struct Ui<'a, Display, const TEXT_CAPACITY: usize> {
    display: &'a mut Display,
    layout: Layout,
    orientation: Orientation,
    text: heapless::String<TEXT_CAPACITY>,
}

impl<'a, Display, const TEXT_CAPACITY: usize> Ui<'a, Display, TEXT_CAPACITY>
where
    Display: CydDisplay,
{
    fn new(display: &'a mut Display, layout: Layout, orientation: Orientation) -> Self {
        Self {
            display,
            layout,
            orientation,
            text: heapless::String::new(),
        }
    }

    fn touch(&self, touch_event: Option<TouchEvent>) -> TouchAction {
        let touch_event = touch_event.map(|touch_event| match touch_event {
            TouchEvent::Down { point } => TouchEvent::Down {
                point: self.orientation.map_landscape_point(point),
            },
            touch_event => touch_event,
        });

        match touch_event {
            Some(TouchEvent::Down { point }) => self
                .layout
                .control_at(point)
                .map_or(TouchAction::StartDns, TouchAction::Control),
            Some(TouchEvent::Move { .. }) | Some(TouchEvent::Up) | None => TouchAction::None,
        }
    }

    fn fill_bitmap(&mut self) -> Result<(), render::Error<Display::Error>> {
        let rectangle = Rectangle::new(Point::zero(), self.layout.bitmap.size());
        self.display
            .fill_contiguous(rectangle, self.layout.bitmap.rgb565_iter())?;
        Ok(())
    }

    async fn text(
        &mut self,
        slot: TextSlot,
        text: impl AsRef<str>,
    ) -> Result<(), render::Error<Display::Error>> {
        draw_text(
            self.display,
            self.layout.bitmap,
            slot,
            text.as_ref(),
            slot.color,
        )
        .await
    }

    async fn value(
        &mut self,
        slot: TextSlot,
        arguments: fmt::Arguments<'_>,
    ) -> Result<(), render::Error<Display::Error>> {
        self.text.clear();
        if self.text.write_fmt(arguments).is_err() {
            return Err(render::Error::Text(fmt::Error));
        }
        draw_text(
            self.display,
            self.layout.bitmap,
            slot,
            self.text.as_str(),
            slot.color,
        )
        .await
    }

    async fn status(
        &mut self,
        slot: StatusSlot,
        text: impl AsRef<str>,
        is_good: bool,
    ) -> Result<(), render::Error<Display::Error>> {
        let text = text.as_ref();
        draw_text(
            self.display,
            self.layout.bitmap,
            slot.text,
            text,
            if is_good {
                slot.success_color
            } else {
                slot.failure_color
            },
        )
        .await
    }
}

async fn draw_text<Display>(
    display: &mut Display,
    bitmap: Image565View,
    slot: TextSlot,
    text: &str,
    color: Rgb888,
) -> Result<(), render::Error<Display::Error>>
where
    Display: CydDisplay,
{
    let rectangle = slot.rectangle;
    let mut frame = display.frame_mut(rectangle);
    DrawItem::Bitmap {
        view: bitmap,
        top_left: Point::new(-rectangle.top_left.x, -rectangle.top_left.y),
    }
    .draw(&mut frame);
    let position = match slot.alignment {
        Alignment::Left => Point::zero(),
        Alignment::Center => Point::new((rectangle.size.width / 2) as i32, 0),
        Alignment::Right => Point::new(rectangle.size.width as i32, 0),
    };
    Text::with_text_style(
        text,
        position,
        MonoTextStyle::new(slot.font.value(), Rgb565::from(color)),
        TextStyleBuilder::new()
            .alignment(slot.alignment)
            .baseline(Baseline::Top)
            .build(),
    )
    .draw(&mut frame)
    .unwrap_infallible();
    frame.flush().await?;
    drop(frame);
    Ok(())
}

impl TextFont {
    fn value(self) -> &'static MonoFont<'static> {
        match self {
            Self::Body => &FONT_10X20,
            Self::Latency => &PROFONT_24_POINT,
        }
    }
}

impl Layout {
    fn control_at(self, point: Point) -> Option<Control> {
        self.taps
            .into_iter()
            .find(|tap_region| tap_region.rectangle.contains(point))
            .map(|tap_region| tap_region.control)
    }
}

/// Show the DNS Tester splash on a calibrated CYD.
pub async fn splash<CydDevice>(
    cyd: &mut CydDevice,
) -> Result<(), Error<CydDevice::Error, Infallible>>
where
    CydDevice: Cyd,
{
    let orientation = cyd.orientation();
    render_notice(cyd.display(), orientation, UiNotice::Splash).await?;
    Ok(())
}

/// Show the DNS Tester Wi-Fi status for a connection event.
pub async fn wifi_status<CydDevice>(
    cyd: &mut CydDevice,
    wifi_auto_event: WifiAutoEvent,
) -> Result<(), Error<CydDevice::Error, Infallible>>
where
    CydDevice: Cyd,
{
    let orientation = cyd.orientation();
    let notice = match wifi_auto_event {
        WifiAutoEvent::CaptivePortalReady => UiNotice::WifiSetup,
        WifiAutoEvent::Connecting { .. } => UiNotice::WifiConnecting,
        WifiAutoEvent::ConnectionFailed => UiNotice::WifiFailed,
    };
    render_notice(cyd.display(), orientation, notice).await?;
    Ok(())
}

/// Errors returned by the shared DNS Tester loop.
// `Dns` and `render::Error::Display` remain explicit because blanket conversions for
// their generic error types would collide with the other generic conversions.
#[derive(Debug, derive_more::From)]
pub enum Error<CydError, DnsError> {
    /// Rendering failed.
    Display(render::Error<CydError>),
    /// Reading calibrated touch failed.
    Touch(CydError),
    /// The DNS lookup failed at the platform boundary.
    #[from(ignore)]
    Dns(DnsError),
}

/// Result returned when the shared DNS Tester loop needs platform handling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Exit {
    /// Clear calibration and restart the platform calibration flow.
    Calibrate,
    /// Clear Wi-Fi credentials and restart the platform setup flow.
    ResetWifi,
    /// Persist this orientation and restart the display adapter.
    Reorientate(Orientation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TouchAction {
    None,
    StartDns,
    Control(Control),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Tap,
    Ok,
    Fail,
}

impl Status {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tap => "TAP",
            Self::Ok => "OK",
            Self::Fail => "FAIL",
        }
    }

    const fn is_good(self) -> bool {
        !matches!(self, Self::Fail)
    }
}

impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Control {
    Calibration,
    Wifi,
    Orientation,
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
    /// Wi-Fi connection attempts are exhausted.
    WifiFailed,
    /// Browser WebAssembly cannot perform arbitrary DNS queries.
    WifiUnavailable,
}

/// Errors from rendering the shared DNS tester UI.
// The generic display error is explicit for the same coherence reason as the
// generic device and DNS errors in [`Error`].
pub mod render {
    #[derive(Debug, derive_more::From)]
    pub enum Error<F> {
        /// The fixed-size text buffer was too small.
        #[from(ignore)]
        Text(core::fmt::Error),
        /// The display failed to flush.
        Display(F),
    }
}

/// Render a full-screen operational notice over the DNS tester artwork.
pub async fn render_notice<D>(
    display: &mut D,
    orientation: Orientation,
    notice: UiNotice,
) -> Result<(), render::Error<D::Error>>
where
    D: CydDisplay,
{
    match orientation {
        Orientation::Landscape | Orientation::LandscapeInverted => {
            display.fill_contiguous(
                Rectangle::new(Point::zero(), LANDSCAPE_BITMAP.size()),
                LANDSCAPE_BITMAP.rgb565_iter(),
            )?
        }
        Orientation::Portrait | Orientation::PortraitInverted => {
            display.fill_contiguous(
                Rectangle::new(Point::zero(), PORTRAIT_BITMAP.size()),
                PORTRAIT_BITMAP.rgb565_iter(),
            )?
        }
    }

    // READY belongs to the live dashboard, not to a startup or operational
    // notice. Mask the static artwork indicator while the notice is shown.
    let ready_rectangle = if orientation.width() > orientation.height() {
        Rectangle::new(Point::new(236, 14), Size::new(68, 28))
    } else {
        Rectangle::new(Point::new(164, 14), Size::new(66, 28))
    };
    fill_artwork_panel(display, ready_rectangle).await?;

    let (rectangle, heading_y, detail_y) = if orientation.width() > orientation.height() {
        (
            Rectangle::new(Point::new(20, 54), Size::new(280, 112)),
            70,
            110,
        )
    } else {
        // The portrait dashboard has several static labels and controls around
        // the notice rectangle. Clear those regions so a splash is a splash,
        // rather than a partially obscured dashboard.
        fill_panel(
            display,
            Rectangle::new(Point::new(0, 43), Size::new(240, 203)),
        )
        .await?;
        fill_panel(
            display,
            Rectangle::new(Point::new(0, 246), Size::new(240, 66)),
        )
        .await?;
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
        UiNotice::WifiFailed => ("WI-FI", "FAILED"),
        UiNotice::WifiUnavailable => ("WI-FI", "UNAVAILABLE"),
    };
    fill_panel(display, rectangle).await?;
    let text_x = if orientation.width() > orientation.height() {
        60
    } else {
        20
    };
    draw_notice_text(
        display,
        Rectangle::new(Point::new(text_x, heading_y), Size::new(200, 29)),
        heading,
        &PROFONT_24_POINT,
    )
    .await?;
    draw_notice_text(
        display,
        Rectangle::new(Point::new(text_x, detail_y), Size::new(200, 20)),
        detail,
        &FONT_10X20,
    )
    .await
}

async fn fill_panel<D>(display: &mut D, rectangle: Rectangle) -> Result<(), render::Error<D::Error>>
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
        frame.flush().await?;
        offset_y += height;
    }
    Ok(())
}

async fn fill_artwork_panel<D>(
    display: &mut D,
    rectangle: Rectangle,
) -> Result<(), render::Error<D::Error>>
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
        frame.fill(Rgb565::from(ARTWORK_PANEL_FILL));
        frame.flush().await?;
        offset_y += height;
    }
    Ok(())
}

async fn draw_notice_text<D>(
    display: &mut D,
    rectangle: Rectangle,
    text: &str,
    font: &MonoFont<'_>,
) -> Result<(), render::Error<D::Error>>
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
    frame.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_contained(rectangle: Rectangle, screen_size: Size) {
        let screen = Rectangle::new(Point::zero(), screen_size);
        let bottom_right = rectangle.top_left
            + Point::new(
                rectangle.size.width as i32 - 1,
                rectangle.size.height as i32 - 1,
            );
        assert!(screen.contains(rectangle.top_left));
        assert!(screen.contains(bottom_right));
    }

    fn assert_layout_invariants(layout: Layout) {
        let screen_size = layout.bitmap.size();
        let slots = [
            layout.hostname,
            layout.latency,
            layout.queries,
            layout.successes,
            layout.failures,
        ];
        for slot in slots {
            assert_contained(slot.rectangle, screen_size);
        }
        assert_contained(layout.status.text.rectangle, screen_size);

        for tap_region in layout.taps {
            assert_contained(tap_region.rectangle, screen_size);
        }
        for (first_index, first_tap) in layout.taps.iter().enumerate() {
            for second_tap in layout.taps.iter().skip(first_index + 1) {
                assert_eq!(
                    first_tap.rectangle.intersection(&second_tap.rectangle).size,
                    Size::zero()
                );
            }
        }

        assert_eq!(
            layout
                .taps
                .iter()
                .filter(|tap| tap.control == Control::Calibration)
                .count(),
            1
        );
        assert_eq!(
            layout
                .taps
                .iter()
                .filter(|tap| tap.control == Control::Wifi)
                .count(),
            1
        );
        assert_eq!(
            layout
                .taps
                .iter()
                .filter(|tap| tap.control == Control::Orientation)
                .count(),
            1
        );
    }

    #[test]
    fn manual_layouts_fit_their_screens_and_have_disjoint_controls() {
        assert_layout_invariants(LANDSCAPE_LAYOUT);
        assert_layout_invariants(PORTRAIT_LAYOUT);
    }

    #[test]
    fn orientation_selects_a_matching_screen_and_layout() {
        let landscape_layout = match Orientation::Landscape {
            Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_LAYOUT,
            Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_LAYOUT,
        };
        let landscape_inverted_layout = match Orientation::LandscapeInverted {
            Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_LAYOUT,
            Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_LAYOUT,
        };
        let portrait_layout = match Orientation::Portrait {
            Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_LAYOUT,
            Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_LAYOUT,
        };
        let portrait_inverted_layout = match Orientation::PortraitInverted {
            Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_LAYOUT,
            Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_LAYOUT,
        };
        assert_eq!(landscape_layout.bitmap.size(), Size::new(320, 240));
        assert_eq!(
            landscape_inverted_layout.hostname.rectangle,
            LANDSCAPE_LAYOUT.hostname.rectangle
        );
        assert_eq!(portrait_layout.bitmap.size(), Size::new(240, 320));
        assert_eq!(
            portrait_inverted_layout.hostname.rectangle,
            PORTRAIT_LAYOUT.hostname.rectangle
        );
    }

    #[test]
    fn status_slot_has_distinct_success_and_failure_colors() {
        assert_eq!(LANDSCAPE_LAYOUT.status.success_color, SUCCESS_TEXT);
        assert_eq!(LANDSCAPE_LAYOUT.status.failure_color, FAILURE_TEXT);
        assert_eq!(LANDSCAPE_LAYOUT.status.text.color, SUCCESS_TEXT);
        assert_eq!(PORTRAIT_LAYOUT.status.success_color, SUCCESS_TEXT);
        assert_eq!(PORTRAIT_LAYOUT.status.failure_color, FAILURE_TEXT);
    }

    #[test]
    fn control_hit_testing_uses_rectangle_boundaries() {
        for layout in [LANDSCAPE_LAYOUT, PORTRAIT_LAYOUT] {
            let [calibration, wifi, orientation] = layout.taps;
            assert_eq!(
                layout.control_at(calibration.rectangle.top_left),
                Some(Control::Calibration)
            );
            assert_eq!(
                layout.control_at(wifi.rectangle.top_left),
                Some(Control::Wifi)
            );
            assert_eq!(
                layout.control_at(orientation.rectangle.top_left),
                Some(Control::Orientation)
            );
            assert_eq!(
                layout.control_at(Point::new(
                    calibration.rectangle.top_left.x + calibration.rectangle.size.width as i32,
                    calibration.rectangle.top_left.y,
                )),
                Some(Control::Wifi)
            );
            assert_eq!(
                layout.control_at(Point::new(
                    calibration.rectangle.top_left.x,
                    calibration.rectangle.top_left.y - 1,
                )),
                None
            );
            assert_eq!(layout.control_at(Point::new(0, 0)), None);
        }
    }
}
