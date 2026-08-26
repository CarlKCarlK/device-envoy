//! A device abstraction for the "Cheap Yellow Display" (CYD) family of
//! 320×240 ILI9341 + XPT2046 resistive-touch ESP32 boards.
//!
//! See [`CydEsp`] and [`CydDisplayEsp`] for the public constructors; the
//! device-agnostic [`CydDisplay`] and [`CydTouch`] traits live in
//! [`device_envoy_core::cyd`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/).

// TODO0 Reduce CYD's public API surface; see specs/CYD_PUBLIC_API_CLEANUP.md.

mod buffer;
mod display;
mod one_spi;
mod text;
#[path = "cyd/touch.rs"]
mod touch_driver;

use core::{convert::Infallible, fmt};

use embedded_graphics::{
    Pixel,
    mono_font::MonoFont,
    pixelcolor::{IntoStorage, Rgb565, Rgb888},
    prelude::{Dimensions, DrawTarget, OriginDimensions, Point, Size},
    primitives::Rectangle,
};
use embedded_hal::spi::SpiDevice;
use static_cell::StaticCell;

use buffer::DynPixelBuffer;
use buffer::{PixelBuffer, RegionView};
use device_envoy_core::button::Button;
use device_envoy_core::cyd::backend;
use device_envoy_core::cyd::{
    SCREEN_PIXELS,
    backend::{CalibrationConfig, RawTouchEvent, TouchUncalibrated},
    display::CydFrame,
    touch::TouchEvent,
};
use device_envoy_core::pixel_target::PixelTarget;
pub use display::DEFAULT_DISPLAY_SPI_HZ;
// The device abstraction and its neutral support types live in
// `device-envoy-core::cyd`; re-export the public surface from this device crate.
pub use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{Orientation, tiling},
    touch,
};
pub use one_spi::CydEspOneSpi;
pub use text::DEFAULT_FONT;
use touch_driver::TOUCH_SPI_HZ;

use crate::flash_block::FlashBlockEsp;
use display::CydDisplayEsp as CydDisplayEspDevice;
use touch_driver::CydTouchEsp as CydTouchEspDevice;

/// An owned CYD-family ESP32 display component.
///
/// `D` is the underlying `embedded-hal` SPI device type; it defaults to an
/// exclusively-owned SPI peripheral. Shared-bus backends (see
/// [`CydEspOneSpi`]) instantiate this with an
/// `embedded_hal_bus::spi::RefCellDevice` instead.
///
/// The display constructor and static-storage pattern are shown by
/// [`CydStaticEsp`].
/// The complete-device constructor is shown by [`CydEsp::new`].
pub struct CydDisplayEsp<D: SpiDevice<u8> = display::CydDisplaySpiDevice> {
    display: CydDisplayEspDevice<D>,
    orientation: Orientation,
    // Every CydEsp owns exactly one draw buffer. Apps that don't draw through it
    // pass a zero-sized buffer (e.g. `CydStaticEsp<0>`).
    pixel_buffer: &'static mut dyn DynPixelBuffer,
    // Default drawing style. Background clears the device at construction and
    // fills every new frame; foreground color and font drive `CydFrameEsp::write_text`.
    // The `Rgb565` versions are precomputed so the hot drawing paths skip the
    // per-call conversion.
    background_color: Rgb888,
    foreground_color: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// An owned uncalibrated CYD-family ESP32 touch component.
///
/// `D` is the underlying `embedded-hal` SPI device type; see
/// [`CydDisplayEsp`] for the shared-bus rationale.
pub(crate) struct CydTouchUncalibratedEsp<D = touch_driver::CydTouchSpiDevice> {
    touch: CydTouchEspDevice<D>,
}

/// An owned calibrated CYD-family ESP32 touch component.
///
/// Construct it as part of [`CydEsp::new`]; applications then call the
/// calibrated [`CydTouch::read`]
/// operation.
pub struct CydTouchEsp<D = touch_driver::CydTouchSpiDevice> {
    raw: CydTouchUncalibratedEsp<D>,
    calibration_config: CalibrationConfig,
    orientation: Orientation,
}

/// A calibrated CYD-family ESP32 bundle.
///
/// Use [`CydEsp::new`] after declaring [`CydStaticEsp`] storage; construction
/// performs the saved-or-interactive touch calibration before returning.
pub struct CydEsp {
    /// The owned display component.
    pub display: CydDisplayEsp,
    /// The owned calibrated touch component.
    pub touch: CydTouchEsp,
}

/// An uncalibrated CYD-family ESP32 bundle.
pub(crate) struct CydEspUncalibrated {
    /// The owned display component.
    pub display: CydDisplayEsp,
    /// The owned uncalibrated touch component.
    pub touch: CydTouchUncalibratedEsp,
}

/// Static storage for a [`CydEsp`]-owned pixel buffer.
///
/// The app declares one at file scope and names the workspace pixel count it
/// wants:
///
/// ```rust,no_run
/// #![no_std]
/// #![no_main]
/// use device_envoy_esp::cyd::{CydEsp, CydStaticEsp};
/// static CYD_STATIC: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
/// ```
///
/// The app chooses the pixel count (policy); [`CydDisplayEsp::new`] owns the
/// initialization protocol and the storage details.
pub struct CydStaticEsp<const PIXEL_COUNT: usize> {
    pixel_buffer: StaticCell<PixelBuffer<PIXEL_COUNT>>,
}

impl<const PIXEL_COUNT: usize> CydStaticEsp<PIXEL_COUNT> {
    /// Internal constructor. Apps create storage via [`CydEsp::new_static`] so all
    /// construction goes through the `CydEsp` device abstraction.
    pub(crate) const fn new() -> Self {
        Self {
            pixel_buffer: StaticCell::new(),
        }
    }
}

/// A single in-progress frame backed by an `Rgb565` pixel buffer.
///
/// Frames are returned by [`CydDisplay::frame_mut`] and implement the core
/// [`CydFrame`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/display/trait.CydFrame.html)
/// operations.
///
/// ```rust,no_run
/// #![no_std]
/// #![no_main]
/// use device_envoy_esp::cyd::{CydFrameEsp, Error};
/// use embedded_hal::spi::SpiDevice;
/// use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// fn use_frame<D: SpiDevice<u8>>(frame: &mut CydFrameEsp<'_, D>) -> Result<(), Error> {
///     let _width = frame.width();
///     let _height = frame.height();
///     frame.fill(Rgb565::BLACK).write_text("CYD");
///     let _pixels = frame.raw_pixels_mut();
///     frame.flush()
/// }
/// ```
pub struct CydFrameEsp<'a, D: SpiDevice<u8> = display::CydDisplaySpiDevice> {
    display: &'a mut CydDisplayEspDevice<D>,
    view: RegionView<'a>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in screen coordinates. Drawing coordinates are translated
    // by this point before reaching the local frame buffer.
    tile_top_left: Point,
    // Default foreground color and font, copied from the owning `CydDisplayEsp`, so
    // `write_text` can render with the device default style.
    pub(crate) background565: Rgb565,
    pub(crate) foreground565: Rgb565,
    pub(crate) font: &'static MonoFont<'static>,
}

impl<'a, D: SpiDevice<u8>> CydFrameEsp<'a, D> {
    /// Fill the frame with an explicit color.
    ///
    /// See the [canonical `CydFrameEsp` example](CydFrameEsp).
    pub fn fill(&mut self, color: Rgb565) -> &mut Self {
        self.view.fill(color);
        self
    }

    /// The frame's width in pixels.
    ///
    /// See the [canonical `CydFrameEsp` example](CydFrameEsp).
    #[must_use]
    pub fn width(&self) -> usize {
        self.view.width()
    }

    /// The frame's height in pixels.
    ///
    /// See the [canonical `CydFrameEsp` example](CydFrameEsp).
    #[must_use]
    pub fn height(&self) -> usize {
        self.view.height()
    }

    /// Borrow the frame's raw RGB565 pixels, row-major.
    ///
    /// See the [canonical `CydFrameEsp` example](CydFrameEsp).
    pub fn raw_pixels_mut(&mut self) -> &mut [u16] {
        self.view.raw_pixels_mut()
    }

    /// Present this frame's pixels at its rectangle's top-left (set by
    /// [`CydDisplay::frame_mut`]).
    ///
    /// See the [canonical `CydFrameEsp` example](CydFrameEsp).
    pub fn flush(&mut self) -> Result<(), Error> {
        Ok(self.display.flush_buffer(
            self.view.size().width as usize,
            self.view.size().height as usize,
            self.view.raw_pixels(),
            self.rectangle.top_left,
        )?)
    }

    fn local_x(&self, x: i32) -> Option<usize> {
        usize::try_from(x.checked_sub(self.tile_top_left.x)?).ok()
    }

    fn local_y(&self, y: i32) -> Option<usize> {
        usize::try_from(y.checked_sub(self.tile_top_left.y)?).ok()
    }
}

impl<D: SpiDevice<u8>> DrawTarget for CydFrameEsp<'_, D> {
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
            if local_x < self.view.width() && local_y < self.view.height() {
                let index = local_y * self.view.width() + local_x;
                self.raw_pixels_mut()[index] = color.into_storage();
            }
        }
        Ok(())
    }
}

impl<D: SpiDevice<u8>> Dimensions for CydFrameEsp<'_, D> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.tile_top_left, self.view.size())
    }
}

impl<D: SpiDevice<u8>> PixelTarget for CydFrameEsp<'_, D> {
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
        let Some(local_x) = self.local_x(x as i32) else {
            return;
        };
        let Some(local_y) = self.local_y(y as i32) else {
            return;
        };
        if local_x >= self.view.width() || local_y >= self.view.height() {
            return;
        }
        let stride = self.view.width();
        self.raw_pixels_mut()[local_y * stride + local_x] = Rgb565::from(color).into_storage();
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
        if local_x >= self.view.width() || local_y >= self.view.height() {
            return;
        }
        let stride = self.view.width();
        self.raw_pixels_mut()[local_y * stride + local_x] = rgb565;
    }
}

/// Error from a CYD ESP display or touch operation.
///
/// See the [`CydEsp::new`] constructor example, which propagates this error.
#[derive(Debug)]
pub enum Error {
    /// Configuring the display SPI peripheral failed.
    ConfigureDisplaySpi(esp_hal::spi::master::ConfigError),
    /// The display panel could not be initialized.
    InitDisplay,
    /// Configuring the touch SPI peripheral failed.
    ConfigureTouchSpi(esp_hal::spi::master::ConfigError),
    /// A frame could not be flushed to the display.
    FlushFrameBuffer,
    /// Changing the display orientation failed.
    SetOrientation,
}

impl<D: SpiDevice<u8>> CydDisplayEsp<D> {
    fn set_orientation(&mut self, orientation: Orientation) -> Result<(), Error> {
        self.display.set_orientation(orientation)?;
        self.orientation = orientation;
        Ok(())
    }

    fn from_display_device(
        mut display: CydDisplayEspDevice<D>,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        pixel_buffer: &'static mut dyn DynPixelBuffer,
    ) -> Result<Self, Error> {
        let background565 = rgb565(background_color);
        display.fill(background565)?;

        Ok(Self {
            display,
            orientation,
            pixel_buffer,
            background_color,
            foreground_color,
            background565,
            foreground565: rgb565(foreground_color),
            font,
        })
    }

    /// Construct a display component from an already-built SPI device.
    ///
    /// Used by shared-bus backends (see [`CydEspOneSpi`]) that build their
    /// own `SpiDevice` instead of owning an exclusive SPI peripheral.
    pub(crate) fn new_from_device(
        spi_device: D,
        dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        pixel_buffer: &'static mut dyn DynPixelBuffer,
    ) -> Result<Self, Error> {
        let display = CydDisplayEspDevice::new_from_device(
            spi_device,
            dc_pin,
            rst_pin,
            backlight_pin,
            orientation,
        )?;
        Self::from_display_device(
            display,
            orientation,
            background_color,
            foreground_color,
            font,
            pixel_buffer,
        )
    }
}

impl CydDisplayEsp<display::CydDisplaySpiDevice> {
    /// Construct a display-only CYD display component that owns its draw buffer.
    ///
    /// ```rust,no_run
    /// #![no_std]
    /// #![no_main]
    /// use device_envoy_esp::{Result, cyd::{CydDisplayEsp, CydEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}};
    /// use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    /// async fn construct(p: esp_hal::peripherals::Peripherals) -> Result<()> {
    ///     static STORAGE: device_envoy_esp::cyd::CydStaticEsp<0> = CydEsp::new_static();
    ///     let _display = CydDisplayEsp::new(&STORAGE, p.SPI2, p.GPIO1, p.GPIO2, p.GPIO3,
    ///         p.GPIO4, p.GPIO5, p.GPIO7, p.GPIO8, DEFAULT_DISPLAY_SPI_HZ,
    ///         Orientation::Landscape, Rgb888::BLACK, Rgb888::WHITE, &DEFAULT_FONT)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_spi_hz: u32,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<Self, Error> {
        let pixel_buffer = PixelBuffer::init_static(&statics.pixel_buffer);
        let display = CydDisplayEspDevice::new(
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            display_spi_hz,
            orientation,
        )?;
        Self::from_display_device(
            display,
            orientation,
            background_color,
            foreground_color,
            font,
            pixel_buffer,
        )
    }
}

impl<D: SpiDevice<u8>> CydTouchUncalibratedEsp<D> {
    /// Construct an uncalibrated touch component from an already-built SPI device.
    ///
    /// Used by shared-bus backends (see [`CydEspOneSpi`]) that build their
    /// own `SpiDevice` instead of owning an exclusive SPI peripheral.
    pub(crate) fn from_device(
        touch_spi_device: D,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
    ) -> Self {
        Self {
            touch: CydTouchEspDevice::from_device(touch_spi_device, touch_irq_pin),
        }
    }
}

impl CydTouchUncalibratedEsp<touch_driver::CydTouchSpiDevice> {
    /// Construct an uncalibrated touch component.
    pub(crate) fn new(
        touch_spi: impl esp_hal::spi::master::Instance + 'static,
        touch_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
    ) -> Result<Self, Error> {
        Ok(Self {
            touch: CydTouchEspDevice::new(
                touch_spi,
                touch_sck_pin,
                touch_mosi_pin,
                touch_miso_pin,
                touch_cs_pin,
                touch_irq_pin,
            )?,
        })
    }
}

impl CydEsp {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    ///
    /// Used by the [`CydStaticEsp`] storage example.
    pub const SCREEN_PIXELS: usize = SCREEN_PIXELS;

    /// Create [`CydStaticEsp`] storage for a `PIXEL_COUNT`-sized draw buffer.
    ///
    /// See the [`CydStaticEsp`] example.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticEsp<PIXEL_COUNT> {
        CydStaticEsp::new()
    }

    /// Construct a ready-to-use calibrated CYD, loading or completing calibration internally.
    ///
    /// ```rust,no_run
    /// #![no_std]
    /// #![no_main]
    /// use device_envoy_esp::{Result, button::{ButtonEsp, PressedTo}, cyd::{CydEsp, CydStaticEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}, flash_block::FlashBlockEsp};
    /// use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// use esp_hal::spi::master::AnySpi;
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    /// async fn construct(mut p: esp_hal::peripherals::Peripherals) -> Result<()> {
    ///     let [mut flash] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    ///     let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    /// #     let touch_spi: AnySpi<'static> = todo!();
    ///     static STORAGE: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
    ///     let _cyd = CydEsp::new(&STORAGE, p.SPI2, p.GPIO1, p.GPIO2, p.GPIO3, p.GPIO4,
    ///         p.GPIO5, p.GPIO7, p.GPIO8, DEFAULT_DISPLAY_SPI_HZ, Orientation::Landscape,
    ///         Rgb888::BLACK, Rgb888::WHITE, &DEFAULT_FONT, touch_spi, p.GPIO9, p.GPIO10,
    ///         p.GPIO11, p.GPIO12, p.GPIO13, &mut flash, &mut button).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new<const PIXEL_COUNT: usize, R: Button>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_spi_hz: u32,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: impl esp_hal::spi::master::Instance + 'static,
        touch_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
        calibration_flash_block: &mut FlashBlockEsp,
        recalibration_button: &mut R,
    ) -> crate::Result<Self> {
        let CydEspUncalibrated { mut display, touch } = CydEspUncalibrated::new(
            statics,
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            display_spi_hz,
            orientation,
            background_color,
            foreground_color,
            font,
            touch_spi,
            touch_sck_pin,
            touch_mosi_pin,
            touch_miso_pin,
            touch_cs_pin,
            touch_irq_pin,
        )?;
        let touch = backend::ensure_calibration(
            &mut display,
            touch,
            calibration_flash_block,
            recalibration_button,
            None,
            orientation,
        )
        .await
        .map_err(|error| match error {
            backend::Error::Device(cyd_error) => crate::Error::from(cyd_error),
            backend::Error::Flash(flash_error) => flash_error,
        })?;
        display.set_orientation(orientation)?;
        Ok(Self { display, touch })
    }
}

impl Cyd for CydEsp {
    type Error = Error;
    type Display = CydDisplayEsp;
    type Touch = CydTouchEsp;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }

    fn orientation(&self) -> Orientation {
        self.display.orientation
    }
}

impl CydEspUncalibrated {
    pub(crate) fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_spi_hz: u32,
        _orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: impl esp_hal::spi::master::Instance + 'static,
        touch_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
    ) -> Result<Self, Error> {
        Ok(Self {
            display: CydDisplayEsp::new(
                statics,
                display_spi,
                display_sck_pin,
                display_mosi_pin,
                display_miso_pin,
                display_cs_pin,
                display_dc_pin,
                display_rst_pin,
                display_backlight_pin,
                display_spi_hz,
                Orientation::Landscape,
                background_color,
                foreground_color,
                font,
            )?,
            touch: CydTouchUncalibratedEsp::new(
                touch_spi,
                touch_sck_pin,
                touch_mosi_pin,
                touch_miso_pin,
                touch_cs_pin,
                touch_irq_pin,
            )?,
        })
    }
}

fn rgb565(color: Rgb888) -> Rgb565 {
    Rgb565::from(color)
}

impl<D: SpiDevice<u8>> fmt::Debug for CydDisplayEsp<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydDisplayEsp")
            .finish_non_exhaustive()
    }
}

impl<D> fmt::Debug for CydTouchUncalibratedEsp<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchUncalibratedEsp")
            .finish_non_exhaustive()
    }
}

impl<D> fmt::Debug for CydTouchEsp<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchEsp")
            .field("calibration_config", &self.calibration_config)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydEsp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CydEsp").finish_non_exhaustive()
    }
}

impl fmt::Debug for CydEspUncalibrated {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydEspUncalibrated")
            .finish_non_exhaustive()
    }
}

impl<D: SpiDevice<u8>> backend::DisplayBackend for CydDisplayEsp<D> {
    type Error = Error;
    type Frame<'a>
        = CydFrameEsp<'a, D>
    where
        Self: 'a;

    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> Self::Frame<'_> {
        self.display.make_frame_with_tile_top_left(
            self.pixel_buffer,
            rectangle,
            tile_top_left,
            self.background565,
            self.foreground565,
            self.font,
        )
    }
}

impl<D: SpiDevice<u8>> CydDisplay for CydDisplayEsp<D> {
    #[inline]
    fn screen_size(&self) -> Size {
        self.display.size()
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

    #[inline]
    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Error> {
        Ok(self.display.fill_rectangle(rectangle, color)?)
    }

    #[inline]
    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        Ok(self.display.fill_contiguous(rectangle, pixels)?)
    }
}

impl<D: SpiDevice<u8>> TouchUncalibrated for CydTouchUncalibratedEsp<D> {
    type Error = Error;
    type Calibrated = CydTouchEsp<D>;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
        Ok(self.touch.read_raw_touch_event())
    }

    fn calibrate(
        self,
        calibration_config: CalibrationConfig,
        orientation: Orientation,
    ) -> Self::Calibrated {
        CydTouchEsp {
            raw: self,
            calibration_config,
            orientation,
        }
    }
}

impl<D: SpiDevice<u8>> CydTouch for CydTouchEsp<D> {
    type Error = Error;

    fn read(&mut self) -> Result<Option<TouchEvent>, Error> {
        Ok(self
            .raw
            .touch
            .read_raw_touch_event()
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

impl<D: SpiDevice<u8>> CydFrame for CydFrameEsp<'_, D> {
    type Error = Error;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        CydFrameEsp::fill(self, color)
    }

    fn clear(&mut self) -> &mut Self {
        self.fill(self.background565)
    }

    fn write_text(&mut self, text: &str) -> &mut Self {
        CydFrameEsp::write_text(self, text)
    }

    fn copy_from_565(&mut self, src: &[u16]) -> device_envoy_core::Result<()> {
        let dst = self.raw_pixels_mut();
        if dst.len() != src.len() {
            return Err(device_envoy_core::Error::CopySize {
                src_len: src.len(),
                frame_len: dst.len(),
            });
        }
        dst.copy_from_slice(src);
        Ok(())
    }

    // Flushing the panel over SPI is synchronous, so this future resolves on its
    // first poll. The `async fn` is the device-agnostic frame boundary the
    // render loop awaits; on the MCU it adds no suspension.
    async fn flush(&mut self) -> Result<(), Error> {
        CydFrameEsp::flush(self)
    }
}
