use core::fmt::Write;

use crate::button::Button;
use crate::flash_block::FlashBlock;
#[cfg(not(test))]
use embassy_time::Timer;
use heapless::String;

use super::super::CydDisplay;
use super::calibration::{
    CALIBRATION_MAX_DRAW_ITEMS, CALIBRATION_TEXT_RECTANGLE, CalibrationConfig, CalibrationCorner,
    VERIFY_HIT_RADIUS_PIXELS, calibration_ack_dot_item, calibration_rejected_target_items,
    calibration_target_items, calibration_verify_target_center, calibration_verify_target_items,
    validate_calibration_points,
};
use super::flow::CalibrationFlow;
use super::flow::CalibrationFlowEvent;
use super::flow::{ReleaseTouchCapture, ReleaseTouchCaptureEvent};
use crate::cyd::backend::TouchUncalibrated;
use crate::cyd::display::{CydFrame, DrawItem};
use crate::cyd::{SCREEN_HEIGHT, SCREEN_WIDTH};
use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

pub const CAPTURE_ACK_FRAME_COUNT: usize = 8;
pub const REJECTED_FRAME_COUNT: usize = 30;
pub const MAX_RAW_EVENTS_PER_FRAME: usize = 64;
const VERIFY_TIMEOUT_SECONDS: usize = 10;
// Verification is paced below so this frame budget remains a real-time
// timeout even when the display only needs to redraw a small text rectangle.
const CALIBRATION_DRAW_FRAMES_PER_SECOND: usize = 10;
pub const VERIFY_TIMEOUT_FRAMES: usize =
    VERIFY_TIMEOUT_SECONDS * CALIBRATION_DRAW_FRAMES_PER_SECOND;

/// Bounds for the target/dot geometry, streamed buffer-free via
/// [`CydDisplay::draw_items`]. Covers the whole screen so every redraw
/// erases any stale shape from the previous state before drawing the
/// current one; the text banner is drawn afterward so it always wins the
/// small overlap at the bottom of the screen.
const CALIBRATION_SHAPES_RECTANGLE: Rectangle = Rectangle::new(
    Point::zero(),
    Size::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32),
);

/// Error from the shared calibration driver.
#[derive(Debug)]
pub enum Error<DeviceError, FlashError> {
    /// Reading or drawing through the platform touch/display backend failed.
    Device(DeviceError),
    /// Loading or saving calibration in persistent storage failed.
    Flash(FlashError),
}

#[derive(Clone, Copy, Debug)]
/// Tunable frame-budget settings for the shared calibration flow.
struct EnsureCalibrationSettings {
    verify_timeout_frames: usize,
}

impl EnsureCalibrationSettings {
    const DEFAULT: Self = Self {
        verify_timeout_frames: VERIFY_TIMEOUT_FRAMES,
    };

    #[must_use]
    const fn verify_timeout_frames(self) -> usize {
        self.verify_timeout_frames
    }
}

enum CalibrationDriverState {
    Capturing,
    ShowCaptured {
        calibration_corner: CalibrationCorner,
        frames_remaining: usize,
    },
    ShowRejected {
        worst_residual_pixels: Option<f32>,
        frames_remaining: usize,
    },
    Verifying {
        candidate_config: CalibrationConfig,
        release_touch_capture: ReleaseTouchCapture,
        polls_remaining: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CalibrationShape {
    Capturing(Option<CalibrationCorner>),
    ShowCaptured {
        calibration_corner: CalibrationCorner,
        next_corner: Option<CalibrationCorner>,
    },
    ShowRejected(Option<CalibrationCorner>),
    Verifying,
}

/// Ensure that `touch` has a calibration, running the shared four-tap flow when
/// the flash block does not currently deserialize as a valid configuration.
///
/// Invalid, corrupt, or absent flash content is treated as "not calibrated"
/// instead of bricking boot. The driver simply reruns the calibration flow and
/// overwrites the block with a fresh solve after the candidate is validated and
/// the user confirms it by hitting the center verify target, then returns so
/// the caller can proceed immediately.
///
/// ```rust,no_run
/// # use core::{convert::Infallible, future::ready};
/// # use device_envoy_core::{
/// #     button::{Button, __ButtonMonitor},
/// #     cyd::{CydDisplay, CydTouch, display::CydFrame, backend::{CalibrationConfig, RawTouchEvent, TouchUncalibrated, ensure_calibration}},
/// #     flash_block::FlashBlock,
/// #     pixel_target::PixelTarget,
/// # };
/// # use embedded_graphics::{
/// #     pixelcolor::{Rgb565, Rgb888},
/// #     prelude::{DrawTarget, OriginDimensions, Point, RgbColor, Size},
/// #     primitives::Rectangle,
/// # };
/// # use serde::{Deserialize, Serialize};
/// # struct DemoDisplay;
/// # struct DemoFrame;
/// # struct DemoTouch;
/// # struct DemoTouchUncalibrated;
/// # struct DemoFlashBlock {
/// #     calibration_config: Option<CalibrationConfig>,
/// # }
/// # struct DemoButton;
/// # impl CydTouch for DemoTouch {
/// #     type Error = Infallible;
/// #     fn read(&mut self) -> Result<Option<device_envoy_core::cyd::touch::TouchEvent>, Self::Error> { Ok(None) }
/// # }
/// # impl TouchUncalibrated for DemoTouchUncalibrated {
/// #     type Error = Infallible;
/// #     type Calibrated = DemoTouch;
/// #     fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
/// #         Ok(None)
/// #     }
/// #     fn calibrate(self, _calibration_config: CalibrationConfig) -> Self::Calibrated {
/// #         DemoTouch
/// #     }
/// # }
/// # impl CydDisplay for DemoDisplay {
/// #     type Error = Infallible;
/// #     type Frame<'a> = DemoFrame;
/// #     fn screen_size(&self) -> Size { Size::new(320, 240) }
/// #     fn background_color(&self) -> Rgb888 { Rgb888::BLACK }
/// #     fn foreground_color(&self) -> Rgb888 { Rgb888::WHITE }
/// #     fn background_565(&self) -> Rgb565 { Rgb565::BLACK }
/// #     fn foreground_565(&self) -> Rgb565 { Rgb565::WHITE }
/// #     fn frame_mut_with_tile_top_left(
/// #         &mut self,
/// #         _rectangle: Rectangle,
/// #         _tile_top_left: Point,
/// #     ) -> Self::Frame<'_> {
/// #         DemoFrame
/// #     }
/// #     fn fill_rectangle(
/// #         &mut self,
/// #         _rectangle: Rectangle,
/// #         _color: Rgb565,
/// #     ) -> Result<(), Self::Error> {
/// #         Ok(())
/// #     }
/// #     fn fill_contiguous<I>(
/// #         &mut self,
/// #         _rectangle: Rectangle,
/// #         _pixels: I,
/// #     ) -> Result<(), Self::Error>
/// #     where
/// #         I: IntoIterator<Item = Rgb565>,
/// #     {
/// #         Ok(())
/// #     }
/// # }
/// # impl DrawTarget for DemoFrame {
/// #     type Color = Rgb565;
/// #     type Error = Infallible;
/// #     fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
/// #     where
/// #         I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
/// #     {
/// #         Ok(())
/// #     }
/// # }
/// # impl OriginDimensions for DemoFrame {
/// #     fn size(&self) -> Size { Size::new(320, 240) }
/// # }
/// # impl PixelTarget for DemoFrame {
/// #     fn width(&self) -> usize { 320 }
/// #     fn height(&self) -> usize { 240 }
/// #     fn put_pixel(&mut self, _x: usize, _y: usize, _color: Rgb888) {}
/// # }
/// # impl CydFrame for DemoFrame {
/// #     type Error = Infallible;
/// #     fn rectangle(&self) -> Rectangle {
/// #         Rectangle::new(Point::zero(), Size::new(320, 240))
/// #     }
/// #     fn fill(&mut self, _color: Rgb565) -> &mut Self { self }
/// #     fn clear(&mut self) -> &mut Self { self }
/// #     fn write_text(&mut self, _text: &str) -> &mut Self { self }
/// #     fn copy_from_565(
/// #         &mut self,
/// #         _src: &[u16],
/// #     ) -> device_envoy_core::Result<()> {
/// #         Ok(())
/// #     }
/// #     fn flush(
/// #         &mut self,
/// #     ) -> impl core::future::Future<Output = Result<(), <Self as CydFrame>::Error>> {
/// #         ready(Ok(()))
/// #     }
/// # }
/// # impl FlashBlock for DemoFlashBlock {
/// #     type Error = Infallible;
/// #     fn load<T>(&mut self) -> Result<Option<T>, Self::Error>
/// #     where
/// #         T: Serialize + for<'de> Deserialize<'de>,
/// #     {
/// #         Ok(None)
/// #     }
/// #     fn save<T>(&mut self, _value: &T) -> Result<(), Self::Error>
/// #     where
/// #         T: Serialize + for<'de> Deserialize<'de>,
/// #     {
/// #         Ok(())
/// #     }
/// #     fn clear(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # impl __ButtonMonitor for DemoButton {
/// #     fn is_pressed_raw(&self) -> bool { false }
/// #     async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
/// # }
/// # impl Button for DemoButton {}
/// # async fn demo() -> Result<(), device_envoy_core::cyd::backend::Error<Infallible, Infallible>> {
/// let mut display = DemoDisplay;
/// let touch = DemoTouchUncalibrated;
/// let mut calibration_flash_block = DemoFlashBlock {
///     calibration_config: None,
/// };
/// let mut recalibration_button = DemoButton;
/// let _touch = ensure_calibration(
///     &mut display,
///     touch,
///     &mut calibration_flash_block,
///     &mut recalibration_button,
///     Some("Touch calibrated"),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn ensure_calibration<D, T, F, R>(
    display: &mut D,
    touch: T,
    calibration_flash_block: &mut F,
    recalibration_button: &mut R,
    confirmed_message: Option<&str>,
) -> Result<T::Calibrated, Error<D::Error, F::Error>>
where
    D: CydDisplay,
    T: TouchUncalibrated<Error = D::Error>,
    F: FlashBlock,
    R: Button,
{
    ensure_calibration_with_settings(
        display,
        touch,
        calibration_flash_block,
        recalibration_button,
        confirmed_message,
        EnsureCalibrationSettings::DEFAULT,
    )
    .await
}

async fn ensure_calibration_with_settings<D, T, F, R>(
    display: &mut D,
    mut touch: T,
    calibration_flash_block: &mut F,
    recalibration_button: &mut R,
    confirmed_message: Option<&str>,
    ensure_calibration_settings: EnsureCalibrationSettings,
) -> Result<T::Calibrated, Error<D::Error, F::Error>>
where
    D: CydDisplay,
    T: TouchUncalibrated<Error = D::Error>,
    F: FlashBlock,
    R: Button,
{
    if let Some(calibration_config) = calibration_flash_block
        .load::<CalibrationConfig>()
        .unwrap_or(None)
    {
        return Ok(touch.calibrate(calibration_config));
    }

    let mut calibration_flow = CalibrationFlow::new();
    let mut calibration_driver_state = CalibrationDriverState::Capturing;
    let mut last_calibration_shape = None;
    let mut calibration_button_released = true;

    loop {
        // A plain Button is intentional here: this loop does synchronous
        // per-frame polling, not cancelable button futures, so ButtonWatch
        // would add an ESP-only dependency without buying correctness.
        let calibration_button_pressed = recalibration_button.is_pressed();
        if calibration_button_pressed && calibration_button_released {
            calibration_flow.restart();
            calibration_driver_state = CalibrationDriverState::Capturing;
        }
        calibration_button_released = !calibration_button_pressed;

        let mut saw_idle = false;
        for _raw_event_index in 0..MAX_RAW_EVENTS_PER_FRAME {
            let raw_touch_event = match touch.read_raw_touch_event() {
                Ok(raw_touch_event) => raw_touch_event,
                Err(error) => {
                    return Err(Error::Device(error));
                }
            };
            let Some(raw_touch_event) = raw_touch_event else {
                saw_idle = true;
                break;
            };

            match &mut calibration_driver_state {
                CalibrationDriverState::Capturing => {
                    let Some(calibration_flow_event) =
                        calibration_flow.handle_raw_touch_event(Some(raw_touch_event))
                    else {
                        continue;
                    };

                    match calibration_flow_event {
                        CalibrationFlowEvent::PointCaptured {
                            calibration_corner,
                            raw_point: _raw_point,
                            next_corner: _next_corner,
                            usable_sample_count: _usable_sample_count,
                        } => {
                            calibration_driver_state = CalibrationDriverState::ShowCaptured {
                                calibration_corner,
                                frames_remaining: CAPTURE_ACK_FRAME_COUNT,
                            };
                        }
                        CalibrationFlowEvent::Completed {
                            raw_points,
                            calibration_corner: _calibration_corner,
                            usable_sample_count: _usable_sample_count,
                        } => match validate_calibration_points(raw_points) {
                            Ok(calibration_validation) => {
                                calibration_driver_state = CalibrationDriverState::Verifying {
                                    candidate_config: calibration_validation.calibration_config(),
                                    release_touch_capture: ReleaseTouchCapture::new(),
                                    polls_remaining: ensure_calibration_settings
                                        .verify_timeout_frames(),
                                };
                            }
                            Err(crate::Error::CalibrationResidualTooLarge {
                                worst_residual_pixels,
                            }) => {
                                calibration_flow.restart();
                                calibration_driver_state = CalibrationDriverState::ShowRejected {
                                    worst_residual_pixels: Some(worst_residual_pixels),
                                    frames_remaining: REJECTED_FRAME_COUNT,
                                };
                            }
                            Err(crate::Error::CalibrationDegenerateGeometry) => {
                                calibration_flow.restart();
                                calibration_driver_state = CalibrationDriverState::ShowRejected {
                                    worst_residual_pixels: None,
                                    frames_remaining: REJECTED_FRAME_COUNT,
                                };
                            }
                            Err(_) => {
                                calibration_flow.restart();
                                calibration_driver_state = CalibrationDriverState::ShowRejected {
                                    worst_residual_pixels: None,
                                    frames_remaining: REJECTED_FRAME_COUNT,
                                };
                            }
                        },
                    }
                }
                CalibrationDriverState::Verifying {
                    candidate_config,
                    release_touch_capture,
                    ..
                } => {
                    let Some(ReleaseTouchCaptureEvent::Captured { raw_point, .. }) =
                        release_touch_capture.handle_raw_touch_event(Some(raw_touch_event))
                    else {
                        continue;
                    };
                    let (mapped_x, mapped_y) =
                        candidate_config.map_raw_to_screen(raw_point.x, raw_point.y);
                    if hit_verify_target(mapped_x, mapped_y) {
                        if let Some(confirmed_message) = confirmed_message
                            && let Err(error) =
                                draw_message_screen(display, confirmed_message).await
                        {
                            return Err(Error::Device(error));
                        }
                        if let Err(error) = calibration_flash_block.save(candidate_config) {
                            return Err(Error::Flash(error));
                        }
                        let calibration_config = *candidate_config;
                        return Ok(touch.calibrate(calibration_config));
                    } else {
                        calibration_flow.restart();
                        calibration_driver_state = CalibrationDriverState::ShowRejected {
                            worst_residual_pixels: None,
                            frames_remaining: REJECTED_FRAME_COUNT,
                        };
                    }
                }
                CalibrationDriverState::ShowCaptured { .. }
                | CalibrationDriverState::ShowRejected { .. } => {}
            }
        }

        if saw_idle {
            advance_driver_state_after_idle(&mut calibration_flow, &mut calibration_driver_state);
        }

        if let Err(error) = draw_calibration_screen(
            display,
            &calibration_flow,
            &calibration_driver_state,
            &mut last_calibration_shape,
        )
        .await
        {
            return Err(Error::Device(error));
        }

        if saw_idle {
            pace_verification_frame(&calibration_driver_state).await;
        }
    }
}

#[cfg(not(test))]
async fn pace_verification_frame(calibration_driver_state: &CalibrationDriverState) {
    // The memory-backed tests intentionally use frame counts without waiting
    // for wall-clock time. Hardware and browser builds need the pause because
    // the optimized redraw path can otherwise consume all timeout frames in a
    // few milliseconds.
    if matches!(
        calibration_driver_state,
        CalibrationDriverState::Verifying { .. }
    ) {
        Timer::after_millis(100).await;
    }
}

#[cfg(test)]
async fn pace_verification_frame(_calibration_driver_state: &CalibrationDriverState) {}

async fn draw_message_screen<D>(display: &mut D, message: &str) -> Result<(), D::Error>
where
    D: CydDisplay,
{
    // Erase any leftover target geometry from the redraw just before this
    // one; buffer-free, so it costs nothing beyond the SPI transfer itself.
    display.clear()?;
    display
        .frame_mut(CALIBRATION_TEXT_RECTANGLE)
        .write_text(message)
        .flush()
        .await
}

fn advance_driver_state_after_idle(
    calibration_flow: &mut CalibrationFlow,
    calibration_driver_state: &mut CalibrationDriverState,
) {
    match calibration_driver_state {
        CalibrationDriverState::Capturing => {
            calibration_flow.handle_raw_touch_event(None);
        }
        CalibrationDriverState::ShowCaptured {
            frames_remaining, ..
        } => {
            if *frames_remaining > 0 {
                *frames_remaining -= 1;
            }
            if *frames_remaining == 0 {
                *calibration_driver_state = CalibrationDriverState::Capturing;
            }
        }
        CalibrationDriverState::ShowRejected {
            frames_remaining, ..
        } => {
            if *frames_remaining > 0 {
                *frames_remaining -= 1;
            }
            if *frames_remaining == 0 {
                *calibration_driver_state = CalibrationDriverState::Capturing;
            }
        }
        CalibrationDriverState::Verifying {
            release_touch_capture,
            polls_remaining,
            ..
        } => {
            release_touch_capture.handle_raw_touch_event(None);
            if *polls_remaining > 0 {
                *polls_remaining -= 1;
            }
            if *polls_remaining == 0 {
                calibration_flow.restart();
                *calibration_driver_state = CalibrationDriverState::ShowRejected {
                    worst_residual_pixels: None,
                    frames_remaining: REJECTED_FRAME_COUNT,
                };
            }
        }
    }
}

async fn draw_calibration_screen<D>(
    display: &mut D,
    calibration_flow: &CalibrationFlow,
    calibration_driver_state: &CalibrationDriverState,
    last_calibration_shape: &mut Option<CalibrationShape>,
) -> Result<(), D::Error>
where
    D: CydDisplay,
{
    let mut shape_items: heapless::Vec<DrawItem, CALIBRATION_MAX_DRAW_ITEMS> = heapless::Vec::new();
    let mut message = String::<48>::new();
    let calibration_shape;

    match calibration_driver_state {
        CalibrationDriverState::Capturing => {
            let next_corner = calibration_flow.next_corner();
            calibration_shape = CalibrationShape::Capturing(next_corner);
            if let Some(calibration_corner) = next_corner {
                push_calibration_items(
                    &mut shape_items,
                    calibration_target_items(calibration_corner),
                );
            }
            push_calibration_message(&mut message, "Tap target, then lift");
        }
        CalibrationDriverState::ShowCaptured {
            calibration_corner, ..
        } => {
            let next_corner = calibration_flow.next_corner();
            calibration_shape = CalibrationShape::ShowCaptured {
                calibration_corner: *calibration_corner,
                next_corner,
            };
            push_calibration_items(
                &mut shape_items,
                [calibration_ack_dot_item(*calibration_corner)],
            );
            if let Some(next_corner) = next_corner {
                push_calibration_items(&mut shape_items, calibration_target_items(next_corner));
            }
            push_calibration_message(&mut message, "Corner captured");
        }
        CalibrationDriverState::ShowRejected {
            worst_residual_pixels,
            ..
        } => {
            let next_corner = calibration_flow.next_corner();
            calibration_shape = CalibrationShape::ShowRejected(next_corner);
            if let Some(calibration_corner) = next_corner {
                push_calibration_items(
                    &mut shape_items,
                    calibration_rejected_target_items(calibration_corner),
                );
            }
            match worst_residual_pixels {
                Some(worst_residual_pixels) => {
                    match write!(&mut message, "Try again ({worst_residual_pixels:.1}px)") {
                        Ok(()) => {}
                        Err(_) => unreachable!("heapless message capacity is sufficient"),
                    }
                }
                None => push_calibration_message(&mut message, "Try again"),
            }
        }
        CalibrationDriverState::Verifying { .. } => {
            calibration_shape = CalibrationShape::Verifying;
            push_calibration_items(&mut shape_items, calibration_verify_target_items());
            push_calibration_message(&mut message, "Tap center to save");
        }
    }

    // Buffer-free: stream the shape only when it changes. Re-streaming a
    // full-screen background on every polling iteration makes the panel flash
    // between the background and the text frame.
    if *last_calibration_shape != Some(calibration_shape) {
        let background = display.background_565();
        display.draw_items::<CALIBRATION_MAX_DRAW_ITEMS>(
            CALIBRATION_SHAPES_RECTANGLE,
            background,
            shape_items,
        )?;
        *last_calibration_shape = Some(calibration_shape);
    }

    // The one buffered flush per redraw: small (`CALIBRATION_TEXT_RECTANGLE`
    // is `CALIBRATION_MIN_PIXEL_COUNT` pixels, not the full screen), drawn
    // after the shapes so it always wins the small overlap at the bottom.
    display
        .frame_mut(CALIBRATION_TEXT_RECTANGLE)
        .write_text(message.as_str())
        .flush()
        .await
}

fn push_calibration_items<const N: usize, const M: usize>(
    shape_items: &mut heapless::Vec<DrawItem, N>,
    items: [DrawItem; M],
) {
    for item in items {
        shape_items
            .push(item)
            .expect("calibration draw items fit CALIBRATION_MAX_DRAW_ITEMS");
    }
}

fn push_calibration_message(message: &mut String<48>, text: &str) {
    message
        .push_str(text)
        .expect("calibration message fits fixed buffer");
}

fn hit_verify_target(mapped_x: f32, mapped_y: f32) -> bool {
    let verify_target_center = calibration_verify_target_center();
    let delta_x = mapped_x - verify_target_center.x as f32;
    let delta_y = mapped_y - verify_target_center.y as f32;
    delta_x * delta_x + delta_y * delta_y <= VERIFY_HIT_RADIUS_PIXELS * VERIFY_HIT_RADIUS_PIXELS
}
