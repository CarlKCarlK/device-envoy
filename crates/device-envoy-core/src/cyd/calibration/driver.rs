use core::fmt::Write;

use crate::button::Button;
use crate::flash_block::FlashBlock;
use heapless::String;

use super::super::{Cyd, CydDisplay, CydFrame};

use super::{
    CalibrationConfig, CalibrationCorner, CalibrationFlow, CalibrationSolveError, CydRawTouch,
    draw_calibration_ack_dot, draw_calibration_cross, draw_calibration_instruction,
    draw_calibration_rejected_cross, draw_calibration_verify_target,
    flow::CalibrationFlowEvent,
    flow::{ReleaseTouchCapture, ReleaseTouchCaptureEvent},
    validate_calibration_points,
};

pub const CAPTURE_ACK_FRAME_COUNT: usize = 8;
pub const REJECTED_FRAME_COUNT: usize = 30;
pub const MAX_RAW_EVENTS_PER_FRAME: usize = 64;
const VERIFY_TIMEOUT_SECONDS: usize = 10;
// The ESP classic CYD currently redraws the full calibration screen at about
// 10 fps, so 100 drawn frames is roughly the intended 10-second timeout.
const CALIBRATION_DRAW_FRAMES_PER_SECOND: usize = 10;
pub const VERIFY_TIMEOUT_FRAMES: usize =
    VERIFY_TIMEOUT_SECONDS * CALIBRATION_DRAW_FRAMES_PER_SECOND;

/// Result of ensuring calibration at startup.
#[derive(Clone, Copy, Debug)]
pub enum EnsureCalibrationOutcome {
    Loaded(CalibrationConfig),
    Saved(CalibrationConfig),
}

impl EnsureCalibrationOutcome {
    #[must_use]
    pub const fn calibration_config(self) -> CalibrationConfig {
        match self {
            Self::Loaded(calibration_config) | Self::Saved(calibration_config) => {
                calibration_config
            }
        }
    }

    #[must_use]
    pub const fn was_saved(self) -> bool {
        matches!(self, Self::Saved(_))
    }
}

/// Error from the shared calibration driver.
#[derive(Debug)]
pub enum EnsureCalibrationError<DeviceError, FlashError> {
    Device(DeviceError),
    Flash(FlashError),
}

#[derive(Clone, Copy, Debug)]
/// Tunable frame-budget settings for the shared calibration flow.
pub struct EnsureCalibrationSettings {
    verify_timeout_frames: usize,
}

impl EnsureCalibrationSettings {
    pub const DEFAULT: Self = Self {
        verify_timeout_frames: VERIFY_TIMEOUT_FRAMES,
    };

    #[must_use]
    pub const fn new(verify_timeout_frames: usize) -> Self {
        assert!(verify_timeout_frames > 0, "verify timeout must be non-zero");
        Self {
            verify_timeout_frames,
        }
    }

    #[must_use]
    pub const fn verify_timeout_frames(self) -> usize {
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

/// Ensure that `cyd` has a calibration, running the shared four-tap flow when
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
/// #     cyd::{
/// #         Cyd, CydDisplay, CydFrame, CydInfallibleError, CydTouch, TouchEvent,
/// #         calibration::{CalibrationConfig, CydRawTouch, RawTouchEvent},
/// #         ensure_calibration,
/// #     },
/// #     flash_block::FlashBlock,
/// #     pixel_target::PixelTarget,
/// # };
/// # use embedded_graphics::{
/// #     pixelcolor::{Rgb565, Rgb888},
/// #     prelude::{DrawTarget, OriginDimensions, Point, RgbColor, Size},
/// #     primitives::Rectangle,
/// # };
/// # use serde::{Deserialize, Serialize};
/// # struct DemoCyd;
/// # struct DemoDisplay;
/// # struct DemoTouch;
/// # struct DemoFrame;
/// # struct DemoFlashBlock {
/// #     calibration_config: Option<CalibrationConfig>,
/// # }
/// # struct DemoButton;
/// # impl Cyd for DemoCyd {
/// #     type Error = CydInfallibleError;
/// #     type Display<'a> = DemoDisplay;
/// #     type Touch<'a> = DemoTouch;
/// #     fn parts(&mut self) -> (Self::Display<'_>, Self::Touch<'_>) {
/// #         (DemoDisplay, DemoTouch)
/// #     }
/// # }
/// # impl CydDisplay for DemoDisplay {
/// #     type Error = CydInfallibleError;
/// #     type Frame<'a> = DemoFrame;
/// #     fn screen_size(&self) -> Size { Size::new(320, 240) }
/// #     fn background(&self) -> Rgb888 { Rgb888::BLACK }
/// #     fn foreground(&self) -> Rgb888 { Rgb888::WHITE }
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
/// #     type Error = CydInfallibleError;
/// #     fn rectangle(&self) -> Rectangle {
/// #         Rectangle::new(Point::zero(), Size::new(320, 240))
/// #     }
/// #     fn fill(&mut self, _color: Rgb565) -> &mut Self { self }
/// #     fn write_text(&mut self, _text: &str) -> &mut Self { self }
/// #     fn copy_from_565(
/// #         &mut self,
/// #         _src: &[u16],
/// #     ) -> Result<(), device_envoy_core::cyd::CopySizeError> {
/// #         Ok(())
/// #     }
/// #     fn flush(
/// #         &mut self,
/// #     ) -> impl core::future::Future<Output = Result<(), <Self as CydFrame>::Error>> {
/// #         ready(Ok(()))
/// #     }
/// # }
/// # impl CydTouch for DemoTouch {
/// #     type Error = CydInfallibleError;
/// #     fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error> { Ok(None) }
/// # }
/// # impl CydRawTouch for DemoCyd {
/// #     type Error = CydInfallibleError;
/// #     fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
/// #         Ok(None)
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
/// # async fn demo() -> Result<(), device_envoy_core::cyd::EnsureCalibrationError<CydInfallibleError, Infallible>> {
/// let mut cyd = DemoCyd;
/// let mut calibration_flash_block = DemoFlashBlock {
///     calibration_config: None,
/// };
/// let mut recalibration_button = DemoButton;
/// let _outcome = ensure_calibration(
///     &mut cyd,
///     &mut calibration_flash_block,
///     &mut recalibration_button,
///     Some("Touch calibrated"),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn ensure_calibration<C, F, R, E>(
    cyd: &mut C,
    calibration_flash_block: &mut F,
    recalibration_button: &mut R,
    confirmed_message: Option<&str>,
) -> Result<EnsureCalibrationOutcome, EnsureCalibrationError<E, F::Error>>
where
    C: Cyd<Error = E> + CydRawTouch<Error = E>,
    F: FlashBlock,
    R: Button,
{
    ensure_calibration_with_settings(
        cyd,
        calibration_flash_block,
        recalibration_button,
        confirmed_message,
        EnsureCalibrationSettings::DEFAULT,
    )
    .await
}

/// Like [`ensure_calibration`], with tunable flow timings.
///
/// Browser builds redraw much faster than the classic CYD, so they need a
/// larger verify-frame budget to preserve the intended real-time grace period
/// before the center confirmation tap.
pub async fn ensure_calibration_with_settings<C, F, R, E>(
    cyd: &mut C,
    calibration_flash_block: &mut F,
    recalibration_button: &mut R,
    confirmed_message: Option<&str>,
    ensure_calibration_settings: EnsureCalibrationSettings,
) -> Result<EnsureCalibrationOutcome, EnsureCalibrationError<E, F::Error>>
where
    C: Cyd<Error = E> + CydRawTouch<Error = E>,
    F: FlashBlock,
    R: Button,
{
    if let Some(calibration_config) = calibration_flash_block
        .load::<CalibrationConfig>()
        .unwrap_or(None)
    {
        return Ok(EnsureCalibrationOutcome::Loaded(calibration_config));
    }

    let mut calibration_flow = CalibrationFlow::new();
    let mut calibration_driver_state = CalibrationDriverState::Capturing;

    loop {
        // A plain Button is intentional here: this loop does synchronous
        // per-frame polling, not cancelable button futures, so ButtonWatch
        // would add an ESP-only dependency without buying correctness.
        if recalibration_button.is_pressed() {
            calibration_flow.restart();
            calibration_driver_state = CalibrationDriverState::Capturing;
        }

        let mut saw_idle = false;
        for _raw_event_index in 0..MAX_RAW_EVENTS_PER_FRAME {
            let raw_touch_event = cyd
                .read_raw_touch_event()
                .map_err(EnsureCalibrationError::Device)?;
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
                            Err(CalibrationSolveError::ResidualTooLarge {
                                worst_residual_pixels,
                            }) => {
                                calibration_flow.restart();
                                calibration_driver_state = CalibrationDriverState::ShowRejected {
                                    worst_residual_pixels: Some(worst_residual_pixels),
                                    frames_remaining: REJECTED_FRAME_COUNT,
                                };
                            }
                            Err(CalibrationSolveError::DegenerateGeometry) => {
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
                        if let Some(confirmed_message) = confirmed_message {
                            draw_message_screen(cyd, confirmed_message)
                                .await
                                .map_err(EnsureCalibrationError::Device)?;
                        }
                        calibration_flash_block
                            .save(candidate_config)
                            .map_err(EnsureCalibrationError::Flash)?;
                        return Ok(EnsureCalibrationOutcome::Saved(*candidate_config));
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

        draw_calibration_screen(cyd, &calibration_flow, &calibration_driver_state)
            .await
            .map_err(EnsureCalibrationError::Device)?;
    }
}

async fn draw_message_screen<C>(cyd: &mut C, message: &str) -> Result<(), C::Error>
where
    C: Cyd,
{
    let (mut display, _touch) = cyd.parts();
    let background565 = display.background_565();
    let mut frame = display.full_frame_mut();
    frame.fill(background565);
    frame.write_text(message).flush().await
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

async fn draw_calibration_screen<C>(
    cyd: &mut C,
    calibration_flow: &CalibrationFlow,
    calibration_driver_state: &CalibrationDriverState,
) -> Result<(), C::Error>
where
    C: Cyd,
{
    let (mut display, _touch) = cyd.parts();
    let background565 = display.background_565();
    let mut frame = display.full_frame_mut();
    frame.fill(background565);

    match calibration_driver_state {
        CalibrationDriverState::Capturing => {
            if let Some(calibration_corner) = calibration_flow.next_corner() {
                match draw_calibration_cross(&mut frame, calibration_corner) {
                    Ok(()) => {}
                    Err(infallible) => match infallible {},
                }
            }
            match draw_calibration_instruction(&mut frame, "Tap cross, then lift") {
                Ok(()) => {}
                Err(infallible) => match infallible {},
            }
        }
        CalibrationDriverState::ShowCaptured {
            calibration_corner, ..
        } => {
            match draw_calibration_ack_dot(&mut frame, *calibration_corner) {
                Ok(()) => {}
                Err(infallible) => match infallible {},
            }
            if let Some(next_corner) = calibration_flow.next_corner() {
                match draw_calibration_cross(&mut frame, next_corner) {
                    Ok(()) => {}
                    Err(infallible) => match infallible {},
                }
            }
            match draw_calibration_instruction(&mut frame, "Corner captured") {
                Ok(()) => {}
                Err(infallible) => match infallible {},
            }
        }
        CalibrationDriverState::ShowRejected {
            worst_residual_pixels,
            ..
        } => {
            if let Some(calibration_corner) = calibration_flow.next_corner() {
                match draw_calibration_rejected_cross(&mut frame, calibration_corner) {
                    Ok(()) => {}
                    Err(infallible) => match infallible {},
                }
            }
            let mut message = String::<48>::new();
            match worst_residual_pixels {
                Some(worst_residual_pixels) => {
                    match write!(&mut message, "Try again ({worst_residual_pixels:.1}px)") {
                        Ok(()) => {}
                        Err(_) => unreachable!("heapless message capacity is sufficient"),
                    }
                }
                None => match message.push_str("Try again") {
                    Ok(()) => {}
                    Err(_) => unreachable!("heapless message capacity is sufficient"),
                },
            }
            match draw_calibration_instruction(&mut frame, message.as_str()) {
                Ok(()) => {}
                Err(infallible) => match infallible {},
            }
        }
        CalibrationDriverState::Verifying { .. } => {
            match draw_calibration_verify_target(&mut frame) {
                Ok(()) => {}
                Err(infallible) => match infallible {},
            }
            match draw_calibration_instruction(&mut frame, "Tap center to save") {
                Ok(()) => {}
                Err(infallible) => match infallible {},
            }
        }
    }

    frame.flush().await
}

fn hit_verify_target(mapped_x: f32, mapped_y: f32) -> bool {
    let verify_target_center = super::calibration_verify_target_center();
    let delta_x = mapped_x - verify_target_center.x as f32;
    let delta_y = mapped_y - verify_target_center.y as f32;
    delta_x * delta_x + delta_y * delta_y
        <= super::VERIFY_HIT_RADIUS_PIXELS * super::VERIFY_HIT_RADIUS_PIXELS
}
