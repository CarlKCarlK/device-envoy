//! A shared CYD DNS tester UI renderer.
//!
//! The renderer is platform-neutral: ESP, RP, WASM, and [`CydMemory`](crate::memory::CydMemory)
//! all consume the same TGA-backed background and dynamic-value layout.

use core::fmt::Write;

use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::{Rgb565, Rgb888},
    primitives::Rectangle,
    text::{Baseline, Text},
};

use crate::{
    UnwrapInfallible,
    cyd::{
        CydDisplay,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
    },
};

const LANDSCAPE_BACKGROUND: Image565Fixed<320, 240, { 320 * 240 }> =
    tga!("../docs/assets/dns_tester_background_landscape.tga").to_565();
const PORTRAIT_BACKGROUND: Image565Fixed<240, 320, { 240 * 320 }> =
    tga!("../docs/assets/dns_tester_background_portrait.tga").to_565();

const PALE_TEXT: Rgb888 = Rgb888::new(185, 210, 223); // pale blue-white
const CYAN_TEXT: Rgb888 = Rgb888::new(127, 220, 255); // bright cyan
const SUCCESS_TEXT: Rgb888 = Rgb888::new(130, 220, 150); // soft green
const FAILURE_TEXT: Rgb888 = Rgb888::new(240, 125, 115); // coral red

/// The changing values shown by the DNS tester UI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DnsTesterUiState {
    /// Number of DNS tests started.
    pub queries: u32,
    /// Number of successful DNS tests.
    pub successes: u32,
    /// Number of failed DNS tests.
    pub failures: u32,
    /// Most recent lookup latency in milliseconds.
    pub last_latency_millis: u64,
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
        "TAP TO TEST"
    } else {
        "READY"
    };
    let mut latency = heapless::String::<16>::new();
    if state.queries == 0 {
        latency
            .push('—')
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
            Rectangle::new(Point::new(220, 0), Size::new(100, 20)),
            status,
            CYAN_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(130, 78), Size::new(60, 28)),
            latency.as_str(),
            PALE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(70, 164), Size::new(38, 18)),
            queries.as_str(),
            PALE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(177, 164), Size::new(38, 18)),
            successes.as_str(),
            SUCCESS_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(282, 164), Size::new(38, 18)),
            failures.as_str(),
            FAILURE_TEXT,
        )
        .await?;
    } else {
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(150, 0), Size::new(90, 20)),
            status,
            CYAN_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(100, 98), Size::new(40, 25)),
            latency.as_str(),
            PALE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(195, 168), Size::new(45, 16)),
            queries.as_str(),
            PALE_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(195, 185), Size::new(45, 16)),
            successes.as_str(),
            SUCCESS_TEXT,
        )
        .await?;
        draw_text(
            display,
            background,
            Rectangle::new(Point::new(195, 202), Size::new(45, 16)),
            failures.as_str(),
            FAILURE_TEXT,
        )
        .await?;
    }
    Ok(())
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
    Text::with_baseline(
        text,
        Point::zero(),
        MonoTextStyle::new(&FONT_6X10, Rgb565::from(color)),
        Baseline::Top,
    )
    .draw(&mut frame)
    .unwrap_infallible();
    frame.flush().await.map_err(DnsTesterUiError::Display)
}
