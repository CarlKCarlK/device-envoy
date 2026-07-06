//! Browser-simulated [`Cyd`], [`Button`], and [`FlashBlock`] implementations.
//!
//! Requires the `wasm` feature. [`CydWasm`] offers the device-agnostic
//! [`Cyd`] display/touch parts against an HTML
//! canvas, so the same generic example code that drives the real esp32 `CydEsp`
//! also runs in a web page. Its [`CydFrameWasm::flush`] awaits the next browser animation
//! frame (see [`next_animation_frame`]), blits the frame to the canvas, then
//! resolves — turning a platform-neutral `loop { draw; flush().await?; }`
//! into smooth, repaint-paced animation without inverting the loop into a state
//! machine.

mod animation_frame;

use core::{
    cell::{Cell, RefCell},
    convert::Infallible,
    ops::Range,
};
use std::{collections::VecDeque, rc::Rc};

use crate::cyd::{
    Cyd, CydDisplay, CydFrame, CydInfallibleError, CydTouch, Orientation, TouchEvent,
    calibration::{
        CalibrationConfig, CydRawTouch, RawPoint, RawTouchEvent, distort_demo_screen_to_raw,
    },
};
use crate::{
    button::{__ButtonMonitor, BUTTON_POLL_INTERVAL, Button},
    flash_block::{FlashBlock, FlashBlockError, FlashDevice, clear_block, load_block, save_block},
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

const FLASH_BLOCK_SIZE: usize = 4096;
const FLASH_BLOCK_OFFSET: u32 = 0;
const FLASH_ERASED_BYTE: u8 = 0xFF;

/// A CYD display simulated on an HTML canvas.
pub struct CydWasm {
    context: CanvasRenderingContext2d,
    size: Size,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
    raw_touch_events: RawTouchEvents,
    interaction_state: Rc<Cell<InteractionState>>,
    latest_raw_point: Rc<Cell<Option<RawPoint>>>,
    calibration_config: Option<CalibrationConfig>,
}

pub struct CydDisplayWasmPart<'a> {
    context: &'a CanvasRenderingContext2d,
    size: Size,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

pub struct CydTouchWasmPart {
    raw_touch_events: RawTouchEvents,
    calibration_config: Option<CalibrationConfig>,
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
pub enum FlashDeviceWasmError {
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
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        touch_source: CydTouchWasmSource,
    ) -> Self {
        Self {
            context,
            size: orientation.size(),
            background,
            foreground,
            background565: Rgb565::from(background),
            foreground565: Rgb565::from(foreground),
            font,
            raw_touch_events: touch_source.raw_touch_events,
            interaction_state: touch_source.interaction_state,
            latest_raw_point: touch_source.latest_raw_point,
            calibration_config: None,
        }
    }

    #[must_use]
    pub fn touch_source(&self) -> CydTouchWasmSource {
        CydTouchWasmSource {
            raw_touch_events: self.raw_touch_events.clone(),
            interaction_state: self.interaction_state.clone(),
            latest_raw_point: self.latest_raw_point.clone(),
        }
    }

    pub fn set_calibration(&mut self, calibration_config: CalibrationConfig) {
        self.calibration_config = Some(calibration_config);
    }

    pub fn clear_calibration(&mut self) {
        self.calibration_config = None;
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

    pub fn touch_down(&self, x: f32, y: f32) {
        match self.interaction_state.get() {
            InteractionState::WaitingForFreshPress => return,
            InteractionState::Ready | InteractionState::PointerDown => {
                self.interaction_state.set(InteractionState::PointerDown);
            }
        }
        let raw_point = distort_demo_screen_to_raw(x, y);
        self.latest_raw_point.set(Some(raw_point));
        self.push(RawTouchEvent::Down {
            raw_x: raw_point.x,
            raw_y: raw_point.y,
        });
    }

    pub fn touch_move(&self, x: f32, y: f32) {
        if self.interaction_state.get() != InteractionState::PointerDown {
            return;
        }
        let raw_point = distort_demo_screen_to_raw(x, y);
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

// TODO When a dedicated `device-envoy-wasm` crate exists, move `ButtonWasm`
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
    pub fn new(storage_key: &str) -> Result<Self, FlashDeviceWasmError> {
        Ok(Self {
            flash_device: FlashDeviceWasm::new(storage_key)?,
        })
    }
}

impl FlashDeviceWasm {
    fn new(storage_key: &str) -> Result<Self, FlashDeviceWasmError> {
        let window = web_sys::window().ok_or(FlashDeviceWasmError::StorageUnavailable)?;
        let storage = window
            .local_storage()
            .map_err(|_error| FlashDeviceWasmError::StorageAccess)?
            .ok_or(FlashDeviceWasmError::StorageUnavailable)?;
        let mut flash_device = Self {
            storage,
            storage_key: storage_key.to_owned(),
            bytes: [FLASH_ERASED_BYTE; FLASH_BLOCK_SIZE],
        };
        flash_device.load_from_storage()?;
        Ok(flash_device)
    }

    fn load_from_storage(&mut self) -> Result<(), FlashDeviceWasmError> {
        let Some(encoded_bytes) = self
            .storage
            .get_item(&self.storage_key)
            .map_err(|_error| FlashDeviceWasmError::StorageAccess)?
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

    fn persist(&self) -> Result<(), FlashDeviceWasmError> {
        let encoded_bytes = encode_hex(&self.bytes);
        self.storage
            .set_item(&self.storage_key, &encoded_bytes)
            .map_err(|_error| FlashDeviceWasmError::StorageAccess)
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
    type Error = FlashDeviceWasmError;

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
    type Error = FlashBlockError<FlashDeviceWasmError>;

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

impl Cyd for CydWasm {
    // Presenting to a canvas cannot fail, so the device-agnostic render loop
    // never has a real error to propagate.
    type Error = CydInfallibleError;
    type Display<'a> = CydDisplayWasmPart<'a>;
    type Touch<'a> = CydTouchWasmPart;

    fn parts(&mut self) -> (CydDisplayWasmPart<'_>, CydTouchWasmPart) {
        (
            CydDisplayWasmPart {
                context: &self.context,
                size: self.size,
                background: self.background,
                foreground: self.foreground,
                background565: self.background565,
                foreground565: self.foreground565,
                font: self.font,
            },
            CydTouchWasmPart {
                raw_touch_events: self.raw_touch_events.clone(),
                calibration_config: self.calibration_config,
            },
        )
    }
}

impl CydDisplay for CydDisplayWasmPart<'_> {
    type Error = CydInfallibleError;
    type Frame<'a>
        = CydFrameWasm<'a>
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

    fn frame_mut(&mut self, rectangle: Rectangle) -> CydFrameWasm<'_> {
        self.frame_mut_with_tile_top_left(rectangle, Point::zero())
    }

    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> CydFrameWasm<'_> {
        let size = rectangle.size;
        let pixel_count = size.width as usize * size.height as usize;
        // Every new frame starts cleared to the device background so callers
        // never have to clear it themselves.
        let pixels = vec![self.background565.into_storage(); pixel_count];
        CydFrameWasm {
            context: &self.context,
            pixels,
            rectangle,
            tile_top_left,
            foreground565: self.foreground565,
            font: self.font,
        }
    }

    fn fill_rectangle(
        &mut self,
        rectangle: Rectangle,
        color: Rgb565,
    ) -> Result<(), CydInfallibleError> {
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

        put_image_data(self.context, rectangle, &bytes);
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        rectangle: Rectangle,
        pixels: I,
    ) -> Result<(), CydInfallibleError>
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

        put_image_data(self.context, rectangle, &bytes);
        Ok(())
    }
}

impl CydTouch for CydTouchWasmPart {
    type Error = CydInfallibleError;

    fn read(&mut self) -> Result<Option<TouchEvent>, CydInfallibleError> {
        let Some(calibration_config) = self.calibration_config else {
            return Ok(None);
        };
        Ok(self
            .raw_touch_events
            .borrow_mut()
            .pop_front()
            .map(|raw_touch_event| match raw_touch_event {
                RawTouchEvent::Down { raw_x, raw_y } => {
                    let (x, y) = calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Down {
                        point: Point::new(x as i32, y as i32),
                    }
                }
                RawTouchEvent::Move { raw_x, raw_y } => {
                    let (x, y) = calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Move {
                        point: Point::new(x as i32, y as i32),
                    }
                }
                RawTouchEvent::Up => TouchEvent::Up,
            }))
    }
}

impl CydRawTouch for CydWasm {
    type Error = CydInfallibleError;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, CydInfallibleError> {
        if let Some(raw_touch_event) = self.raw_touch_events.borrow_mut().pop_front() {
            return Ok(Some(raw_touch_event));
        }

        if self.interaction_state.get() != InteractionState::PointerDown {
            return Ok(None);
        }

        let Some(raw_point) = self.latest_raw_point.get() else {
            return Ok(None);
        };

        Ok(Some(RawTouchEvent::Move {
            raw_x: raw_point.x,
            raw_y: raw_point.y,
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
    type Error = CydInfallibleError;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        CydFrameWasm::fill(self, color)
    }

    fn copy_from_565(&mut self, src: &[u16]) -> Result<(), crate::cyd::CopySizeError> {
        if self.pixels.len() != src.len() {
            return Err(crate::cyd::CopySizeError {
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

    async fn flush(&mut self) -> Result<(), CydInfallibleError> {
        // Present immediately so the first drawn frame is visible without
        // waiting a browser tick, then yield to the next animation frame to
        // pace the loop.
        self.present();
        next_animation_frame().await;
        Ok(())
    }
}
