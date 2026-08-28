#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "cyd_application_preview",
        "../device-envoy-core/docs/assets/cyd_application_preview.png"
    )
)]
#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "linkage_blaze_gallery",
        "../device-envoy-core/docs/assets/linkage_blaze_gallery.png"
    )
)]
//! ESP32 support for Cheap Yellow Display (CYD) boards.
//!
//! These boards combine a 320×240 ILI9341 display with XPT2046 resistive
//! touch. After construction, applications use the portable display and
//! calibrated-touch interfaces from
//! [`device_envoy_core::cyd`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/).
//! The RP, WebAssembly, and in-memory implementations share these interfaces.
#![doc = include_str!("../../../docs/cyd/gallery.md")]
#![doc = include_str!("../../../docs/cyd/application-example.md")]
//!
//! [Choose a constructor](#choose-a-constructor) explains how to construct an
//! ESP32 device. The
//! [ESP32 CYD touch-paint example](https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-esp/examples/esp32/generic/cyd_touch_paint.rs)
//! puts both stages together in a complete program.
#![doc = include_str!("../../../docs/cyd/drawing-strategies.md")]
//! ## Choose a constructor
//!
//! Construction depends on how many [SPI resources](crate#glossary) the CYD
//! should use and whether the application needs touch:
//!
//! - [`CydEsp`] uses two SPI resources: one for the display and one for touch.
//!   Choose it when the board has both resources available. Construction also
//!   loads or runs touch calibration.
//! - [`CydEspOneSpi`] uses one SPI resource for both the display and touch.
//!   Choose it when the board has only one available, or when the application
//!   needs to keep another SPI resource for something else. Device Envoy
//!   coordinates access internally.
//! - [`CydDisplayEsp`] uses one SPI resource for the display and omits touch.
//!   Choose it when the application does not need touch.
//!
//! The [`CydEsp::new`], [`CydEspOneSpi::new`], and [`CydDisplayEsp::new`]
//! examples show the constructor arguments for each choice. After construction,
//! shared application code can use the [`Cyd`] trait without naming an ESP type.
#![doc = include_str!("../../../docs/cyd/implementations.md")]

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
/// Start with the [`cyd`](mod@crate::cyd) module example. The
/// display-only constructor and static-storage pattern are shown by the
/// [`CydDisplayEsp::new`] example.
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
/// Start with the [`cyd`](mod@crate::cyd) module example. Construction
/// is covered by [`CydEsp::new`]; applications then call the calibrated
/// [`CydTouch::read`] operation.
pub struct CydTouchEsp<D = touch_driver::CydTouchSpiDevice> {
    raw: CydTouchUncalibratedEsp<D>,
    calibration_config: CalibrationConfig,
    orientation: Orientation,
}

/// A calibrated CYD-family ESP32 bundle.
///
/// Start with the short [`cyd`](mod@crate::cyd) module example to draw and read
/// touch input. See [`CydEsp::new`] when choosing pins and constructing the
/// hardware; construction performs saved or interactive touch calibration
/// before returning.
pub struct CydEsp {
    /// The owned display component.
    /// See the [`cyd`](mod@crate::cyd) module example.
    pub display: CydDisplayEsp,
    /// The owned calibrated touch component.
    /// See the [`cyd`](mod@crate::cyd) module example.
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
/// `PIXEL_COUNT` is an RGB565 pixel count, not a byte count. Use
/// `CydEsp::SCREEN_PIXELS` for `full_frame_mut`, the pixel count of the largest
/// independently updated rectangle for `frame_mut`, or
/// [`TileGrid`](device_envoy_core::cyd::display::tiling::TileGrid)'s
/// `max_tile_pixel_count()` for `for_each_tile`. If a requested frame
/// or tile is larger than the declared capacity, frame creation panics with
/// the buffer's `view must fit in workspace` assertion; no pixels are silently
/// clipped. `CydStaticEsp<0>` is useful
/// only with immediate operations and contiguous streaming, which do not use a
/// frame buffer.
///
/// The app declares one at file scope and names the workspace pixel count it
/// wants:
///
/// ```rust,no_run
/// #![no_std]
/// #![no_main]
/// use device_envoy_esp::cyd::{CydEsp, CydStaticEsp};
/// static CYD_STATIC: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
/// # use device_envoy_core::cyd::display::tiling::{TileGrid, rectangle_pixel_count};
/// # use embedded_graphics::{prelude::{Point, Size}, primitives::Rectangle};
/// const STATUS_REGION: Rectangle = Rectangle::new(Point::new(0, 0), Size::new(160, 40));
/// const STATUS_PIXELS: usize = rectangle_pixel_count(STATUS_REGION);
/// const GRID: TileGrid = TileGrid::new(
///     Rectangle::new(Point::zero(), Size::new(320, 240)),
///     4,
///     3,
/// );
/// static STATUS_STORAGE: CydStaticEsp<STATUS_PIXELS> = CydEsp::new_static();
/// static TILE_STORAGE: CydStaticEsp<{ GRID.max_tile_pixel_count() }> = CydEsp::new_static();
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
/// operations. This page contains the `CydFrameEsp` example.
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
///     let width = frame.width();
///     let height = frame.height();
///     assert!(width > 0 && height > 0);
///     frame.fill(Rgb565::BLACK).write_text("CYD");
///     let pixels = frame.raw_pixels_mut();
///     assert_eq!(pixels.len(), width * height);
///     frame.flush()
/// }
/// ```
pub struct CydFrameEsp<'a, D: SpiDevice<u8> = display::CydDisplaySpiDevice> {
    display: &'a mut CydDisplayEspDevice<D>,
    view: RegionView<'a>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in logical display coordinates. Drawing coordinates are translated
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
    /// See the [`CydFrameEsp` example](CydFrameEsp).
    pub fn fill(&mut self, color: Rgb565) -> &mut Self {
        self.view.fill(color);
        self
    }

    /// The frame's width in pixels.
    ///
    /// See the [`CydFrameEsp` example](CydFrameEsp).
    #[must_use]
    pub fn width(&self) -> usize {
        self.view.width()
    }

    /// The frame's height in pixels.
    ///
    /// See the [`CydFrameEsp` example](CydFrameEsp).
    #[must_use]
    pub fn height(&self) -> usize {
        self.view.height()
    }

    /// Borrow the frame's raw RGB565 pixels, row-major.
    ///
    /// See the [`CydFrameEsp` example](CydFrameEsp).
    pub fn raw_pixels_mut(&mut self) -> &mut [u16] {
        self.view.raw_pixels_mut()
    }

    /// Present this frame's pixels at its rectangle's top-left (set by
    /// [`CydDisplay::frame_mut`]).
    ///
    /// See the [`CydFrameEsp` example](CydFrameEsp).
    /// This inherent method is the synchronous ESP-specific call. In generic
    /// code, call [`CydFrame::flush`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/display/trait.CydFrame.html#tymethod.flush)
    /// and await its future instead.
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
/// Most applications propagate this error with `?`. Code that reports errors
/// differently by operation can match the preserved source-bearing variants:
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
/// use device_envoy_esp::cyd::Error;
///
/// fn report(error: Error) {
///     match error {
///         Error::ConfigureDisplaySpi(source) | Error::ConfigureTouchSpi(source) => {
///             // Report the SPI configuration details from `source`.
///             drop(source);
///         }
///         Error::InitDisplay => { /* Display initialization failed. */ }
///         Error::FlushFrameBuffer => { /* Sending pixels failed. */ }
///         Error::SetOrientation => { /* Changing orientation failed. */ }
///     }
/// }
/// ```
#[derive(Debug)]
pub enum Error {
    /// Configuring the display SPI peripheral failed.
    /// See the [`Error`] example.
    ConfigureDisplaySpi(esp_hal::spi::master::ConfigError),
    /// The display panel could not be initialized.
    /// See the [`Error`] example.
    InitDisplay,
    /// Configuring the touch SPI peripheral failed.
    /// See the [`Error`] example.
    ConfigureTouchSpi(esp_hal::spi::master::ConfigError),
    /// A frame could not be flushed to the display.
    /// See the [`Error`] example.
    FlushFrameBuffer,
    /// Changing the display orientation failed.
    /// See the [`Error`] example.
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
    /// Zero capacity is intentional when only immediate fills or contiguous
    /// streaming are used; frame-based and tiled drawing need positive storage.
    ///
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// use device_envoy_esp::{Result, cyd::{CydDisplay, CydDisplayEsp, CydEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}};
    /// use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    /// async fn construct(p: esp_hal::peripherals::Peripherals) -> Result<()> {
    ///     static STORAGE: device_envoy_esp::cyd::CydStaticEsp<0> = CydEsp::new_static();
    ///     let display = CydDisplayEsp::new(&STORAGE, p.SPI2, p.GPIO1, p.GPIO2, p.GPIO3,
    ///         p.GPIO4, p.GPIO5, p.GPIO7, p.GPIO8, DEFAULT_DISPLAY_SPI_HZ,
    ///         Orientation::Landscape, Rgb888::BLACK, Rgb888::WHITE, &DEFAULT_FONT)?;
    ///     assert_eq!(display.screen_size(), Orientation::Landscape.size());
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
    /// See the [`CydStaticEsp`] storage example.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticEsp<PIXEL_COUNT> {
        CydStaticEsp::new()
    }

    /// Construct a ready-to-use calibrated CYD, loading or completing calibration internally.
    /// The display arguments use one SPI bus and its SCK/MOSI/MISO/CS/DC/reset/backlight pins;
    /// `touch_spi` and the following touch pins are a separate SPI bus. The flash block stores
    /// calibration, and the button requests interactive recalibration. Use
    /// [`CydEspOneSpi`] when display and touch must share one bus.
    ///
    /// This example focuses on the board-specific construction. For the normal
    /// draw/flush/read loop, start with the [`cyd`](mod@crate::cyd) module
    /// example. For complete startup and wiring in a real program, see the
    /// [checked ESP32 CYD touch-paint example](https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-esp/examples/esp32/generic/cyd_touch_paint.rs).
    ///
    /// ```rust,no_run
    /// #![no_std]
    /// #![no_main]
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    /// # use device_envoy_esp::{Result, button::{ButtonEsp, PressedTo}, cyd::{CydEsp, CydStaticEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}, flash_block::FlashBlockEsp};
    /// # use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// # use esp_hal::spi::master::AnySpi;
    /// async fn construct(mut p: esp_hal::peripherals::Peripherals, touch_spi: AnySpi<'static>) -> Result<()> {
    ///     let [mut calibration_flash] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    ///     let mut recalibration_button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    ///     static STORAGE: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
    ///
    ///     let cyd = CydEsp::new(
    ///         &STORAGE,
    ///         // Display SPI resource and pins:
    ///         p.SPI2, p.GPIO1, p.GPIO2, p.GPIO3, p.GPIO4,
    ///         p.GPIO5, p.GPIO7, p.GPIO8,
    ///         DEFAULT_DISPLAY_SPI_HZ,
    ///         // Presentation:
    ///         Orientation::Landscape,
    ///         Rgb888::BLACK, Rgb888::WHITE, &DEFAULT_FONT,
    ///         // Touch SPI resource and pins:
    ///         touch_spi, p.GPIO9, p.GPIO10, p.GPIO11, p.GPIO12, p.GPIO13,
    ///         // Saved calibration and the recalibration button:
    ///         &mut calibration_flash, &mut recalibration_button,
    ///     ).await?;
    ///
    ///     drop(cyd);
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
