//! A device abstraction for a standalone 320×240 CYD-style SPI display/touch
//! module wired over SPI to a Raspberry Pi Pico (1 or 2).
//!
//! See [`CydRp`], [`CydRpOneSpi`], and [`CydDisplayRp`] for the public
//! constructors; the device-agnostic [`CydDisplay`] and [`CydTouch`] traits live in
//! [`device_envoy_core::cyd`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/).
//! [`CydRp`] uses `SPI0` for the display and `SPI1`
//! for touch; [`CydRpOneSpi`] shares a single SPI peripheral between the two.

// TODO0 Reduce CYD's public API surface; see specs/CYD_PUBLIC_API_CLEANUP.md.

mod buffer;
mod display;
mod one_spi;
mod text;
#[path = "cyd/touch.rs"]
mod touch_driver;

use core::{convert::Infallible, fmt};

use device_envoy_core::button::Button;
use device_envoy_core::cyd::backend;
use device_envoy_core::cyd::{
    SCREEN_PIXELS,
    backend::{CalibrationConfig, RawTouchEvent, TouchUncalibrated},
    display::CydFrame,
    touch::TouchEvent,
};
use device_envoy_core::pixel_target::PixelTarget;
// The device abstraction and its neutral support types live in
// `device-envoy-core::cyd`; re-export the public surface from this device crate.
pub use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{Orientation, tiling},
    touch,
};
use embassy_rp::Peri;
use embassy_rp::gpio::Pin;
use embassy_rp::peripherals::{SPI0, SPI1};
use embassy_rp::spi::{ClkPin, MisoPin, MosiPin};
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
pub use display::DEFAULT_DISPLAY_SPI_HZ;
pub use one_spi::{CydRpOneSpi, CydRpOneSpiStatic};
pub use text::DEFAULT_FONT;
use touch_driver::TOUCH_SPI_HZ;

use crate::flash_block::FlashBlockRp;
use display::CydDisplayRp as CydDisplayRpDevice;
use touch_driver::CydTouchRp as CydTouchRpDevice;

/// An owned CYD display component for RP boards.
///
/// `D` is the underlying `embedded-hal` SPI device type; it defaults to an
/// exclusively-owned SPI peripheral. Shared-bus backends (see
/// [`CydRpOneSpi`]) instantiate this with a shared-bus device instead.
///
/// The display constructor and static-storage pattern are shown by
/// [`CydStaticRp`].
/// The complete-device constructor is shown by [`CydRp::new`].
pub struct CydDisplayRp<D: SpiDevice<u8> = display::CydDisplaySpiDevice> {
    display: CydDisplayRpDevice<D>,
    orientation: Orientation,
    // Every CydRp owns exactly one draw buffer. Apps that don't draw through it
    // pass a zero-sized buffer (e.g. `CydStaticRp<0>`).
    pixel_buffer: &'static mut dyn DynPixelBuffer,
    // Default drawing style. Background clears the device at construction and
    // fills every new frame; foreground color and font drive `CydFrameRp::write_text`.
    // The `Rgb565` versions are precomputed so the hot drawing paths skip the
    // per-call conversion.
    background_color: Rgb888,
    foreground_color: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// An owned uncalibrated CYD touch component for RP boards.
///
/// `D` is the underlying `embedded-hal` SPI device type; see
/// [`CydDisplayRp`] for the shared-bus rationale.
pub(crate) struct CydTouchUncalibratedRp<D = touch_driver::CydTouchSpiDevice> {
    touch: CydTouchRpDevice<D>,
}

/// An owned calibrated CYD touch component for RP boards.
///
/// Construct it as part of [`CydRp::new`]; applications then call the
/// calibrated [`CydTouch::read`]
/// operation.
pub struct CydTouchRp<D = touch_driver::CydTouchSpiDevice> {
    raw: CydTouchUncalibratedRp<D>,
    calibration_config: CalibrationConfig,
    orientation: Orientation,
}

/// A calibrated CYD RP bundle.
///
/// Use [`CydRp::new`] after declaring [`CydStaticRp`] storage; construction
/// performs the saved-or-interactive touch calibration before returning.
pub struct CydRp {
    /// The owned display component.
    pub display: CydDisplayRp,
    /// The owned calibrated touch component.
    pub touch: CydTouchRp,
}

/// An uncalibrated CYD RP bundle.
pub(crate) struct CydRpUncalibrated {
    /// The owned display component.
    pub display: CydDisplayRp,
    /// The owned uncalibrated touch component.
    pub touch: CydTouchUncalibratedRp,
}

/// Static storage for a [`CydRp`]-owned pixel buffer.
///
/// The app declares one at file scope and names the workspace pixel count it
/// wants:
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::cyd::{CydRp, CydStaticRp};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// static CYD_STATIC: CydStaticRp<{ CydRp::SCREEN_PIXELS }> = CydRp::new_static();
/// ```
///
/// The app chooses the pixel count (policy); [`CydDisplayRp::new`] owns the
/// initialization protocol and the storage details.
pub struct CydStaticRp<const PIXEL_COUNT: usize> {
    pixel_buffer: StaticCell<PixelBuffer<PIXEL_COUNT>>,
}

impl<const PIXEL_COUNT: usize> CydStaticRp<PIXEL_COUNT> {
    /// Internal constructor. Apps create storage via [`CydRp::new_static`] so all
    /// construction goes through the `CydRp` device abstraction.
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
/// use device_envoy_rp::cyd::{CydFrameRp, Error};
/// use embedded_hal::spi::SpiDevice;
/// use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// fn use_frame<D: SpiDevice<u8>>(frame: &mut CydFrameRp<'_, D>) -> Result<(), Error> {
///     let _width = frame.width();
///     let _height = frame.height();
///     frame.fill(Rgb565::BLACK).write_text("CYD");
///     let _pixels = frame.raw_pixels_mut();
///     frame.flush()
/// }
/// ```
pub struct CydFrameRp<'a, D: SpiDevice<u8> = display::CydDisplaySpiDevice> {
    display: &'a mut CydDisplayRpDevice<D>,
    view: RegionView<'a>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in screen coordinates. Drawing coordinates are translated
    // by this point before reaching the local frame buffer.
    tile_top_left: Point,
    // Default foreground color and font, copied from the owning `CydDisplayRp`, so
    // `write_text` can render with the device default style.
    pub(crate) background565: Rgb565,
    pub(crate) foreground565: Rgb565,
    pub(crate) font: &'static MonoFont<'static>,
}

impl<'a, D: SpiDevice<u8>> CydFrameRp<'a, D> {
    /// Fill the frame with an explicit color.
    ///
    /// See the [canonical `CydFrameRp` example](CydFrameRp).
    pub fn fill(&mut self, color: Rgb565) -> &mut Self {
        self.view.fill(color);
        self
    }

    /// The frame's width in pixels.
    ///
    /// See the [canonical `CydFrameRp` example](CydFrameRp).
    #[must_use]
    pub fn width(&self) -> usize {
        self.view.width()
    }

    /// The frame's height in pixels.
    ///
    /// See the [canonical `CydFrameRp` example](CydFrameRp).
    #[must_use]
    pub fn height(&self) -> usize {
        self.view.height()
    }

    /// Borrow the frame's raw RGB565 pixels, row-major.
    ///
    /// See the [canonical `CydFrameRp` example](CydFrameRp).
    pub fn raw_pixels_mut(&mut self) -> &mut [u16] {
        self.view.raw_pixels_mut()
    }

    /// Present this frame's pixels at its rectangle's top-left (set by
    /// [`CydDisplay::frame_mut`]).
    ///
    /// See the [canonical `CydFrameRp` example](CydFrameRp).
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

impl<D: SpiDevice<u8>> DrawTarget for CydFrameRp<'_, D> {
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

impl<D: SpiDevice<u8>> Dimensions for CydFrameRp<'_, D> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.tile_top_left, self.view.size())
    }
}

impl<D: SpiDevice<u8>> PixelTarget for CydFrameRp<'_, D> {
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

#[derive(Debug)]
/// Error from a CYD RP display or touch operation.
///
/// See the [`CydRp::new`] constructor example, which propagates this error.
pub enum Error {
    /// The display panel could not be initialized.
    InitDisplay,
    /// A frame could not be flushed to the display.
    FlushFrameBuffer,
    /// Changing the display orientation failed.
    SetOrientation,
}

impl<D: SpiDevice<u8>> CydDisplayRp<D> {
    fn set_orientation(&mut self, orientation: Orientation) -> Result<(), Error> {
        self.display.set_orientation(orientation)?;
        self.orientation = orientation;
        Ok(())
    }

    fn from_display_device(
        mut display: CydDisplayRpDevice<D>,
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
    /// Used by shared-bus backends (see [`CydRpOneSpi`]) that build their own
    /// `SpiDevice` instead of owning an exclusive SPI peripheral.
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub(crate) fn new_from_device<Dc, Rst, Backlight>(
        spi_device: D,
        dc_pin: Peri<'static, Dc>,
        rst_pin: Peri<'static, Rst>,
        backlight_pin: Peri<'static, Backlight>,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        pixel_buffer: &'static mut dyn DynPixelBuffer,
    ) -> Result<Self, Error>
    where
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
    {
        let display = CydDisplayRpDevice::new_from_device(
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

impl CydDisplayRp<display::CydDisplaySpiDevice> {
    /// Construct a display-only `CydDisplayRp` that owns its draw buffer.
    ///
    /// ```rust,no_run
    /// #![no_std]
    /// #![no_main]
    /// use device_envoy_rp::{Result, cyd::{CydDisplayRp, CydRp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}};
    /// use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    /// async fn construct(p: embassy_rp::Peripherals) -> Result<()> {
    ///     static STORAGE: device_envoy_rp::cyd::CydStaticRp<0> = CydRp::new_static();
    ///     let _display = CydDisplayRp::new(&STORAGE, p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16,
    ///         p.PIN_17, p.PIN_20, p.PIN_21, p.PIN_22, DEFAULT_DISPLAY_SPI_HZ,
    ///         Orientation::Landscape, Rgb888::BLACK, Rgb888::WHITE, &DEFAULT_FONT)?;
    ///     Ok(())
    /// }
    /// ```
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors CydEsp's constructor shape"
    )]
    pub fn new<const PIXEL_COUNT: usize, Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        statics: &'static CydStaticRp<PIXEL_COUNT>,
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<Self, Error>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
    {
        let pixel_buffer = PixelBuffer::init_static(&statics.pixel_buffer);
        let display = CydDisplayRpDevice::new(
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

impl<D: SpiDevice<u8>> CydTouchUncalibratedRp<D> {
    /// Construct an uncalibrated touch component from an already-built SPI device.
    ///
    /// Used by shared-bus backends (see [`CydRpOneSpi`]) that build their own
    /// `SpiDevice` instead of owning an exclusive SPI peripheral.
    pub(crate) fn from_device<Irq: Pin>(
        touch_spi_device: D,
        touch_irq_pin: Peri<'static, Irq>,
    ) -> Self {
        Self {
            touch: CydTouchRpDevice::from_device(touch_spi_device, touch_irq_pin),
        }
    }
}

impl CydTouchUncalibratedRp<touch_driver::CydTouchSpiDevice> {
    /// Construct an uncalibrated touch component.
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub(crate) fn new<TouchSck, TouchMosi, TouchMiso, TouchCs, TouchIrq>(
        touch_spi: Peri<'static, SPI1>,
        touch_sck_pin: Peri<'static, TouchSck>,
        touch_mosi_pin: Peri<'static, TouchMosi>,
        touch_miso_pin: Peri<'static, TouchMiso>,
        touch_cs_pin: Peri<'static, TouchCs>,
        touch_irq_pin: Peri<'static, TouchIrq>,
    ) -> Result<Self, Error>
    where
        TouchSck: Pin + ClkPin<SPI1>,
        TouchMosi: Pin + MosiPin<SPI1>,
        TouchMiso: Pin + MisoPin<SPI1>,
        TouchCs: Pin,
        TouchIrq: Pin,
    {
        Ok(Self {
            touch: CydTouchRpDevice::new(
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

impl CydRp {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    ///
    /// Used by the [`CydStaticRp`] storage example.
    pub const SCREEN_PIXELS: usize = SCREEN_PIXELS;

    /// Create [`CydStaticRp`] storage for a `PIXEL_COUNT`-sized draw buffer.
    ///
    /// See the [`CydStaticRp`] example.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticRp<PIXEL_COUNT> {
        CydStaticRp::new()
    }

    /// Construct a calibrated CYD bundle using the saved-or-interactive calibration flow.
    ///
    /// ```rust,no_run
    /// #![no_std]
    /// #![no_main]
    /// use device_envoy_rp::{Result, button::{ButtonRp, PressedTo}, cyd::{CydRp, CydStaticRp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}, flash_block::FlashBlockRp};
    /// use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    ///
    /// async fn construct(p: embassy_rp::Peripherals) -> Result<()> {
    ///     let [mut flash] = FlashBlockRp::new_array::<1>(p.FLASH)?;
    ///     let mut button = ButtonRp::new(p.PIN_15, PressedTo::Ground);
    ///     static STORAGE: CydStaticRp<{ CydRp::SCREEN_PIXELS }> = CydRp::new_static();
    ///     let _cyd = CydRp::new(&STORAGE, p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.PIN_17,
    ///         p.PIN_20, p.PIN_21, p.PIN_22, DEFAULT_DISPLAY_SPI_HZ, Orientation::Landscape,
    ///         Rgb888::BLACK, Rgb888::WHITE, &DEFAULT_FONT, p.SPI1, p.PIN_10, p.PIN_11,
    ///         p.PIN_12, p.PIN_13, p.PIN_14, &mut flash, &mut button).await?;
    ///     Ok(())
    /// }
    /// ```
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub async fn new<
        const PIXEL_COUNT: usize,
        Sck,
        Mosi,
        Miso,
        Cs,
        Dc,
        Rst,
        Backlight,
        TouchSck,
        TouchMosi,
        TouchMiso,
        TouchCs,
        TouchIrq,
    >(
        statics: &'static CydStaticRp<PIXEL_COUNT>,
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: Peri<'static, SPI1>,
        touch_sck_pin: Peri<'static, TouchSck>,
        touch_mosi_pin: Peri<'static, TouchMosi>,
        touch_miso_pin: Peri<'static, TouchMiso>,
        touch_cs_pin: Peri<'static, TouchCs>,
        touch_irq_pin: Peri<'static, TouchIrq>,
        calibration_flash_block: &mut FlashBlockRp,
        recalibration_button: &mut impl Button,
    ) -> crate::Result<Self>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
        TouchSck: Pin + ClkPin<SPI1>,
        TouchMosi: Pin + MosiPin<SPI1>,
        TouchMiso: Pin + MisoPin<SPI1>,
        TouchCs: Pin,
        TouchIrq: Pin,
    {
        let CydRpUncalibrated { mut display, touch } = CydRpUncalibrated::new(
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

impl Cyd for CydRp {
    type Error = Error;
    type Display = CydDisplayRp;
    type Touch = CydTouchRp;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }

    fn orientation(&self) -> Orientation {
        self.display.orientation
    }
}

impl CydRpUncalibrated {
    /// Construct an uncalibrated CYD bundle from display and touch parts.
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub(crate) fn new<
        const PIXEL_COUNT: usize,
        Sck,
        Mosi,
        Miso,
        Cs,
        Dc,
        Rst,
        Backlight,
        TouchSck,
        TouchMosi,
        TouchMiso,
        TouchCs,
        TouchIrq,
    >(
        statics: &'static CydStaticRp<PIXEL_COUNT>,
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
        _orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: Peri<'static, SPI1>,
        touch_sck_pin: Peri<'static, TouchSck>,
        touch_mosi_pin: Peri<'static, TouchMosi>,
        touch_miso_pin: Peri<'static, TouchMiso>,
        touch_cs_pin: Peri<'static, TouchCs>,
        touch_irq_pin: Peri<'static, TouchIrq>,
    ) -> Result<Self, Error>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
        TouchSck: Pin + ClkPin<SPI1>,
        TouchMosi: Pin + MosiPin<SPI1>,
        TouchMiso: Pin + MisoPin<SPI1>,
        TouchCs: Pin,
        TouchIrq: Pin,
    {
        Ok(Self {
            display: CydDisplayRp::new(
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
            touch: CydTouchUncalibratedRp::new(
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

impl<D: SpiDevice<u8>> fmt::Debug for CydDisplayRp<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydDisplayRp")
            .finish_non_exhaustive()
    }
}

impl<D> fmt::Debug for CydTouchUncalibratedRp<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchUncalibratedRp")
            .finish_non_exhaustive()
    }
}

impl<D> fmt::Debug for CydTouchRp<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchRp")
            .field("calibration_config", &self.calibration_config)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydRp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CydRp").finish_non_exhaustive()
    }
}

impl fmt::Debug for CydRpUncalibrated {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydRpUncalibrated")
            .finish_non_exhaustive()
    }
}

impl<D: SpiDevice<u8>> backend::DisplayBackend for CydDisplayRp<D> {
    type Error = Error;
    type Frame<'a>
        = CydFrameRp<'a, D>
    where
        Self: 'a;

    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> Self::Frame<'_> {
        self.display.make_frame_with_tile_top_left(
            &mut *self.pixel_buffer,
            rectangle,
            tile_top_left,
            self.background565,
            self.foreground565,
            self.font,
        )
    }
}

impl<D: SpiDevice<u8>> CydDisplay for CydDisplayRp<D> {
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

impl<D: SpiDevice<u8>> TouchUncalibrated for CydTouchUncalibratedRp<D> {
    type Error = Error;
    type Calibrated = CydTouchRp<D>;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
        Ok(self.touch.read_raw_touch_event())
    }

    fn calibrate(
        self,
        calibration_config: CalibrationConfig,
        orientation: Orientation,
    ) -> Self::Calibrated {
        CydTouchRp {
            raw: self,
            calibration_config,
            orientation,
        }
    }
}

impl<D: SpiDevice<u8>> CydTouch for CydTouchRp<D> {
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

impl<D: SpiDevice<u8>> CydFrame for CydFrameRp<'_, D> {
    type Error = Error;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        CydFrameRp::fill(self, color)
    }

    fn clear(&mut self) -> &mut Self {
        self.fill(self.background565)
    }

    fn write_text(&mut self, text: &str) -> &mut Self {
        CydFrameRp::write_text(self, text)
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
        CydFrameRp::flush(self)
    }
}
