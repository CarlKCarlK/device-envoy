#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("cyd_memory_bitmap", "docs/assets/cyd_memory_bitmap.png")
)]
//! In-memory [`Button`] mocks and a CYD test harness for host-side tests.
//!
//! Requires the `host` feature. Script touch and button input into
//! [`CydMemory`]/[`ButtonMemory`], then assert on drawn pixels, flush counts,
//! or compare against golden-image PNGs with
//! [`assert_framebuffer_matches_expected_png`].
//!
//! ```rust
//! use device_envoy_core::cyd::{Cyd, CydDisplay};
//! use device_envoy_core::memory::CydMemory;
//! use device_envoy_core::cyd::display::{CydFrame, DrawItem, Image565View};
//! use embedded_graphics::{
//!     mono_font::ascii::FONT_9X15_BOLD,
//!     pixelcolor::Rgb888,
//!     prelude::{Point, RgbColor, Size},
//! };
//! use futures_executor::block_on;
//!
//! const BITMAP_WIDTH: usize = 64;
//! const BITMAP_HEIGHT: usize = 64;
//! const BITMAP_PIXEL_COUNT: usize = BITMAP_WIDTH * BITMAP_HEIGHT;
//! const BITMAP_COLOR0: u16 = 0xfbe0;
//! const BITMAP_COLOR1: u16 = 0x051f;
//! const BITMAP_COLOR2: u16 = 0xffff;
//!
//! const fn cyd_memory_bitmap_pixels() -> [u16; BITMAP_PIXEL_COUNT] {
//!     let mut pixels = [0u16; BITMAP_PIXEL_COUNT];
//!     let mut y = 0;
//!     while y < BITMAP_HEIGHT {
//!         let mut x = 0;
//!         while x < BITMAP_WIDTH {
//!             let edge = x < 2 || y < 2 || x >= BITMAP_WIDTH - 2 || y >= BITMAP_HEIGHT - 2;
//!             let diagonal = x == y || x + y == BITMAP_WIDTH - 1;
//!             pixels[y * BITMAP_WIDTH + x] = if edge {
//!                 BITMAP_COLOR2
//!             } else if diagonal {
//!                 BITMAP_COLOR1
//!             } else {
//!                 BITMAP_COLOR0
//!             };
//!             x += 1;
//!         }
//!         y += 1;
//!     }
//!     pixels
//! }
//!
//! static BITMAP_PIXELS: [u16; BITMAP_PIXEL_COUNT] = cyd_memory_bitmap_pixels();
//!
//! let mut cyd_memory = CydMemory::new(
//!     Size::new(320, 240),
//!     Rgb888::BLACK,
//!     Rgb888::WHITE,
//!     &FONT_9X15_BOLD,
//! );
//! let (mut display, _touch) = Cyd::parts(&mut cyd_memory);
//! let mut frame = display.full_frame_mut();
//! frame.write_text("Hello CYD");
//! DrawItem::Bitmap {
//!     view: Image565View::new(
//!         &BITMAP_PIXELS,
//!         Size::new(BITMAP_WIDTH as u32, BITMAP_HEIGHT as u32),
//!     ),
//!     top_left: Point::new(128, 88),
//! }
//! .draw(&mut frame);
//! block_on(frame.flush())?;
//! # if let Err(error) = device_envoy_core::memory::assert_framebuffer_matches_expected_png(
//! #     &cyd_memory,
//! #     env!("CARGO_MANIFEST_DIR"),
//! #     "cyd_memory_bitmap.png",
//! # ) {
//! #     panic!("{error}");
//! # }
//! # Ok::<(), device_envoy_core::memory::CydMemoryError>(())
//! ```
//!
//! ![CydMemory framebuffer preview][cyd_memory_bitmap]

#[cfg(test)]
use core::ops::Range;
use core::{
    cell::{Cell, RefCell},
    convert::Infallible,
    future::{Future, ready},
};
use std::{
    fs,
    io::BufWriter,
    path::{Path, PathBuf},
    process,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
    vec::Vec,
};

#[cfg(test)]
use crate::cyd::touch::flow::{MIN_SAMPLES_PER_POINT, SAMPLES_DISCARDED_AFTER_DOWN};
use crate::cyd::{
    Cyd, CydDisplay, CydParts, CydTouch, CydTouchUncalibrated,
    display::{CydFrame, Orientation, RectanglePixels},
    touch::{RawTouchEvent, TouchEvent, calibration::CalibrationConfig},
};
#[cfg(test)]
use crate::flash_block::{
    FlashBlock, FlashBlockError, FlashDevice, clear_block, load_block, save_block,
};
use crate::{
    UnwrapInfallible,
    button::{__ButtonMonitor, Button},
    pixel_target::{PixelTarget, rgb888_from_rgb565},
};
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use embedded_graphics::{
    Drawable, Pixel,
    mono_font::{MonoFont, MonoTextStyle, ascii::FONT_9X15_BOLD},
    pixelcolor::{IntoStorage, Rgb565, raw::RawU16},
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
};
#[cfg(test)]
use serde::{Deserialize, Serialize};

const DEFAULT_FRAME_BUDGET: usize = 1000;
#[cfg(test)]
const FLASH_BLOCK_SIZE: usize = 4096;
#[cfg(test)]
const FLASH_BLOCK_OFFSET: u32 = 0;
#[cfg(test)]
const FLASH_ERASED_BYTE: u8 = 0xFF;

const fn identity_calibration_config() -> CalibrationConfig {
    CalibrationConfig::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
}

#[derive(Clone)]
pub(crate) struct FrameClockMemory {
    frame_index: Rc<Cell<usize>>,
}

impl FrameClockMemory {
    #[must_use]
    pub fn frame_index(&self) -> usize {
        self.frame_index.get()
    }
}

/// Error from the in-memory CYD test surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CydMemoryError {
    OutOfFrames,
}
/// In-memory CYD device for host-side tests and screenshots.
pub struct CydMemory {
    display: CydDisplayMemory,
    touch: CydTouchMemory,
    shared: Rc<RefCell<CydMemoryShared>>,
}

struct CydMemoryShared {
    framebuffer: Vec<u16>,
    flush_count: usize,
    last_flush_rectangle: Option<Rectangle>,
    frame_budget: usize,
    raw_touch_script: FrameScript<RawTouchEvent>,
    touch_script: FrameScript<TouchEvent>,
    frame_clock: FrameClockMemory,
}

/// Owned display half of [`CydMemory`].
#[derive(Clone)]
pub struct CydDisplayMemory {
    size: Size,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
    shared: Rc<RefCell<CydMemoryShared>>,
}

/// Owned calibrated touch half of [`CydMemory`].
#[derive(Clone)]
pub struct CydTouchMemory {
    shared: Rc<RefCell<CydMemoryShared>>,
    calibration_config: CalibrationConfig,
}

/// Owned uncalibrated touch half of [`CydMemory`].
pub struct CydTouchUncalibratedMemory {
    shared: Rc<RefCell<CydMemoryShared>>,
}

/// In-progress in-memory frame that flushes into a host framebuffer.
pub struct CydFrameMemory {
    shared: Rc<RefCell<CydMemoryShared>>,
    screen_size: Size,
    rectangle: Rectangle,
    tile_top_left: Point,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
    pixels: Vec<u16>,
}

struct FrameScript<Event> {
    current_frame: Vec<Event>,
    future_frames: Vec<Vec<Event>>,
    current_read_index: usize,
}

#[cfg(test)]
pub(crate) struct FlashBlockMemory {
    flash_device_memory: FlashDeviceMemory,
    save_count: usize,
}

#[cfg(test)]
struct FlashDeviceMemory {
    bytes: [u8; FLASH_BLOCK_SIZE],
}

/// Host-side button test double returned by [`CydMemory::button_memory`].
pub struct ButtonMemory {
    pressed: bool,
    pressed_frames: Vec<(usize, bool)>,
    frame_clock: Option<FrameClockMemory>,
}

impl CydMemory {
    /// Construct an empty in-memory CYD surface with the given screen style.
    #[must_use]
    pub fn new(
        size: Size,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Self {
        Self::new_inner(size, background, foreground, font)
    }

    /// Construct an in-memory CYD surface with an oriented logical screen.
    #[must_use]
    pub fn new_with_orientation(
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Self {
        Self::new_inner(orientation.size(), background, foreground, font)
    }

    fn new_inner(
        size: Size,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Self {
        let background565 = Rgb565::from(background);
        let pixel_count = size.width as usize * size.height as usize;
        let shared = Rc::new(RefCell::new(CydMemoryShared {
            framebuffer: vec![background565.into_storage(); pixel_count],
            flush_count: 0,
            last_flush_rectangle: None,
            frame_budget: DEFAULT_FRAME_BUDGET,
            raw_touch_script: FrameScript::default(),
            touch_script: FrameScript::default(),
            frame_clock: FrameClockMemory {
                frame_index: Rc::new(Cell::new(0)),
            },
        }));
        let display = CydDisplayMemory {
            size,
            background,
            foreground,
            background565,
            foreground565: Rgb565::from(foreground),
            font,
            shared: shared.clone(),
        };
        let touch = CydTouchMemory {
            shared: shared.clone(),
            calibration_config: identity_calibration_config(),
        };
        Self {
            display,
            touch,
            shared,
        }
    }

    #[must_use]
    pub fn display(&self) -> CydDisplayMemory {
        self.display.clone()
    }

    /// Clone owned calibrated parts that share this harness's backing state.
    #[must_use]
    pub fn owned_parts(&self) -> (CydDisplayMemory, CydTouchMemory) {
        (self.display.clone(), self.touch.clone())
    }

    #[must_use]
    pub fn parts_uncalibrated(&self) -> (CydDisplayMemory, CydTouchUncalibratedMemory) {
        (self.display.clone(), self.touch.clone().decalibrate())
    }
}

impl Cyd for CydMemory {
    type Error = CydMemoryError;
    type Display = CydDisplayMemory;
    type Touch = CydTouchMemory;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }
}

impl CydParts for CydMemory {
    fn into_parts(self) -> (Self::Display, Self::Touch) {
        let Self {
            display,
            touch,
            shared: _shared,
        } = self;
        (display, touch)
    }

    fn from_parts(display: Self::Display, touch: Self::Touch) -> Self {
        Self {
            shared: display.shared.clone(),
            display,
            touch,
        }
    }
}

impl CydMemory {
    /// Limit how many frames may flush before [`CydMemoryError::OutOfFrames`].
    pub fn set_frame_budget(&mut self, frame_budget: usize) {
        self.shared.borrow_mut().frame_budget = frame_budget;
    }

    #[must_use]
    pub(crate) fn frame_clock(&self) -> FrameClockMemory {
        self.shared.borrow().frame_clock.clone()
    }

    /// Create a host-side button tied to this device's frame clock.
    #[must_use]
    pub fn button_memory(&self) -> ButtonMemory {
        ButtonMemory::with_frame_clock(self.frame_clock())
    }

    #[cfg(test)]
    pub(crate) fn script_raw_frames(&mut self, raw_touch_frames: &[&[RawTouchEvent]]) {
        self.shared
            .borrow_mut()
            .raw_touch_script
            .replace_frames(raw_touch_frames);
    }

    #[cfg(test)]
    pub(crate) fn script_raw_frames_owned(&mut self, raw_touch_frames: Vec<Vec<RawTouchEvent>>) {
        self.shared
            .borrow_mut()
            .raw_touch_script
            .replace_owned_frames(raw_touch_frames);
    }

    #[cfg(test)]
    pub(crate) fn push_raw_touch_event(&mut self, raw_touch_event: RawTouchEvent) {
        self.shared
            .borrow_mut()
            .raw_touch_script
            .push_current_frame_event(raw_touch_event);
    }

    /// Queue one calibrated touch event for the current frame.
    pub fn push_touch_event(&mut self, touch_event: TouchEvent) {
        self.shared
            .borrow_mut()
            .touch_script
            .push_current_frame_event(touch_event);
    }

    /// Return how many frames have flushed so far.
    #[must_use]
    pub fn flush_count(&self) -> usize {
        self.shared.borrow().flush_count
    }

    /// Return the rectangle flushed most recently, if any.
    #[must_use]
    pub fn last_flush_rectangle(&self) -> Option<Rectangle> {
        self.shared.borrow().last_flush_rectangle
    }

    /// Read one pixel from the host framebuffer.
    #[must_use]
    pub fn pixel(&self, position_x: usize, position_y: usize) -> Rgb565 {
        assert!(
            position_x < self.display.size.width as usize,
            "position_x must stay within the screen"
        );
        assert!(
            position_y < self.display.size.height as usize,
            "position_y must stay within the screen"
        );
        let stride = self.display.size.width as usize;
        let shared = self.shared.borrow();
        Rgb565::from(RawU16::new(
            shared.framebuffer[position_y * stride + position_x],
        ))
    }

    /// Apply the physical 180-degree presentation used by an inverted CYD orientation.
    ///
    /// Hardware display drivers and browser shells apply this transform outside the logical
    /// application framebuffer. Host previews can call this after rendering to compare the
    /// user-facing presentation rather than the untransformed logical buffer.
    #[cfg(feature = "host")]
    pub fn rotate_framebuffer_180(&self) {
        let mut shared = self.shared.borrow_mut();
        let width = self.display.size.width as usize;
        let height = self.display.size.height as usize;
        for row_index in 0..height / 2 {
            let opposite_row_index = height - 1 - row_index;
            for column_index in 0..width {
                let first_index = row_index * width + column_index;
                let second_index = opposite_row_index * width + (width - 1 - column_index);
                shared.framebuffer.swap(first_index, second_index);
            }
        }
        if height % 2 == 1 {
            let row_start = (height / 2) * width;
            let row_end = row_start + width;
            shared.framebuffer[row_start..row_end].reverse();
        }
    }

    /// Write the framebuffer as an RGB PNG for host-side previews and assertions.
    pub(crate) fn write_framebuffer_png(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.display.size.width;
        let height = self.display.size.height;
        let mut rgb_bytes = Vec::with_capacity(width as usize * height as usize * 3);
        let shared = self.shared.borrow();
        for pixel in &shared.framebuffer {
            let color = rgb888_from_rgb565(*pixel);
            rgb_bytes.push(color.r());
            rgb_bytes.push(color.g());
            rgb_bytes.push(color.b());
        }

        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(path)?;
        let writer = BufWriter::new(file);
        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut png_writer = encoder.write_header()?;
        png_writer.write_image_data(&rgb_bytes)?;
        Ok(())
    }
}

/// Golden-image assertion for a rendered [`CydMemory`] framebuffer.
///
/// `manifest_dir` is normally `env!("CARGO_MANIFEST_DIR")` from the calling
/// crate, so the expected PNG lives at `<crate>/tests/assets/<relative_filename>`.
/// Set `DEVICE_ENVOY_UPDATE_CYD_PNGS=1` to (re)write the expected file
/// instead of comparing against it, e.g. after an intentional visual change.
/// The helper also honors `DEVICE_ENVOY_PREVIEW_OUTPUT_PATH` to copy the
/// freshly rendered PNG to an arbitrary path while still performing the normal
/// golden-image comparison.
pub fn assert_framebuffer_matches_expected_png(
    cyd_memory: &CydMemory,
    manifest_dir: &str,
    relative_filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // The Pages xtask `build-pages` command reuses these exact tests to render gallery
    // preview images: no browser needed, since this crate already renders
    // the real example logic onto a native in-memory framebuffer. Set this
    // env var to also copy the freshly rendered frame out to an arbitrary
    // path, on top of (not instead of) the normal golden-image comparison
    // below, so a preview build still catches a real rendering regression.
    if let Some(preview_output_path) = std::env::var_os("DEVICE_ENVOY_PREVIEW_OUTPUT_PATH") {
        cyd_memory.write_framebuffer_png(preview_output_path)?;
    }

    let mut expected_path = PathBuf::from(manifest_dir);
    expected_path.push("tests");
    expected_path.push("assets");
    expected_path.push(relative_filename);

    if std::env::var_os("DEVICE_ENVOY_UPDATE_CYD_PNGS").is_some() {
        cyd_memory.write_framebuffer_png(&expected_path)?;
        std::println!("updated PNG at {}", expected_path.display());
        return Ok(());
    }

    if !expected_path.exists() {
        return Err(std::format!(
            "expected PNG is missing at {}; rerun with DEVICE_ENVOY_UPDATE_CYD_PNGS=1 to create it",
            expected_path.display()
        )
        .into());
    }

    let unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let temp_path = std::env::temp_dir().join(std::format!(
        "{}-{}-{unix_nanos}",
        relative_filename.replace('/', "_"),
        process::id()
    ));
    cyd_memory.write_framebuffer_png(&temp_path)?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&temp_path)?;
    if let Err(error) = fs::remove_file(&temp_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
    }

    if expected_bytes != actual_bytes {
        return Err(std::format!(
            "PNG bytes differ from {}; rerun with DEVICE_ENVOY_UPDATE_CYD_PNGS=1 to accept the new image",
            expected_path.display()
        )
        .into());
    }
    Ok(())
}

impl Default for CydMemory {
    fn default() -> Self {
        Self::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        )
    }
}

impl core::fmt::Debug for CydMemory {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_struct("CydMemory").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for CydTouchMemory {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CydTouchMemory")
            .field("calibration_config", &self.calibration_config)
            .finish_non_exhaustive()
    }
}

impl core::fmt::Debug for CydTouchUncalibratedMemory {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CydTouchUncalibratedMemory")
            .finish_non_exhaustive()
    }
}

impl CydTouchUncalibrated for CydTouchUncalibratedMemory {
    type Error = CydMemoryError;
    type Calibrated = CydTouchMemory;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
        Ok(self
            .shared
            .borrow_mut()
            .raw_touch_script
            .pop_current_frame_event())
    }

    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated {
        CydTouchMemory {
            shared: self.shared,
            calibration_config,
        }
    }
}

impl CydDisplay for CydDisplayMemory {
    type Error = CydMemoryError;
    type Frame<'a>
        = CydFrameMemory
    where
        Self: 'a;

    fn screen_size(&self) -> Size {
        self.size
    }

    fn background(&self) -> Rgb888 {
        self.background
    }

    fn foreground(&self) -> Rgb888 {
        self.foreground
    }

    fn background_565(&self) -> Rgb565 {
        self.background565
    }

    fn foreground_565(&self) -> Rgb565 {
        self.foreground565
    }

    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> Self::Frame<'_> {
        let pixel_count = rectangle.size.width as usize * rectangle.size.height as usize;
        CydFrameMemory {
            shared: self.shared.clone(),
            screen_size: self.size,
            rectangle,
            tile_top_left,
            background565: self.background565,
            foreground565: self.foreground565,
            font: self.font,
            pixels: vec![self.background565.into_storage(); pixel_count],
        }
    }

    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Self::Error> {
        fill_rectangle_in_framebuffer(
            &mut self.shared.borrow_mut().framebuffer,
            self.size,
            rectangle,
            color.into_storage(),
        );
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        fill_contiguous_in_framebuffer(
            &mut self.shared.borrow_mut().framebuffer,
            self.size,
            rectangle,
            pixels.into_iter().map(IntoStorage::into_storage),
        );
        Ok(())
    }
}

impl CydTouch for CydTouchMemory {
    type Error = CydMemoryError;
    type Uncalibrated = CydTouchUncalibratedMemory;

    fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error> {
        Ok(self
            .shared
            .borrow_mut()
            .touch_script
            .pop_current_frame_event())
    }

    fn calibration_config(&self) -> CalibrationConfig {
        self.calibration_config
    }

    fn decalibrate(self) -> Self::Uncalibrated {
        CydTouchUncalibratedMemory {
            shared: self.shared,
        }
    }
}

impl CydFrameMemory {
    fn width(&self) -> usize {
        self.rectangle.size.width as usize
    }

    fn height(&self) -> usize {
        self.rectangle.size.height as usize
    }

    fn local_x(&self, position_x: i32) -> Option<usize> {
        usize::try_from(position_x.checked_sub(self.tile_top_left.x)?).ok()
    }

    fn local_y(&self, position_y: i32) -> Option<usize> {
        usize::try_from(position_y.checked_sub(self.tile_top_left.y)?).ok()
    }

    fn flush_now(&mut self) -> Result<(), CydMemoryError> {
        let mut shared = self.shared.borrow_mut();
        if shared.flush_count >= shared.frame_budget {
            return Err(CydMemoryError::OutOfFrames);
        }

        blit_frame_to_screen(
            &mut shared.framebuffer,
            self.screen_size,
            self.rectangle,
            &self.pixels,
        );
        shared.last_flush_rectangle = Some(self.rectangle);
        shared.flush_count += 1;
        shared.raw_touch_script.advance_frame();
        shared.touch_script.advance_frame();
        shared
            .frame_clock
            .frame_index
            .set(shared.frame_clock.frame_index.get() + 1);
        Ok(())
    }
}

impl DrawTarget for CydFrameMemory {
    type Color = Rgb565;
    type Error = Infallible;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill(color);
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            let Some(local_x) = self.local_x(point.x) else {
                continue;
            };
            let Some(local_y) = self.local_y(point.y) else {
                continue;
            };
            if local_x >= self.width() || local_y >= self.height() {
                continue;
            }
            let stride = self.width();
            self.pixels[local_y * stride + local_x] = color.into_storage();
        }
        Ok(())
    }
}

impl Dimensions for CydFrameMemory {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.tile_top_left, self.rectangle.size)
    }
}

impl PixelTarget for CydFrameMemory {
    fn width(&self) -> usize {
        usize::try_from(self.tile_top_left.x)
            .expect("tile top-left x must be non-negative")
            .checked_add(self.width())
            .expect("frame width must fit in usize")
    }

    fn height(&self) -> usize {
        usize::try_from(self.tile_top_left.y)
            .expect("tile top-left y must be non-negative")
            .checked_add(self.height())
            .expect("frame height must fit in usize")
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        self.put_pixel_565(x, y, Rgb565::from(color).into_storage());
    }

    fn put_pixel_565(&mut self, x: usize, y: usize, rgb565: u16) {
        let Some(local_x) = self.local_x(x as i32) else {
            return;
        };
        let Some(local_y) = self.local_y(y as i32) else {
            return;
        };
        if local_x >= self.width() || local_y >= self.height() {
            return;
        }
        let stride = self.width();
        self.pixels[local_y * stride + local_x] = rgb565;
    }
}

impl RectanglePixels for CydFrameMemory {
    fn width(&self) -> usize {
        self.width()
    }

    fn height(&self) -> usize {
        self.height()
    }

    fn raw_pixels(&self) -> &[u16] {
        &self.pixels
    }
}

impl CydFrame for CydFrameMemory {
    type Error = CydMemoryError;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        self.pixels.fill(color.into_storage());
        self
    }

    fn clear(&mut self) -> &mut Self {
        self.fill(self.background565)
    }

    fn write_text(&mut self, text: &str) -> &mut Self {
        Text::with_baseline(
            text,
            Point::zero(),
            MonoTextStyle::new(self.font, self.foreground565),
            Baseline::Top,
        )
        .draw(self)
        .unwrap_infallible();
        self
    }

    fn copy_from_565(&mut self, src: &[u16]) -> crate::Result<()> {
        if self.pixels.len() != src.len() {
            return Err(crate::Error::CopySize {
                src_len: src.len(),
                frame_len: self.pixels.len(),
            });
        }
        self.pixels.copy_from_slice(src);
        Ok(())
    }

    fn flush(&mut self) -> impl Future<Output = Result<(), <Self as CydFrame>::Error>> {
        ready(self.flush_now())
    }
}

impl<Event> Default for FrameScript<Event> {
    fn default() -> Self {
        Self {
            current_frame: Vec::new(),
            future_frames: Vec::new(),
            current_read_index: 0,
        }
    }
}

impl<Event: Clone> FrameScript<Event> {
    #[cfg(test)]
    fn replace_frames(&mut self, frames: &[&[Event]]) {
        self.current_frame.clear();
        self.future_frames.clear();
        self.current_read_index = 0;
        if let Some((first_frame, remaining_frames)) = frames.split_first() {
            self.current_frame = first_frame.to_vec();
            self.future_frames = remaining_frames
                .iter()
                .map(|frame| frame.to_vec())
                .collect();
        }
    }

    #[cfg(test)]
    fn replace_owned_frames(&mut self, mut frames: Vec<Vec<Event>>) {
        self.current_frame.clear();
        self.future_frames.clear();
        self.current_read_index = 0;
        if frames.is_empty() {
            return;
        }
        self.current_frame = frames.remove(0);
        self.future_frames = frames;
    }

    fn push_current_frame_event(&mut self, event: Event) {
        self.current_frame.push(event);
    }

    fn pop_current_frame_event(&mut self) -> Option<Event> {
        let event = self.current_frame.get(self.current_read_index).cloned();
        if event.is_some() {
            self.current_read_index += 1;
        }
        event
    }

    fn advance_frame(&mut self) {
        if self.current_read_index >= self.current_frame.len() {
            if let Some(next_frame) = self.future_frames.first().cloned() {
                self.current_frame = next_frame;
                self.future_frames.remove(0);
            } else {
                self.current_frame.clear();
            }
            self.current_read_index = 0;
            return;
        }

        self.current_frame.drain(0..self.current_read_index);
        self.current_read_index = 0;
    }
}

#[cfg(test)]
impl FlashBlockMemory {
    #[must_use]
    pub fn new() -> Self {
        Self {
            flash_device_memory: FlashDeviceMemory::new(),
            save_count: 0,
        }
    }

    #[must_use]
    pub fn with_value<T>(value: &T) -> Self
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let mut flash_block_memory = Self::new();
        flash_block_memory
            .save(value)
            .expect("saving a small in-memory flash value should succeed");
        flash_block_memory
    }

    #[must_use]
    pub fn with_raw_bytes(bytes: &[u8]) -> Self {
        let mut flash_block_memory = Self::new();
        flash_block_memory
            .flash_device_memory
            .write_raw_bytes(bytes);
        flash_block_memory
    }

    #[must_use]
    pub fn save_count(&self) -> usize {
        self.save_count
    }
}

#[cfg(test)]
impl Default for FlashBlockMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl FlashBlock for FlashBlockMemory {
    type Error = FlashBlockError<Infallible>;

    fn load<T>(&mut self) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        match load_block::<FLASH_BLOCK_SIZE, T, _>(
            &mut self.flash_device_memory,
            FLASH_BLOCK_OFFSET,
        ) {
            Ok(value) => Ok(value),
            Err(FlashBlockError::StorageCorrupted | FlashBlockError::FormatError) => Ok(None),
            Err(FlashBlockError::Io(infallible)) => match infallible {},
        }
    }

    fn save<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        save_block::<FLASH_BLOCK_SIZE, _, _>(
            &mut self.flash_device_memory,
            FLASH_BLOCK_OFFSET,
            value,
        )?;
        self.save_count += 1;
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        clear_block::<FLASH_BLOCK_SIZE, _>(&mut self.flash_device_memory, FLASH_BLOCK_OFFSET)
    }
}

#[cfg(test)]
impl FlashDeviceMemory {
    // TODO Consider consolidating this host-side flash test double with the
    // test-private `FlashDeviceMemory` in device-envoy-core's flash_block.rs tests.
    fn new() -> Self {
        Self {
            bytes: [FLASH_ERASED_BYTE; FLASH_BLOCK_SIZE],
        }
    }

    fn checked_range(&self, offset: u32, len: usize) -> Range<usize> {
        let start = usize::try_from(offset).expect("flash offset must fit in usize");
        let end = start
            .checked_add(len)
            .expect("flash range must fit in usize");
        assert!(
            end <= FLASH_BLOCK_SIZE,
            "flash range must stay in the block"
        );
        start..end
    }

    fn write_raw_bytes(&mut self, bytes: &[u8]) {
        self.bytes.fill(FLASH_ERASED_BYTE);
        let len = bytes.len().min(FLASH_BLOCK_SIZE);
        self.bytes[..len].copy_from_slice(&bytes[..len]);
    }
}

#[cfg(test)]
impl FlashDevice for FlashDeviceMemory {
    type Error = Infallible;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let checked_range = self.checked_range(offset, bytes.len());
        bytes.copy_from_slice(&self.bytes[checked_range]);
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let checked_range = self.checked_range(offset, bytes.len());
        self.bytes[checked_range].copy_from_slice(bytes);
        Ok(())
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let len = usize::try_from(to.saturating_sub(from)).expect("flash erase length fits usize");
        let checked_range = self.checked_range(from, len);
        self.bytes[checked_range].fill(FLASH_ERASED_BYTE);
        Ok(())
    }
}

impl ButtonMemory {
    /// Construct a button test double with no frame scheduling.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pressed: false,
            pressed_frames: Vec::new(),
            frame_clock: None,
        }
    }

    #[must_use]
    pub(crate) fn with_frame_clock(frame_clock: FrameClockMemory) -> Self {
        Self {
            pressed: false,
            pressed_frames: Vec::new(),
            frame_clock: Some(frame_clock),
        }
    }

    /// Set the button's default pressed state.
    pub fn set_pressed(&mut self, pressed: bool) {
        self.pressed = pressed;
    }

    /// Override the pressed state for one specific flushed frame index.
    pub fn set_pressed_for_frame(&mut self, frame_index: usize, pressed: bool) {
        if let Some(existing_state) = self
            .pressed_frames
            .iter_mut()
            .find(|(existing_frame_index, _pressed_state)| *existing_frame_index == frame_index)
        {
            existing_state.1 = pressed;
            return;
        }
        self.pressed_frames.push((frame_index, pressed));
    }

    fn current_pressed_state(&self) -> bool {
        let Some(frame_clock) = &self.frame_clock else {
            return self.pressed;
        };
        let frame_index = frame_clock.frame_index();
        self.pressed_frames
            .iter()
            .find_map(|(pressed_frame_index, pressed)| {
                (*pressed_frame_index == frame_index).then_some(*pressed)
            })
            .unwrap_or(self.pressed)
    }
}

impl Default for ButtonMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl __ButtonMonitor for ButtonMemory {
    fn is_pressed_raw(&self) -> bool {
        self.current_pressed_state()
    }

    async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
}

impl Button for ButtonMemory {}

fn fill_rectangle_in_framebuffer(
    framebuffer: &mut [u16],
    screen_size: Size,
    rectangle: Rectangle,
    color: u16,
) {
    let clipped_rectangle = rectangle.intersection(&Rectangle::new(Point::zero(), screen_size));
    if clipped_rectangle.size.width == 0 || clipped_rectangle.size.height == 0 {
        return;
    }
    let stride = screen_size.width as usize;
    for position_y in clipped_rectangle.top_left.y
        ..clipped_rectangle.top_left.y + clipped_rectangle.size.height as i32
    {
        for position_x in clipped_rectangle.top_left.x
            ..clipped_rectangle.top_left.x + clipped_rectangle.size.width as i32
        {
            let index = position_y as usize * stride + position_x as usize;
            framebuffer[index] = color;
        }
    }
}

fn fill_contiguous_in_framebuffer<I>(
    framebuffer: &mut [u16],
    screen_size: Size,
    rectangle: Rectangle,
    pixels: I,
) where
    I: IntoIterator<Item = u16>,
{
    if rectangle.size.width == 0 || rectangle.size.height == 0 {
        return;
    }
    let stride = screen_size.width as usize;
    for (pixel_index, pixel) in pixels.into_iter().enumerate() {
        let local_x = pixel_index % rectangle.size.width as usize;
        let local_y = pixel_index / rectangle.size.width as usize;
        if local_y >= rectangle.size.height as usize {
            break;
        }
        let position_x = rectangle.top_left.x + local_x as i32;
        let position_y = rectangle.top_left.y + local_y as i32;
        if position_x < 0
            || position_y < 0
            || position_x >= screen_size.width as i32
            || position_y >= screen_size.height as i32
        {
            continue;
        }
        framebuffer[position_y as usize * stride + position_x as usize] = pixel;
    }
}

fn blit_frame_to_screen(
    framebuffer: &mut [u16],
    screen_size: Size,
    rectangle: Rectangle,
    pixels: &[u16],
) {
    fill_contiguous_in_framebuffer(framebuffer, screen_size, rectangle, pixels.iter().copied());
}

#[cfg(test)]
mod tests {
    use super::{
        ButtonMemory, CydMemory, CydMemoryError, CydTouchMemory, CydTouchUncalibratedMemory,
        FlashBlockMemory,
    };
    use crate::cyd::CydParts;
    use crate::cyd::touch::driver::{
        CAPTURE_ACK_FRAME_COUNT, MAX_RAW_EVENTS_PER_FRAME, REJECTED_FRAME_COUNT,
        VERIFY_TIMEOUT_FRAMES,
    };
    use crate::cyd::{
        Cyd, CydDisplay, CydTouch, CydTouchUncalibrated,
        display::{CydFrame, RectanglePixels},
        touch::{
            RawPoint, RawTouchEvent, TouchEvent,
            calibration::{
                CalibrationConfig, CalibrationCorner, EnsureCalibrationErrorKind,
                EnsureCalibrationOutcome, VERIFY_HIT_RADIUS_PIXELS, calibration_corner_center,
                calibration_verify_target_center, distort_demo_screen_to_raw, ensure_calibration,
            },
        },
    };
    use crate::flash_block::FlashBlock;
    use embedded_graphics::{
        Pixel,
        mono_font::ascii::FONT_9X15_BOLD,
        pixelcolor::{IntoStorage, Rgb565, Rgb888, WebColors},
        prelude::{DrawTarget, Point, Size},
        primitives::Rectangle,
    };
    use futures_executor::block_on;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct DemoValue {
        count: u16,
    }

    fn test_cyd_memory() -> CydMemory {
        CydMemory::new(
            Size::new(320, 240),
            Rgb888::CSS_BLACK,
            Rgb888::CSS_WHITE,
            &FONT_9X15_BOLD,
        )
    }

    fn read_next_raw_touch_event(
        memory_cyd: &CydMemory,
    ) -> Result<Option<RawTouchEvent>, CydMemoryError> {
        let (_display, mut touch) = memory_cyd.parts_uncalibrated();
        touch.read_raw_touch_event()
    }

    fn run_ensure_calibration(
        memory_cyd: &CydMemory,
        memory_flash_block: &mut FlashBlockMemory,
        memory_button: &mut ButtonMemory,
        confirmed_message: Option<&str>,
    ) -> Result<
        (CydTouchMemory, EnsureCalibrationOutcome),
        crate::cyd::touch::calibration::EnsureCalibrationError<
            CydTouchUncalibratedMemory,
            <FlashBlockMemory as FlashBlock>::Error,
        >,
    > {
        let (mut display, touch) = memory_cyd.parts_uncalibrated();
        block_on(ensure_calibration(
            &mut display,
            touch,
            memory_flash_block,
            memory_button,
            confirmed_message,
        ))
    }

    #[test]
    fn fresh_frame_starts_cleared_to_background() {
        let memory_cyd = test_cyd_memory();
        let mut display = memory_cyd.display();
        let frame = display.frame_mut(Rectangle::new(Point::new(3, 4), Size::new(2, 2)));
        assert_eq!(frame.raw_pixels(), &[Rgb565::CSS_BLACK.into_storage(); 4]);
    }

    #[test]
    fn draw_target_pixel_flushes_to_screen_coordinate() {
        let memory_cyd = test_cyd_memory();
        {
            let mut display = memory_cyd.display();
            let mut frame = display.frame_mut_with_tile_top_left(
                Rectangle::new(Point::new(10, 20), Size::new(4, 3)),
                Point::new(10, 20),
            );
            frame
                .draw_iter([Pixel(Point::new(11, 21), Rgb565::CSS_RED)])
                .expect("drawing into memory frame should succeed");
            block_on(frame.flush()).expect("flush should succeed");
        }
        assert_eq!(memory_cyd.pixel(11, 21), Rgb565::CSS_RED);
        assert_eq!(
            memory_cyd.last_flush_rectangle(),
            Some(Rectangle::new(Point::new(10, 20), Size::new(4, 3)))
        );
    }

    #[test]
    fn fill_rectangle_clips_to_screen_edges() {
        let memory_cyd = CydMemory::new(
            Size::new(4, 4),
            Rgb888::CSS_BLACK,
            Rgb888::CSS_WHITE,
            &FONT_9X15_BOLD,
        );
        {
            let mut display = memory_cyd.display();
            display
                .fill_rectangle(
                    Rectangle::new(Point::new(-1, -1), Size::new(3, 3)),
                    Rgb565::CSS_GREEN,
                )
                .expect("fill_rectangle should succeed");
            display
                .fill_rectangle(
                    Rectangle::new(Point::new(10, 10), Size::new(2, 2)),
                    Rgb565::CSS_RED,
                )
                .expect("off-screen fill_rectangle should stay a no-op");
        }
        assert_eq!(memory_cyd.pixel(0, 0), Rgb565::CSS_GREEN);
        assert_eq!(memory_cyd.pixel(1, 1), Rgb565::CSS_GREEN);
        assert_eq!(memory_cyd.pixel(3, 3), Rgb565::CSS_BLACK);
    }

    #[test]
    fn cyd_into_from_parts_decalibrates_and_recalibrates() {
        let cyd = test_cyd_memory();
        let saved_config = CalibrationConfig::new(1.0, 0.0, 2.0, 0.0, 1.0, 3.0);
        let mut memory_flash_block = FlashBlockMemory::with_value(&saved_config);
        let mut memory_button = cyd.button_memory();

        let (mut display, touch) = cyd.into_parts();
        let touch_uncalibrated = touch.decalibrate();

        let (touch, ensure_calibration_outcome) = block_on(ensure_calibration(
            &mut display,
            touch_uncalibrated,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        ))
        .expect("preloaded calibration should load");
        assert!(matches!(
            ensure_calibration_outcome,
            EnsureCalibrationOutcome::Loaded(_)
        ));
        assert_eq!(touch.calibration_config(), saved_config);

        let mut cyd = CydMemory::from_parts(display, touch);
        cyd.push_touch_event(TouchEvent::Down {
            point: Point::new(12, 34),
        });
        {
            let (display, touch) = cyd.parts();
            assert!(matches!(
                touch.read().expect("touch read should succeed"),
                Some(TouchEvent::Down { .. })
            ));
            let mut frame = display.full_frame_mut();
            block_on(frame.flush()).expect("flush should succeed");
        }
        assert_eq!(cyd.flush_count(), 1);
    }

    #[test]
    fn raw_touch_frames_drain_then_advance_after_flush() {
        let mut memory_cyd = test_cyd_memory();
        let first_frame = [
            RawTouchEvent::Down { raw_x: 1, raw_y: 2 },
            RawTouchEvent::Up,
        ];
        let second_frame = [RawTouchEvent::Down { raw_x: 3, raw_y: 4 }];
        memory_cyd.script_raw_frames(&[&first_frame, &second_frame]);

        assert_eq!(
            read_next_raw_touch_event(&memory_cyd).expect("read should succeed"),
            Some(RawTouchEvent::Down { raw_x: 1, raw_y: 2 })
        );
        assert_eq!(
            read_next_raw_touch_event(&memory_cyd).expect("read should succeed"),
            Some(RawTouchEvent::Up)
        );
        assert_eq!(
            read_next_raw_touch_event(&memory_cyd).expect("read should succeed"),
            None
        );

        {
            let mut display = memory_cyd.display();
            let mut frame = display.full_frame_mut();
            block_on(frame.flush()).expect("flush should succeed");
        }

        assert_eq!(memory_cyd.flush_count(), 1);
        assert_eq!(
            read_next_raw_touch_event(&memory_cyd).expect("read should succeed"),
            Some(RawTouchEvent::Down { raw_x: 3, raw_y: 4 })
        );
    }

    #[test]
    fn flush_budget_returns_out_of_frames() {
        let mut memory_cyd = test_cyd_memory();
        memory_cyd.set_frame_budget(1);
        {
            let mut display = memory_cyd.display();
            let mut frame = display.full_frame_mut();
            block_on(frame.flush()).expect("first flush should succeed");
        }
        {
            let mut display = memory_cyd.display();
            let mut frame = display.full_frame_mut();
            let error = block_on(frame.flush()).expect_err("second flush should hit frame budget");
            assert_eq!(error, CydMemoryError::OutOfFrames);
        }
        assert_eq!(memory_cyd.flush_count(), 1);
    }

    #[test]
    fn memory_flash_block_round_trips_and_handles_corruption() {
        let mut memory_flash_block = FlashBlockMemory::new();
        memory_flash_block
            .save(&DemoValue { count: 7 })
            .expect("save should succeed");
        assert_eq!(
            memory_flash_block
                .load::<DemoValue>()
                .expect("load should succeed"),
            Some(DemoValue { count: 7 })
        );

        let mut corrupt_flash_block = FlashBlockMemory::with_raw_bytes(&[1, 2, 3, 4]);
        assert_eq!(
            corrupt_flash_block
                .load::<DemoValue>()
                .expect("corrupt load should degrade to None"),
            None
        );

        memory_flash_block.clear().expect("clear should succeed");
        assert_eq!(
            memory_flash_block
                .load::<DemoValue>()
                .expect("load should succeed"),
            None
        );
    }

    #[test]
    fn ensure_calibration_happy_path_saves_predictable_config() {
        let mut memory_cyd = test_cyd_memory();
        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        let raw_points = script_happy_path(&mut memory_cyd);

        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            Some("saved"),
        )
        .expect("happy-path calibration should succeed");

        let EnsureCalibrationOutcome::Saved(calibration_config) = outcome else {
            panic!("happy-path calibration should save a new config");
        };
        assert_eq!(memory_flash_block.save_count(), 1);

        let saved_config = memory_flash_block
            .load::<CalibrationConfig>()
            .expect("saved config should deserialize")
            .expect("saved config should exist");
        assert_eq!(saved_config, calibration_config);

        for (raw_point, calibration_corner) in raw_points.into_iter().zip([
            CalibrationCorner::UpperLeft,
            CalibrationCorner::UpperRight,
            CalibrationCorner::LowerRight,
            CalibrationCorner::LowerLeft,
        ]) {
            let expected_screen_point = calibration_corner_center(calibration_corner);
            let (mapped_x, mapped_y) = saved_config.map_raw_to_screen(raw_point.x, raw_point.y);
            assert!(
                (mapped_x - expected_screen_point.x as f32).abs() <= 1.0,
                "mapped_x={mapped_x} expected_x={}",
                expected_screen_point.x
            );
            assert!(
                (mapped_y - expected_screen_point.y as f32).abs() <= 1.0,
                "mapped_y={mapped_y} expected_y={}",
                expected_screen_point.y
            );
        }
        assert!(memory_cyd.flush_count() > 0);
        // The confirmation message is the only buffered flush per redraw;
        // target/dot geometry streams buffer-free via `draw_items` and
        // doesn't touch `last_flush_rectangle`. See `CALIBRATION_TEXT_RECTANGLE`.
        assert_eq!(
            memory_cyd.last_flush_rectangle(),
            Some(Rectangle::new(Point::new(0, 220), Size::new(320, 20)))
        );
    }

    #[test]
    fn ensure_calibration_uses_preloaded_flash_without_flushing() {
        let mut memory_cyd = test_cyd_memory();
        let saved_config = CalibrationConfig::new(1.0, 0.0, 2.0, 0.0, 1.0, 3.0);
        memory_cyd.push_raw_touch_event(RawTouchEvent::Down { raw_x: 7, raw_y: 9 });
        let mut memory_flash_block = FlashBlockMemory::with_value(&saved_config);
        let mut memory_button = memory_cyd.button_memory();

        let (touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("preloaded calibration should load");

        let EnsureCalibrationOutcome::Loaded(loaded_config) = outcome else {
            panic!("preloaded flash should skip the calibration flow");
        };
        assert_eq!(loaded_config, saved_config);
        assert_eq!(memory_cyd.flush_count(), 0);
        let mut touch = touch.decalibrate();
        assert_eq!(
            touch
                .read_raw_touch_event()
                .expect("touch read should succeed"),
            Some(RawTouchEvent::Down { raw_x: 7, raw_y: 9 })
        );
    }

    #[test]
    fn ensure_calibration_corrupt_flash_reruns_and_overwrites() {
        let mut memory_cyd = test_cyd_memory();
        let mut memory_flash_block = FlashBlockMemory::with_raw_bytes(&[1, 2, 3, 4]);
        let mut memory_button = memory_cyd.button_memory();
        script_happy_path(&mut memory_cyd);

        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("corrupt flash should fall back to calibration");

        assert!(matches!(outcome, EnsureCalibrationOutcome::Saved(_)));
        assert_eq!(memory_flash_block.save_count(), 1);
        assert!(
            memory_flash_block
                .load::<CalibrationConfig>()
                .expect("load should succeed")
                .is_some()
        );
    }

    #[test]
    fn ensure_calibration_paces_with_one_flush_per_iteration() {
        let mut memory_cyd = test_cyd_memory();
        memory_cyd.set_frame_budget(3);
        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();

        let error = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect_err("empty input should stop at the frame budget");

        assert!(matches!(
            error.kind,
            EnsureCalibrationErrorKind::Device(CydMemoryError::OutOfFrames)
        ));
        assert_eq!(memory_cyd.flush_count(), 3);
    }

    #[test]
    fn ensure_calibration_drains_a_full_tap_in_one_frame() {
        let mut memory_cyd = test_cyd_memory();
        memory_cyd.set_frame_budget(1);
        let upper_left_raw_point = raw_point_for_corner(CalibrationCorner::UpperLeft);
        memory_cyd.script_raw_frames_owned(vec![tap_events(upper_left_raw_point)]);
        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();

        let error = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect_err("single-frame budget should stop after the first drawn frame");

        assert!(matches!(
            error.kind,
            EnsureCalibrationErrorKind::Device(CydMemoryError::OutOfFrames)
        ));
        let upper_left_center = calibration_corner_center(CalibrationCorner::UpperLeft);
        let upper_right_center = calibration_corner_center(CalibrationCorner::UpperRight);
        assert_eq!(
            memory_cyd.pixel(upper_left_center.x as usize, upper_left_center.y as usize),
            Rgb565::CSS_WHITE
        );
        assert_eq!(
            memory_cyd.pixel(upper_right_center.x as usize, upper_right_center.y as usize),
            Rgb565::CSS_WHITE
        );
        assert_eq!(memory_cyd.pixel(160, 120), Rgb565::CSS_BLACK);
    }

    #[test]
    fn ensure_calibration_verify_timeout_restarts_and_then_succeeds() {
        let mut memory_cyd = test_cyd_memory();
        let mut frames = happy_path_frames();
        frames.truncate(frames.len() - 1);
        frames.extend((0..verify_timeout_extra_idle_frames()).map(|_| Vec::new()));
        frames.extend((0..rejected_restart_idle_frames()).map(|_| Vec::new()));
        frames.extend(happy_path_frames());
        memory_cyd.script_raw_frames_owned(frames);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();

        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("flow should restart after verify timeout and then save");

        assert!(matches!(outcome, EnsureCalibrationOutcome::Saved(_)));
        assert_eq!(memory_flash_block.save_count(), 1);
    }

    #[test]
    fn ensure_calibration_dropout_does_not_leak_corner_two_into_corner_three() {
        let mut memory_cyd = test_cyd_memory();
        let upper_left_raw_point = raw_point_for_corner(CalibrationCorner::UpperLeft);
        let upper_right_raw_point = raw_point_for_corner(CalibrationCorner::UpperRight);
        let lower_right_raw_point = raw_point_for_corner(CalibrationCorner::LowerRight);
        let lower_left_raw_point = raw_point_for_corner(CalibrationCorner::LowerLeft);
        let verify_raw_point = raw_point_for_verify_target();
        let mut frames = vec![tap_events(upper_left_raw_point)];
        append_idle_frames(&mut frames, capture_ack_extra_idle_frames());
        frames.push(dropout_tap_events(upper_right_raw_point));
        append_idle_frames(&mut frames, capture_ack_extra_idle_frames());
        frames.push(tap_events(lower_right_raw_point));
        append_idle_frames(&mut frames, capture_ack_extra_idle_frames());
        frames.push(tap_events(lower_left_raw_point));
        frames.push(tap_events(verify_raw_point));
        memory_cyd.script_raw_frames_owned(frames);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("dropout sequence should still save a calibration");

        let EnsureCalibrationOutcome::Saved(calibration_config) = outcome else {
            panic!("dropout sequence should save a calibration");
        };
        assert_maps_near_corner(
            calibration_config,
            lower_right_raw_point,
            CalibrationCorner::LowerRight,
        );
    }

    #[test]
    fn ensure_calibration_lift_off_drift_keeps_captured_point_near_stable_raw_point() {
        let mut memory_cyd = test_cyd_memory();
        let upper_left_raw_point = raw_point_for_corner(CalibrationCorner::UpperLeft);
        let drifted_raw_point = RawPoint {
            x: upper_left_raw_point.x + 400,
            y: upper_left_raw_point.y + 400,
        };
        let upper_right_raw_point = raw_point_for_corner(CalibrationCorner::UpperRight);
        let lower_right_raw_point = raw_point_for_corner(CalibrationCorner::LowerRight);
        let lower_left_raw_point = raw_point_for_corner(CalibrationCorner::LowerLeft);
        let verify_raw_point = raw_point_for_verify_target();
        let mut frames = vec![long_press_with_lift_off_drift_frame(
            upper_left_raw_point,
            drifted_raw_point,
        )];
        append_idle_frames(&mut frames, capture_ack_extra_idle_frames());
        frames.extend(calibration_attempt_frames(&[
            upper_right_raw_point,
            lower_right_raw_point,
            lower_left_raw_point,
            verify_raw_point,
        ]));
        memory_cyd.script_raw_frames_owned(frames);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("lift-off drift sequence should still save a calibration");

        let EnsureCalibrationOutcome::Saved(calibration_config) = outcome else {
            panic!("lift-off drift sequence should save a calibration");
        };
        assert_maps_near_corner(
            calibration_config,
            upper_left_raw_point,
            CalibrationCorner::UpperLeft,
        );
    }

    #[test]
    fn ensure_calibration_rejected_solve_restarts_and_then_saves_honest_script() {
        let mut memory_cyd = test_cyd_memory();
        let upper_left_raw_point = raw_point_for_corner(CalibrationCorner::UpperLeft);
        let lower_right_raw_point = raw_point_for_corner(CalibrationCorner::LowerRight);
        let lower_left_raw_point = raw_point_for_corner(CalibrationCorner::LowerLeft);
        let mut frames = calibration_attempt_frames(&[
            upper_left_raw_point,
            upper_left_raw_point,
            lower_right_raw_point,
            lower_left_raw_point,
            raw_point_for_verify_target(),
        ]);
        append_idle_frames(&mut frames, rejected_restart_idle_frames());
        frames.extend(happy_path_frames());
        memory_cyd.script_raw_frames_owned(frames);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("rejected solve should restart and then save");

        let EnsureCalibrationOutcome::Saved(calibration_config) = outcome else {
            panic!("rejected solve should eventually save");
        };
        assert_eq!(memory_flash_block.save_count(), 1);
        assert_maps_near_corner(
            calibration_config,
            raw_point_for_corner(CalibrationCorner::UpperRight),
            CalibrationCorner::UpperRight,
        );
    }

    #[test]
    fn ensure_calibration_verify_miss_restarts_without_saving_candidate() {
        let mut memory_cyd = test_cyd_memory();
        let verify_target_center = calibration_verify_target_center();
        let verify_miss_screen_x =
            verify_target_center.x + VERIFY_HIT_RADIUS_PIXELS.ceil() as i32 + 10;
        let verify_miss_raw_point =
            distort_demo_screen_to_raw(verify_miss_screen_x as f32, verify_target_center.y as f32);
        let mut frames = calibration_attempt_frames(&[
            raw_point_for_corner(CalibrationCorner::UpperLeft),
            raw_point_for_corner(CalibrationCorner::UpperRight),
            raw_point_for_corner(CalibrationCorner::LowerRight),
            raw_point_for_corner(CalibrationCorner::LowerLeft),
            verify_miss_raw_point,
        ]);
        append_idle_frames(&mut frames, rejected_restart_idle_frames());
        frames.extend(happy_path_frames());
        memory_cyd.script_raw_frames_owned(frames);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("verify miss should restart and then save");

        assert!(matches!(outcome, EnsureCalibrationOutcome::Saved(_)));
        assert_eq!(memory_flash_block.save_count(), 1);
    }

    #[test]
    fn ensure_calibration_recalibration_button_restarts_mid_flow() {
        let mut memory_cyd = test_cyd_memory();
        let mut frames = vec![tap_events(raw_point_for_corner(
            CalibrationCorner::UpperLeft,
        ))];
        append_idle_frames(&mut frames, 2);
        frames.extend(happy_path_frames());
        memory_cyd.script_raw_frames_owned(frames);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        memory_button.set_pressed_for_frame(2, true);
        let (_touch, outcome) = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect("button-triggered recalibration should restart and then save");

        let EnsureCalibrationOutcome::Saved(calibration_config) = outcome else {
            panic!("button-triggered recalibration should save");
        };
        assert_eq!(memory_flash_block.save_count(), 1);
        assert_maps_near_corner(
            calibration_config,
            raw_point_for_corner(CalibrationCorner::UpperLeft),
            CalibrationCorner::UpperLeft,
        );
    }

    #[test]
    fn ensure_calibration_drain_cap_flushes_and_preserves_leftovers_during_hold() {
        let mut memory_cyd = test_cyd_memory();
        memory_cyd.set_frame_budget(2);
        let upper_left_raw_point = raw_point_for_corner(CalibrationCorner::UpperLeft);
        let mut oversized_hold_frame = Vec::new();
        oversized_hold_frame.push(RawTouchEvent::Down {
            raw_x: upper_left_raw_point.x,
            raw_y: upper_left_raw_point.y,
        });
        for _raw_event_index in 0..MAX_RAW_EVENTS_PER_FRAME.saturating_sub(1) {
            oversized_hold_frame.push(RawTouchEvent::Move {
                raw_x: upper_left_raw_point.x,
                raw_y: upper_left_raw_point.y,
            });
        }
        oversized_hold_frame.push(RawTouchEvent::Up);
        memory_cyd.script_raw_frames_owned(vec![oversized_hold_frame]);

        let mut memory_flash_block = FlashBlockMemory::new();
        let mut memory_button = memory_cyd.button_memory();
        let error = run_ensure_calibration(
            &memory_cyd,
            &mut memory_flash_block,
            &mut memory_button,
            None,
        )
        .expect_err("oversized hold should stop at the frame budget");

        assert!(matches!(
            error.kind,
            EnsureCalibrationErrorKind::Device(CydMemoryError::OutOfFrames)
        ));
        assert_eq!(memory_cyd.flush_count(), 2);
        let upper_left_center = calibration_corner_center(CalibrationCorner::UpperLeft);
        let upper_right_center = calibration_corner_center(CalibrationCorner::UpperRight);
        assert_eq!(
            memory_cyd.pixel(upper_left_center.x as usize, upper_left_center.y as usize),
            Rgb565::CSS_WHITE
        );
        assert_eq!(
            memory_cyd.pixel(upper_right_center.x as usize, upper_right_center.y as usize),
            Rgb565::CSS_WHITE
        );
        assert_eq!(
            read_next_raw_touch_event(&memory_cyd)
                .expect("the oversized frame should be fully drained by the second iteration"),
            None
        );
    }

    fn script_happy_path(memory_cyd: &mut CydMemory) -> [RawPoint; 4] {
        memory_cyd.script_raw_frames_owned(happy_path_frames());
        [
            raw_point_for_corner(CalibrationCorner::UpperLeft),
            raw_point_for_corner(CalibrationCorner::UpperRight),
            raw_point_for_corner(CalibrationCorner::LowerRight),
            raw_point_for_corner(CalibrationCorner::LowerLeft),
        ]
    }

    fn happy_path_frames() -> Vec<Vec<RawTouchEvent>> {
        calibration_attempt_frames(&[
            raw_point_for_corner(CalibrationCorner::UpperLeft),
            raw_point_for_corner(CalibrationCorner::UpperRight),
            raw_point_for_corner(CalibrationCorner::LowerRight),
            raw_point_for_corner(CalibrationCorner::LowerLeft),
            raw_point_for_verify_target(),
        ])
    }

    fn raw_point_for_corner(calibration_corner: CalibrationCorner) -> RawPoint {
        let screen_point = calibration_corner_center(calibration_corner);
        distort_demo_screen_to_raw(screen_point.x as f32, screen_point.y as f32)
    }

    fn tap_events(raw_point: RawPoint) -> Vec<RawTouchEvent> {
        super::tap_events(raw_point)
    }

    fn dropout_tap_events(raw_point: RawPoint) -> Vec<RawTouchEvent> {
        let mut raw_touch_events = tap_events(raw_point);
        raw_touch_events.extend([
            RawTouchEvent::Down {
                raw_x: raw_point.x,
                raw_y: raw_point.y,
            },
            RawTouchEvent::Move {
                raw_x: raw_point.x,
                raw_y: raw_point.y,
            },
            RawTouchEvent::Up,
        ]);
        raw_touch_events
    }

    fn long_press_with_lift_off_drift_frame(
        stable_raw_point: RawPoint,
        drifted_raw_point: RawPoint,
    ) -> Vec<RawTouchEvent> {
        let mut raw_touch_events = Vec::new();
        raw_touch_events.push(RawTouchEvent::Down {
            raw_x: stable_raw_point.x,
            raw_y: stable_raw_point.y,
        });
        for _stable_move_index in 0..2_004 {
            raw_touch_events.push(RawTouchEvent::Move {
                raw_x: stable_raw_point.x,
                raw_y: stable_raw_point.y,
            });
        }
        for _drifted_move_index in 0..3 {
            raw_touch_events.push(RawTouchEvent::Move {
                raw_x: drifted_raw_point.x,
                raw_y: drifted_raw_point.y,
            });
        }
        raw_touch_events.push(RawTouchEvent::Up);
        raw_touch_events
    }

    fn calibration_attempt_frames(raw_points: &[RawPoint]) -> Vec<Vec<RawTouchEvent>> {
        let mut frames = Vec::new();
        for (tap_index, raw_point) in raw_points.iter().copied().enumerate() {
            frames.push(tap_events(raw_point));
            if tap_index + 2 < raw_points.len() {
                append_idle_frames(&mut frames, capture_ack_extra_idle_frames());
            }
        }
        frames
    }

    fn append_idle_frames(frames: &mut Vec<Vec<RawTouchEvent>>, idle_frame_count: usize) {
        frames.extend((0..idle_frame_count).map(|_| Vec::new()));
    }

    fn raw_point_for_verify_target() -> RawPoint {
        let verify_center = calibration_verify_target_center();
        distort_demo_screen_to_raw(verify_center.x as f32, verify_center.y as f32)
    }

    fn assert_maps_near_corner(
        calibration_config: CalibrationConfig,
        raw_point: RawPoint,
        calibration_corner: CalibrationCorner,
    ) {
        let expected_screen_point = calibration_corner_center(calibration_corner);
        let (mapped_x, mapped_y) = calibration_config.map_raw_to_screen(raw_point.x, raw_point.y);
        assert!(
            (mapped_x - expected_screen_point.x as f32).abs() <= 1.0,
            "mapped_x={mapped_x} expected_x={}",
            expected_screen_point.x
        );
        assert!(
            (mapped_y - expected_screen_point.y as f32).abs() <= 1.0,
            "mapped_y={mapped_y} expected_y={}",
            expected_screen_point.y
        );
    }

    const fn capture_ack_extra_idle_frames() -> usize {
        // The tap frame may end with an immediate `None`, but that idle pass only
        // decrements the freshly-entered `ShowCaptured` state before drawing its
        // first acknowledgment screen. Tests still need a full
        // `CAPTURE_ACK_FRAME_COUNT` later idle frames before the next scripted tap
        // is guaranteed to run after the ack window.
        CAPTURE_ACK_FRAME_COUNT
    }

    const fn rejected_restart_idle_frames() -> usize {
        REJECTED_FRAME_COUNT
    }

    const fn verify_timeout_extra_idle_frames() -> usize {
        VERIFY_TIMEOUT_FRAMES.saturating_sub(1)
    }
}

#[cfg(test)]
fn tap_events(raw_point: crate::cyd::touch::RawPoint) -> Vec<RawTouchEvent> {
    let mut raw_touch_events = Vec::new();
    raw_touch_events.push(RawTouchEvent::Down {
        raw_x: raw_point.x,
        raw_y: raw_point.y,
    });
    for _discarded_sample_index in 0..SAMPLES_DISCARDED_AFTER_DOWN {
        raw_touch_events.push(RawTouchEvent::Move {
            raw_x: raw_point.x,
            raw_y: raw_point.y,
        });
    }
    for _usable_sample_index in 0..MIN_SAMPLES_PER_POINT {
        raw_touch_events.push(RawTouchEvent::Move {
            raw_x: raw_point.x,
            raw_y: raw_point.y,
        });
    }
    raw_touch_events.push(RawTouchEvent::Up);
    raw_touch_events
}
