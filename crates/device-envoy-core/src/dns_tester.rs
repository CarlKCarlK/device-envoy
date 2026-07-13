//! A shared CYD DNS tester UI renderer.
//!
//! The renderer is platform-neutral: ESP, RP, WASM, and [`CydMemory`](crate::memory::CydMemory)
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

use crate::{
    UnwrapInfallible,
    cyd::{
        CydDisplay,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
    },
};

const LANDSCAPE_BACKGROUND: Image565Fixed<320, 240, { 320 * 240 }> =
    tga!("../docs/assets/dns_tester/dns_landscape.tga").to_565();
const PORTRAIT_BACKGROUND: Image565Fixed<240, 320, { 240 * 320 }> =
    tga!("../docs/assets/dns_tester/dns_portrait.tga").to_565();

const VALUE_TEXT: Rgb888 = Rgb888::new(255, 255, 255); // white
const SUCCESS_TEXT: Rgb888 = Rgb888::new(121, 226, 164); // soft green
const FAILURE_TEXT: Rgb888 = Rgb888::new(255, 117, 110); // coral red
const PANEL_FILL: Rgb888 = Rgb888::new(15, 38, 55); // dark desaturated blue

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
    /// The device is joining the configured Wi-Fi network.
    WifiConnecting,
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
        DnsTesterUiNotice::WifiConnecting => ("WI-FI", "CONNECTING"),
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
