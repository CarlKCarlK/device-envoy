use super::calibration::{CALIBRATION_POINT_COUNT, CalibrationCorner, calibration_corner_for_index};
use super::{RawPoint, RawTouchEvent};

pub const SAMPLES_DISCARDED_AFTER_DOWN: usize = 4;
pub const MIN_SAMPLES_PER_POINT: usize = 3;

/// Sans-io state machine for the four-tap calibration flow.
///
/// Callers own I/O: they draw [`CalibrationCorner`] crosses, log progress, and
/// persist the finished solve. This flow tracks which corner is next and
/// accumulates per-touch raw samples until all four corners have been captured
/// on release.
pub struct CalibrationFlow {
    calibration_index: usize,
    calibration_points: [RawPoint; CALIBRATION_POINT_COUNT],
    release_touch_capture: ReleaseTouchCapture,
}

#[derive(Clone, Copy)]
enum ReleaseTouchCaptureState {
    Armed,
    Sampling {
        discarded_sample_count: usize,
        // Raw coordinates peak around 4095 (~2^12), so even an absurdly long
        // press would need more than 2^50 samples before these u64 sums could
        // overflow.
        sum_x: u64,
        sum_y: u64,
        usable_sample_count: usize,
    },
    WaitForIdle,
}

pub(super) struct ReleaseTouchCapture {
    release_touch_capture_state: ReleaseTouchCaptureState,
}

impl CalibrationFlow {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            calibration_index: 0,
            calibration_points: [RawPoint { x: 0, y: 0 }; CALIBRATION_POINT_COUNT],
            release_touch_capture: ReleaseTouchCapture::new(),
        }
    }

    pub fn restart(&mut self) {
        self.calibration_index = 0;
        self.calibration_points = [RawPoint { x: 0, y: 0 }; CALIBRATION_POINT_COUNT];
        self.release_touch_capture.restart();
    }

    #[must_use]
    pub fn next_corner(&self) -> Option<CalibrationCorner> {
        calibration_corner_for_index(self.calibration_index)
    }

    #[cfg(test)]
    #[must_use]
    pub fn calibration_index(&self) -> usize {
        self.calibration_index
    }

    pub fn handle_raw_touch_event(
        &mut self,
        raw_touch_event: Option<RawTouchEvent>,
    ) -> Option<CalibrationFlowEvent> {
        let Some(calibration_corner) = self.next_corner() else {
            return None;
        };

        let Some(release_touch_capture_event) = self
            .release_touch_capture
            .handle_raw_touch_event(raw_touch_event)
        else {
            return None;
        };

        let ReleaseTouchCaptureEvent::Captured {
            raw_point,
            usable_sample_count,
        } = release_touch_capture_event;
        self.calibration_points[self.calibration_index] = raw_point;
        self.calibration_index += 1;

        if self.calibration_index == CALIBRATION_POINT_COUNT {
            return Some(CalibrationFlowEvent::Completed {
                raw_points: self.calibration_points,
                calibration_corner,
                usable_sample_count,
            });
        }

        Some(CalibrationFlowEvent::PointCaptured {
            calibration_corner,
            raw_point,
            next_corner: self
                .next_corner()
                .expect("next corner exists until calibration completes"),
            usable_sample_count,
        })
    }
}

impl ReleaseTouchCapture {
    pub const fn new() -> Self {
        Self {
            release_touch_capture_state: ReleaseTouchCaptureState::Armed,
        }
    }

    pub fn restart(&mut self) {
        self.release_touch_capture_state = ReleaseTouchCaptureState::Armed;
    }

    pub fn handle_raw_touch_event(
        &mut self,
        raw_touch_event: Option<RawTouchEvent>,
    ) -> Option<ReleaseTouchCaptureEvent> {
        match self.release_touch_capture_state {
            ReleaseTouchCaptureState::Armed => {
                let Some(RawTouchEvent::Down { raw_x, raw_y }) = raw_touch_event else {
                    return None;
                };
                self.release_touch_capture_state = ReleaseTouchCaptureState::Sampling {
                    discarded_sample_count: 0,
                    sum_x: u64::from(raw_x),
                    sum_y: u64::from(raw_y),
                    usable_sample_count: 1,
                };
                None
            }
            ReleaseTouchCaptureState::Sampling {
                discarded_sample_count,
                sum_x,
                sum_y,
                usable_sample_count,
            } => match raw_touch_event {
                Some(RawTouchEvent::Down { raw_x, raw_y })
                | Some(RawTouchEvent::Move { raw_x, raw_y }) => {
                    if discarded_sample_count < SAMPLES_DISCARDED_AFTER_DOWN {
                        self.release_touch_capture_state = ReleaseTouchCaptureState::Sampling {
                            discarded_sample_count: discarded_sample_count + 1,
                            sum_x,
                            sum_y,
                            usable_sample_count,
                        };
                        return None;
                    }

                    self.release_touch_capture_state = ReleaseTouchCaptureState::Sampling {
                        discarded_sample_count,
                        sum_x: sum_x + u64::from(raw_x),
                        sum_y: sum_y + u64::from(raw_y),
                        usable_sample_count: usable_sample_count + 1,
                    };
                    None
                }
                Some(RawTouchEvent::Up) => {
                    self.release_touch_capture_state = ReleaseTouchCaptureState::WaitForIdle;
                    if usable_sample_count < MIN_SAMPLES_PER_POINT {
                        return None;
                    }

                    Some(ReleaseTouchCaptureEvent::Captured {
                        raw_point: average_samples(sum_x, sum_y, usable_sample_count),
                        usable_sample_count,
                    })
                }
                None => None,
            },
            ReleaseTouchCaptureState::WaitForIdle => {
                if raw_touch_event.is_none() {
                    self.release_touch_capture_state = ReleaseTouchCaptureState::Armed;
                }
                None
            }
        }
    }
}

impl Default for CalibrationFlow {
    fn default() -> Self {
        Self::new()
    }
}

pub enum CalibrationFlowEvent {
    PointCaptured {
        calibration_corner: CalibrationCorner,
        raw_point: RawPoint,
        next_corner: CalibrationCorner,
        usable_sample_count: usize,
    },
    Completed {
        raw_points: [RawPoint; CALIBRATION_POINT_COUNT],
        calibration_corner: CalibrationCorner,
        usable_sample_count: usize,
    },
}

pub(super) enum ReleaseTouchCaptureEvent {
    Captured {
        raw_point: RawPoint,
        usable_sample_count: usize,
    },
}

fn average_samples(sum_x: u64, sum_y: u64, usable_sample_count: usize) -> RawPoint {
    let usable_sample_count = usable_sample_count as u64;
    RawPoint {
        x: ((sum_x + usable_sample_count / 2) / usable_sample_count) as u16,
        y: ((sum_y + usable_sample_count / 2) / usable_sample_count) as u16,
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::{
        CalibrationCorner, CalibrationFlow, CalibrationFlowEvent, MIN_SAMPLES_PER_POINT, RawPoint,
        RawTouchEvent, SAMPLES_DISCARDED_AFTER_DOWN,
    };

    #[test]
    fn clean_taps_complete_with_averaged_points() {
        let mut calibration_flow = CalibrationFlow::new();

        let calibration_flow_event =
            run_tap(&mut calibration_flow, &sample_fixture(100, 200, 1, 1, 0));
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::UpperLeft,
            RawPoint { x: 105, y: 205 },
            CalibrationCorner::UpperRight,
        );

        let calibration_flow_event =
            run_tap(&mut calibration_flow, &sample_fixture(900, 210, 1, 1, 0));
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::UpperRight,
            RawPoint { x: 905, y: 215 },
            CalibrationCorner::LowerRight,
        );

        let calibration_flow_event =
            run_tap(&mut calibration_flow, &sample_fixture(910, 800, 1, 1, 0));
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::LowerRight,
            RawPoint { x: 915, y: 805 },
            CalibrationCorner::LowerLeft,
        );

        let calibration_flow_event =
            run_tap(&mut calibration_flow, &sample_fixture(120, 790, 1, 1, 0));
        let CalibrationFlowEvent::Completed {
            raw_points,
            calibration_corner,
            usable_sample_count,
            ..
        } = calibration_flow_event
        else {
            panic!("expected completed event");
        };

        assert_eq!(calibration_corner, CalibrationCorner::LowerLeft);
        assert_eq!(usable_sample_count, 4);
        assert_eq!(
            raw_points,
            [
                RawPoint { x: 105, y: 205 },
                RawPoint { x: 905, y: 215 },
                RawPoint { x: 915, y: 805 },
                RawPoint { x: 125, y: 795 },
            ]
        );
    }

    #[test]
    fn held_stylus_dropout_does_not_capture_next_corner_from_old_spot() {
        let mut calibration_flow = CalibrationFlow::new();
        let calibration_flow_event =
            run_tap(&mut calibration_flow, &sample_fixture(100, 200, 1, 1, 0));
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::UpperLeft,
            RawPoint { x: 105, y: 205 },
            CalibrationCorner::UpperRight,
        );

        let calibration_flow_event =
            consume_tap_without_assert(&mut calibration_flow, &sample_fixture(900, 210, 1, 1, 0));
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::UpperRight,
            RawPoint { x: 905, y: 215 },
            CalibrationCorner::LowerRight,
        );

        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Down {
                    raw_x: 905,
                    raw_y: 215,
                }))
                .is_none()
        );
        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Move {
                    raw_x: 906,
                    raw_y: 216,
                }))
                .is_none()
        );
        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Up))
                .is_none()
        );
        assert_eq!(calibration_flow.calibration_index(), 2);
        assert_eq!(
            calibration_flow.next_corner(),
            Some(CalibrationCorner::LowerRight)
        );

        assert!(calibration_flow.handle_raw_touch_event(None).is_none());

        let calibration_flow_event =
            run_tap(&mut calibration_flow, &sample_fixture(910, 800, 1, 1, 0));
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::LowerRight,
            RawPoint { x: 915, y: 805 },
            CalibrationCorner::LowerLeft,
        );
    }

    #[test]
    fn short_graze_below_minimum_samples_does_not_capture_corner() {
        let mut calibration_flow = CalibrationFlow::new();

        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Down {
                    raw_x: 100,
                    raw_y: 200,
                }))
                .is_none()
        );
        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Up))
                .is_none()
        );
        assert!(calibration_flow.handle_raw_touch_event(None).is_none());
        assert_eq!(calibration_flow.calibration_index(), 0);
        assert_eq!(
            calibration_flow.next_corner(),
            Some(CalibrationCorner::UpperLeft)
        );
    }

    #[test]
    fn move_only_noise_while_armed_is_ignored() {
        let mut calibration_flow = CalibrationFlow::new();

        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Move {
                    raw_x: 400,
                    raw_y: 500,
                }))
                .is_none()
        );
        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Up))
                .is_none()
        );
        assert_eq!(calibration_flow.calibration_index(), 0);
        assert_eq!(
            calibration_flow.next_corner(),
            Some(CalibrationCorner::UpperLeft)
        );
    }

    #[test]
    fn long_press_average_ignores_lift_off_drift() {
        let mut calibration_flow = CalibrationFlow::new();

        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Down {
                    raw_x: 1000,
                    raw_y: 2000,
                }))
                .is_none()
        );

        for _ in 0..1004 {
            assert!(
                calibration_flow
                    .handle_raw_touch_event(Some(RawTouchEvent::Move {
                        raw_x: 1000,
                        raw_y: 2000,
                    }))
                    .is_none()
            );
        }

        for _ in 0..5 {
            assert!(
                calibration_flow
                    .handle_raw_touch_event(Some(RawTouchEvent::Move {
                        raw_x: 1400,
                        raw_y: 2400,
                    }))
                    .is_none()
            );
        }

        let calibration_flow_event = calibration_flow
            .handle_raw_touch_event(Some(RawTouchEvent::Up))
            .expect("long press should capture a calibration point");
        assert_point_captured(
            calibration_flow_event,
            CalibrationCorner::UpperLeft,
            RawPoint { x: 1002, y: 2002 },
            CalibrationCorner::UpperRight,
        );
        assert!(calibration_flow.handle_raw_touch_event(None).is_none());
    }

    fn sample_fixture(
        start_x: u16,
        start_y: u16,
        delta_x: u16,
        delta_y: u16,
        extra_usable_sample_count: usize,
    ) -> Vec<(u16, u16)> {
        let sample_count =
            1 + SAMPLES_DISCARDED_AFTER_DOWN + MIN_SAMPLES_PER_POINT + extra_usable_sample_count;
        let mut raw_samples = Vec::with_capacity(sample_count);
        for sample_index in 0..sample_count {
            raw_samples.push((
                start_x + sample_index as u16 * delta_x,
                start_y + sample_index as u16 * delta_y,
            ));
        }
        raw_samples
    }

    fn run_tap(
        calibration_flow: &mut CalibrationFlow,
        raw_samples: &[(u16, u16)],
    ) -> CalibrationFlowEvent {
        let calibration_flow_event = consume_tap_without_assert(calibration_flow, raw_samples);
        assert!(calibration_flow.handle_raw_touch_event(None).is_none());
        calibration_flow_event
    }

    fn consume_tap_without_assert(
        calibration_flow: &mut CalibrationFlow,
        raw_samples: &[(u16, u16)],
    ) -> CalibrationFlowEvent {
        let Some(&(down_x, down_y)) = raw_samples.first() else {
            panic!("tap must include a down sample");
        };

        assert!(
            calibration_flow
                .handle_raw_touch_event(Some(RawTouchEvent::Down {
                    raw_x: down_x,
                    raw_y: down_y,
                }))
                .is_none()
        );
        for &(move_x, move_y) in &raw_samples[1..] {
            assert!(
                calibration_flow
                    .handle_raw_touch_event(Some(RawTouchEvent::Move {
                        raw_x: move_x,
                        raw_y: move_y,
                    }))
                    .is_none()
            );
        }
        calibration_flow
            .handle_raw_touch_event(Some(RawTouchEvent::Up))
            .expect("tap should capture a calibration point")
    }

    fn assert_point_captured(
        calibration_flow_event: CalibrationFlowEvent,
        calibration_corner: CalibrationCorner,
        raw_point: RawPoint,
        next_corner: CalibrationCorner,
    ) {
        let CalibrationFlowEvent::PointCaptured {
            calibration_corner: actual_corner,
            raw_point: actual_raw_point,
            next_corner: actual_next_corner,
            usable_sample_count,
        } = calibration_flow_event
        else {
            panic!("expected point captured event");
        };

        assert_eq!(actual_corner, calibration_corner);
        assert_eq!(actual_raw_point, raw_point);
        assert_eq!(actual_next_corner, next_corner);
        assert!(usable_sample_count >= MIN_SAMPLES_PER_POINT);
    }
}
