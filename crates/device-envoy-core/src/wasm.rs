//! Browser-simulated CYD parts plus [`Button`] and [`FlashBlock`] implementations.
//!
//! Requires the `wasm` feature. [`CydWasm`] offers owned CYD display/touch
//! parts against an HTML canvas, so the same generic example code that drives
//! the real esp32 `CydEsp` also runs in a web page. Its [`CydFrameWasm::flush`]
//! awaits the next browser animation frame (see [`next_animation_frame`]),
//! blits the frame to the canvas, then resolves.

mod animation_frame;
pub mod clock;
pub mod cyd_web;
pub mod dns;
pub mod simulator;

use core::{
    cell::{Cell, RefCell},
    convert::Infallible,
    ops::Range,
};
use std::{collections::VecDeque, rc::Rc};

use crate::cyd::{
    Cyd, CydDisplay, CydTouch,
    backend::{CalibrationConfig, RawTouchEvent},
    display::{CydFrame, Orientation},
    touch::{RawPoint, TouchEvent},
};
use crate::{
    button::{__ButtonMonitor, BUTTON_POLL_INTERVAL, Button},
    flash_block::{
        Error as FlashBlockError, FlashBlock, FlashDevice, clear_block, load_block, save_block,
    },
    pixel_target::{PixelTarget, rgb888_from_rgb565},
};
use embassy_time::Timer;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::{
    Drawable, Pixel,
    mono_font::{MonoFont, MonoTextStyle},
    pixelcolor::{IntoStorage, Rgb565, Rgb888},
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, ImageData, Storage};

pub use animation_frame::next_animation_frame;
pub use clock::ClockSyncWasm;
pub use dns::DnsSimulatorWasm;
pub use simulator::{
    CydSimulatorControlWasm, CydSimulatorWasm, WifiConnectOutcome, WifiSimulatorWasm,
};

const FLASH_BLOCK_SIZE: usize = 4096;
const FLASH_BLOCK_OFFSET: u32 = 0;
const FLASH_ERASED_BYTE: u8 = 0xFF;

const fn identity_calibration_config() -> CalibrationConfig {
    CalibrationConfig::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
}

/// A CYD display simulated on an HTML canvas.
pub struct CydWasm {
    display: CydDisplayWasm,
    touch: CydTouchWasm,
}

#[derive(Clone)]
pub struct CydDisplayWasm {
    context: CanvasRenderingContext2d,
    size: Size,
    orientation: Orientation,
    background_color: Rgb888,
    foreground_color: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

#[derive(Clone)]
pub struct CydTouchWasm {
    raw_touch_events: RawTouchEvents,
    interaction_state: Rc<Cell<InteractionState>>,
    latest_raw_point: Rc<Cell<Option<RawPoint>>>,
    calibration_config: CalibrationConfig,
    orientation: Orientation,
}

#[derive(Clone)]
pub struct CydTouchWasmSource {
    raw_touch_events: RawTouchEvents,
    interaction_state: Rc<Cell<InteractionState>>,
    latest_raw_point: Rc<Cell<Option<RawPoint>>>,
}

pub struct ButtonWasm {
    pressed: Rc<Cell<bool>>,
}

#[derive(Clone)]
pub struct ButtonWasmSource {
    pressed: Rc<Cell<bool>>,
}

pub struct FlashBlockWasm {
    flash_device: FlashDeviceWasm,
}

type RawTouchEvents = Rc<RefCell<VecDeque<RawTouchEvent>>>;

struct FlashDeviceWasm {
    storage: Storage,
    storage_key: String,
    bytes: [u8; FLASH_BLOCK_SIZE],
}

#[derive(Debug)]
pub enum Error {
    StorageUnavailable,
    StorageAccess,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InteractionState {
    Ready,
    PointerDown,
    WaitingForFreshPress,
}

impl CydWasm {
    /// Build a simulated CYD that presents onto `context`, sized for `orientation`.
    #[must_use]
    pub fn new(
        context: CanvasRenderingContext2d,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        touch_source: CydTouchWasmSource,
    ) -> Self {
        let display = CydDisplayWasm {
            context: context.clone(),
            size: orientation.size(),
            orientation,
            background_color,
            foreground_color,
            background565: Rgb565::from(background_color),
            foreground565: Rgb565::from(foreground_color),
            font,
        };
        let touch = CydTouchWasm {
            raw_touch_events: touch_source.raw_touch_events,
            interaction_state: touch_source.interaction_state,
            latest_raw_point: touch_source.latest_raw_point,
            calibration_config: identity_calibration_config(),
            orientation,
        };
        Self { display, touch }
    }

    #[must_use]
    pub fn touch_source(&self) -> CydTouchWasmSource {
        CydTouchWasmSource {
            raw_touch_events: self.touch.raw_touch_events.clone(),
            interaction_state: self.touch.interaction_state.clone(),
            latest_raw_point: self.touch.latest_raw_point.clone(),
        }
    }

    #[must_use]
    pub fn display(&self) -> CydDisplayWasm {
        self.display.clone()
    }

    /// Clone owned calibrated parts that share this device's browser state.
    #[must_use]
    pub fn owned_parts(&self) -> (CydDisplayWasm, CydTouchWasm) {
        (self.display.clone(), self.touch.clone())
    }
}

impl Cyd for CydWasm {
    type Error = Infallible;
    type Display = CydDisplayWasm;
    type Touch = CydTouchWasm;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }

    fn orientation(&self) -> Orientation {
        self.display.orientation
    }
}

impl CydTouchWasmSource {
    #[must_use]
    pub fn new() -> Self {
        Self {
            raw_touch_events: Rc::new(RefCell::new(VecDeque::new())),
            interaction_state: Rc::new(Cell::new(InteractionState::Ready)),
            latest_raw_point: Rc::new(Cell::new(None)),
        }
    }

    /// Queue a calibrated-panel point in fixed landscape coordinates.
    ///
    /// The browser control converts logical canvas coordinates to this raw
    /// calibration boundary before calling the source. `CydTouchWasm::read`
    /// performs the one runtime-orientation mapping for the application.
    pub fn touch_down(&self, x: f32, y: f32) {
        match self.interaction_state.get() {
            InteractionState::WaitingForFreshPress => return,
            InteractionState::Ready | InteractionState::PointerDown => {
                self.interaction_state.set(InteractionState::PointerDown);
            }
        }
        assert!((0.0..=u16::MAX as f32).contains(&x));
        assert!((0.0..=u16::MAX as f32).contains(&y));
        let raw_point = RawPoint {
            x: x as u16,
            y: y as u16,
        };
        self.latest_raw_point.set(Some(raw_point));
        self.push(RawTouchEvent::Down {
            raw_x: raw_point.x,
            raw_y: raw_point.y,
        });
    }

    /// Queue a movement point in fixed landscape calibration coordinates.
    pub fn touch_move(&self, x: f32, y: f32) {
        if self.interaction_state.get() != InteractionState::PointerDown {
            return;
        }
        assert!((0.0..=u16::MAX as f32).contains(&x));
        assert!((0.0..=u16::MAX as f32).contains(&y));
        let raw_point = RawPoint {
            x: x as u16,
            y: y as u16,
        };
        self.latest_raw_point.set(Some(raw_point));
        self.push(RawTouchEvent::Move {
            raw_x: raw_point.x,
            raw_y: raw_point.y,
        });
    }

    pub fn touch_up(&self) {
        let interaction_state = self.interaction_state.get();
        self.interaction_state.set(InteractionState::Ready);
        self.latest_raw_point.set(None);
        if interaction_state == InteractionState::WaitingForFreshPress {
            return;
        }
        self.push(RawTouchEvent::Up);
    }

    pub fn wait_for_fresh_press(&self) {
        self.raw_touch_events.borrow_mut().clear();
        self.latest_raw_point.set(None);
        self.interaction_state
            .set(InteractionState::WaitingForFreshPress);
    }

    fn push(&self, raw_touch_event: RawTouchEvent) {
        self.raw_touch_events
            .borrow_mut()
            .push_back(raw_touch_event);
    }
}

impl Default for CydTouchWasmSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonWasmSource {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pressed: Rc::new(Cell::new(false)),
        }
    }

    #[must_use]
    pub fn button(&self) -> ButtonWasm {
        ButtonWasm {
            pressed: self.pressed.clone(),
        }
    }

    pub fn press(&self) {
        self.pressed.set(true);
    }

    pub fn release(&self) {
        self.pressed.set(false);
    }
}

impl Default for ButtonWasmSource {
    fn default() -> Self {
        Self::new()
    }
}

// TODO (may no longer apply) When a dedicated `device-envoy-wasm` crate exists, move `ButtonWasm`
// there so browser button plumbing lives beside the platform button adapter.
impl __ButtonMonitor for ButtonWasm {
    fn is_pressed_raw(&self) -> bool {
        self.pressed.get()
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        loop {
            if self.is_pressed_raw() == pressed {
                break;
            }
            Timer::after(BUTTON_POLL_INTERVAL).await;
        }
    }
}

impl Button for ButtonWasm {}

impl FlashBlockWasm {
    pub fn new(storage_key: &str) -> Result<Self, Error> {
        Ok(Self {
            flash_device: FlashDeviceWasm::new(storage_key)?,
        })
    }
}

impl FlashDeviceWasm {
    fn new(storage_key: &str) -> Result<Self, Error> {
        let window = web_sys::window().ok_or(Error::StorageUnavailable)?;
        let storage = window
            .local_storage()
            .map_err(|_error| Error::StorageAccess)?
            .ok_or(Error::StorageUnavailable)?;
        let mut flash_device = Self {
            storage,
            storage_key: storage_key.to_owned(),
            bytes: [FLASH_ERASED_BYTE; FLASH_BLOCK_SIZE],
        };
        flash_device.load_from_storage()?;
        Ok(flash_device)
    }

    fn load_from_storage(&mut self) -> Result<(), Error> {
        let Some(encoded_bytes) = self
            .storage
            .get_item(&self.storage_key)
            .map_err(|_error| Error::StorageAccess)?
        else {
            return Ok(());
        };

        if encoded_bytes.len() != FLASH_BLOCK_SIZE * 2 {
            return Ok(());
        }

        let mut decoded_bytes = [FLASH_ERASED_BYTE; FLASH_BLOCK_SIZE];
        if !decode_hex_into(&encoded_bytes, &mut decoded_bytes) {
            return Ok(());
        }
        self.bytes = decoded_bytes;
        Ok(())
    }

    fn persist(&self) -> Result<(), Error> {
        let encoded_bytes = encode_hex(&self.bytes);
        self.storage
            .set_item(&self.storage_key, &encoded_bytes)
            .map_err(|_error| Error::StorageAccess)
    }

    fn checked_range(&self, offset: u32, len: usize) -> Range<usize> {
        let start = usize::try_from(offset).expect("flash offset must fit in usize");
        let end = start
            .checked_add(len)
            .expect("flash range must fit in usize");
        assert!(
            end <= FLASH_BLOCK_SIZE,
            "flash range must stay within the block"
        );
        start..end
    }
}

impl FlashDevice for FlashDeviceWasm {
    type Error = Error;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let checked_range = self.checked_range(offset, bytes.len());
        bytes.copy_from_slice(&self.bytes[checked_range]);
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let checked_range = self.checked_range(offset, bytes.len());
        self.bytes[checked_range].copy_from_slice(bytes);
        self.persist()
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let len = usize::try_from(to.saturating_sub(from)).expect("flash erase length fits usize");
        let checked_range = self.checked_range(from, len);
        self.bytes[checked_range].fill(FLASH_ERASED_BYTE);
        self.persist()
    }
}

impl FlashBlock for FlashBlockWasm {
    type Error = FlashBlockError<Error>;

    fn load<T>(&mut self) -> Result<Option<T>, Self::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        load_block::<FLASH_BLOCK_SIZE, T, _>(&mut self.flash_device, FLASH_BLOCK_OFFSET)
    }

    fn save<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        save_block::<FLASH_BLOCK_SIZE, _, _>(&mut self.flash_device, FLASH_BLOCK_OFFSET, value)
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        clear_block::<FLASH_BLOCK_SIZE, _>(&mut self.flash_device, FLASH_BLOCK_OFFSET)
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        encoded.push(HEX_DIGITS[(byte & 0x0F) as usize] as char);
    }
    encoded
}

fn decode_hex_into(encoded_bytes: &str, dst: &mut [u8]) -> bool {
    let encoded_bytes = encoded_bytes.as_bytes();
    if encoded_bytes.len() != dst.len() * 2 {
        return false;
    }

    for (dst_index, chunk) in encoded_bytes.chunks_exact(2).enumerate() {
        let Some(high) = decode_hex_nibble(chunk[0]) else {
            return false;
        };
        let Some(low) = decode_hex_nibble(chunk[1]) else {
            return false;
        };
        dst[dst_index] = (high << 4) | low;
    }

    true
}

const fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl crate::cyd::backend::DisplayBackend for CydDisplayWasm {
    type Error = Infallible;
    type Frame<'a>
        = CydFrameWasm<'a>
    where
        Self: 'a;

    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> Self::Frame<'_> {
        let size = rectangle.size;
        let pixel_count = size.width as usize * size.height as usize;
        let pixels = vec![self.background565.into_storage(); pixel_count];
        CydFrameWasm {
            context: &self.context,
            pixels,
            rectangle,
            tile_top_left,
            background565: self.background565,
            foreground565: self.foreground565,
            font: self.font,
        }
    }
}

impl CydDisplay for CydDisplayWasm {
    fn screen_size(&self) -> Size {
        self.size
    }

    fn background_color(&self) -> Rgb888 {
        self.background_color
    }

    fn foreground_color(&self) -> Rgb888 {
        self.foreground_color
    }

    fn background_565(&self) -> Rgb565 {
        self.background565
    }

    fn foreground_565(&self) -> Rgb565 {
        self.foreground565
    }

    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Infallible> {
        let screen_rectangle = Rectangle::new(Point::zero(), self.size);
        let rectangle = rectangle.intersection(&screen_rectangle);
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return Ok(());
        }

        let pixel_count = rectangle.size.width as usize * rectangle.size.height as usize;
        let mut bytes = Vec::with_capacity(pixel_count * 4);
        for _pixel_index in 0..pixel_count {
            push_rgb565_rgba(&mut bytes, color.into_storage());
        }

        put_image_data(&self.context, rectangle, &bytes);
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), Infallible>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return Ok(());
        }

        let mut bytes =
            Vec::with_capacity(rectangle.size.width as usize * rectangle.size.height as usize * 4);
        for pixel in pixels {
            push_rgb565_rgba(&mut bytes, pixel.into_storage());
        }

        put_image_data(&self.context, rectangle, &bytes);
        Ok(())
    }
}

impl CydTouch for CydTouchWasm {
    type Error = Infallible;

    fn read(&mut self) -> Result<Option<TouchEvent>, Infallible> {
        Ok(self
            .raw_touch_events
            .borrow_mut()
            .pop_front()
            .map(|raw_touch_event| match raw_touch_event {
                RawTouchEvent::Down { raw_x, raw_y } => {
                    let (x, y) = self.calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Down {
                        point: self
                            .orientation
                            .map_landscape_point(Point::new(x as i32, y as i32)),
                    }
                }
                RawTouchEvent::Move { raw_x, raw_y } => {
                    let (x, y) = self.calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Move {
                        point: self
                            .orientation
                            .map_landscape_point(Point::new(x as i32, y as i32)),
                    }
                }
                RawTouchEvent::Up => TouchEvent::Up,
            }))
    }
}

fn put_image_data(context: &CanvasRenderingContext2d, rectangle: Rectangle, bytes: &[u8]) {
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(bytes),
        rectangle.size.width,
        rectangle.size.height,
    )
    .expect("ImageData dimensions match the rectangle");
    context
        .put_image_data(
            &image_data,
            f64::from(rectangle.top_left.x),
            f64::from(rectangle.top_left.y),
        )
        .expect("put_image_data with in-bounds coordinates cannot fail");
}

fn push_rgb565_rgba(bytes: &mut Vec<u8>, pixel: u16) {
    let color = rgb888_from_rgb565(pixel);
    bytes.push(color.r());
    bytes.push(color.g());
    bytes.push(color.b());
    bytes.push(255);
}

/// A single in-progress frame backed by an `Rgb565` pixel buffer.
pub struct CydFrameWasm<'a> {
    context: &'a CanvasRenderingContext2d,
    pixels: Vec<u16>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in screen coordinates. Drawing coordinates are translated
    // by this point before reaching the local frame buffer.
    tile_top_left: Point,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

impl CydFrameWasm<'_> {
    fn width(&self) -> usize {
        self.rectangle.size.width as usize
    }

    fn height(&self) -> usize {
        self.rectangle.size.height as usize
    }

    fn local_x(&self, x: i32) -> Option<usize> {
        usize::try_from(x.checked_sub(self.tile_top_left.x)?).ok()
    }

    fn local_y(&self, y: i32) -> Option<usize> {
        usize::try_from(y.checked_sub(self.tile_top_left.y)?).ok()
    }

    pub fn fill(&mut self, color: Rgb565) -> &mut Self {
        self.pixels.fill(color.into_storage());
        self
    }

    /// Convert the `Rgb565` buffer to RGBA8 and `putImageData` it at the frame's top-left.
    fn present(&self) {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            let color = rgb888_from_rgb565(*pixel);
            bytes.push(color.r());
            bytes.push(color.g());
            bytes.push(color.b());
            bytes.push(255);
        }
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&bytes),
            self.rectangle.size.width,
            self.rectangle.size.height,
        )
        .expect("ImageData dimensions match the pixel buffer");
        self.context
            .put_image_data(
                &image_data,
                f64::from(self.rectangle.top_left.x),
                f64::from(self.rectangle.top_left.y),
            )
            .expect("put_image_data with in-bounds coordinates cannot fail");
    }
}

impl DrawTarget for CydFrameWasm<'_> {
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
            if local_x < CydFrameWasm::width(self) && local_y < CydFrameWasm::height(self) {
                let index = local_y * CydFrameWasm::width(self) + local_x;
                self.pixels[index] = color.into_storage();
            }
        }
        Ok(())
    }
}

impl Dimensions for CydFrameWasm<'_> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.tile_top_left, self.rectangle.size)
    }
}

impl PixelTarget for CydFrameWasm<'_> {
    fn width(&self) -> usize {
        usize::try_from(self.tile_top_left.x)
            .expect("tile top-left x must be non-negative")
            .checked_add(CydFrameWasm::width(self))
            .expect("frame width must fit in usize")
    }

    fn height(&self) -> usize {
        usize::try_from(self.tile_top_left.y)
            .expect("tile top-left y must be non-negative")
            .checked_add(CydFrameWasm::height(self))
            .expect("frame height must fit in usize")
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        let Some(local_x) = self.local_x(x as i32) else {
            return;
        };
        let Some(local_y) = self.local_y(y as i32) else {
            return;
        };
        if local_x >= CydFrameWasm::width(self) || local_y >= CydFrameWasm::height(self) {
            return;
        }
        let stride = CydFrameWasm::width(self);
        self.pixels[local_y * stride + local_x] = Rgb565::from(color).into_storage();
    }

    /// The frame buffer already stores RGB565, so a decoded image pixel can be
    /// written verbatim with no RGB888 round-trip.
    fn put_pixel_565(&mut self, x: usize, y: usize, rgb565: u16) {
        let Some(local_x) = self.local_x(x as i32) else {
            return;
        };
        let Some(local_y) = self.local_y(y as i32) else {
            return;
        };
        if local_x >= CydFrameWasm::width(self) || local_y >= CydFrameWasm::height(self) {
            return;
        }
        let stride = CydFrameWasm::width(self);
        self.pixels[local_y * stride + local_x] = rgb565;
    }
}

impl CydFrame for CydFrameWasm<'_> {
    type Error = Infallible;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        CydFrameWasm::fill(self, color)
    }

    fn clear(&mut self) -> &mut Self {
        self.fill(self.background565)
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

    fn write_text(&mut self, text: &str) -> &mut Self {
        let style = MonoTextStyle::new(self.font, self.foreground565);
        Text::with_baseline(text, Point::zero(), style, Baseline::Top)
            .draw(self)
            .expect("drawing onto an Infallible frame cannot fail");
        self
    }

    async fn flush(&mut self) -> Result<(), Infallible> {
        // Present immediately so the first drawn frame is visible without
        // waiting a browser tick, then yield to the next animation frame to
        // pace the loop.
        self.present();
        next_animation_frame().await;
        Ok(())
    }
}
