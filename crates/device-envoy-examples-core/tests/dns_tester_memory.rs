#![cfg(feature = "host")]

use device_envoy_core::{
    cyd::{display::Orientation, touch::TouchEvent},
    dns::{Dns, DnsResult},
    memory::{CydMemory, assert_framebuffer_matches_expected_png},
};
use device_envoy_examples_core::dns_tester::{Exit, UiNotice, dns_tester, render_notice};
use embedded_graphics::{
    geometry::Point, mono_font::ascii::FONT_6X10, pixelcolor::Rgb888, prelude::Size,
};
use futures_executor::block_on;
use std::{cell::Cell, rc::Rc};

struct SuccessfulDns;

impl Dns for SuccessfulDns {
    type Error = core::convert::Infallible;

    fn hostname(&self) -> &'static str {
        "example.com"
    }

    async fn lookup(&mut self) -> Result<DnsResult, Self::Error> {
        Ok(DnsResult {
            succeeded: true,
            latency_millis: 22,
        })
    }
}

struct CountingDns {
    lookup_count: Rc<Cell<u32>>,
}

impl Dns for CountingDns {
    type Error = core::convert::Infallible;

    fn hostname(&self) -> &'static str {
        "example.com"
    }

    async fn lookup(&mut self) -> Result<DnsResult, Self::Error> {
        self.lookup_count
            .set(self.lookup_count.get().saturating_add(1));
        Ok(DnsResult {
            succeeded: true,
            latency_millis: 22,
        })
    }
}

#[test]
fn scripted_runtime_owns_startup_input_dns_and_rendering() -> Result<(), Box<dyn std::error::Error>>
{
    let mut cyd_memory = CydMemory::new(
        Size::new(320, 240),
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    let mut button = cyd_memory.button_memory();
    button.set_pressed(true);
    let mut dns = SuccessfulDns;
    assert_eq!(
        block_on(dns_tester(&mut cyd_memory, &mut button, &mut dns))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
        Exit::Calibrate
    );
    assert!(cyd_memory.flush_count() > 0);
    Ok(())
}

#[test]
fn shared_dns_tester_orientation_goldens() -> Result<(), Box<dyn std::error::Error>> {
    for (orientation, filename) in [
        (Orientation::Landscape, "dns_tester_landscape.png"),
        (Orientation::Portrait, "dns_tester_portrait.png"),
        (
            Orientation::LandscapeInverted,
            "dns_tester_landscape_inverted.png",
        ),
        (
            Orientation::PortraitInverted,
            "dns_tester_portrait_inverted.png",
        ),
    ] {
        let mut cyd_memory = CydMemory::new_with_orientation(
            orientation,
            Rgb888::new(10, 10, 12),
            Rgb888::new(230, 230, 230),
            &FONT_6X10,
        );
        assert_eq!(
            device_envoy_core::cyd::Cyd::orientation(&mut cyd_memory),
            orientation
        );
        cyd_memory.push_touch_event(TouchEvent::Down {
            point: Point::new(
                orientation.width() as i32 / 2,
                orientation.height() as i32 / 2,
            ),
        });
        let mut button = cyd_memory.button_memory();
        for frame_index in 7..100 {
            button.set_pressed_for_frame(frame_index, true);
        }
        let mut dns = SuccessfulDns;
        assert_eq!(
            block_on(dns_tester(&mut cyd_memory, &mut button, &mut dns,))
                .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
            Exit::Calibrate
        );
        if matches!(
            orientation,
            Orientation::LandscapeInverted | Orientation::PortraitInverted
        ) {
            cyd_memory.rotate_framebuffer_180();
        }
        assert_framebuffer_matches_expected_png(&cyd_memory, env!("CARGO_MANIFEST_DIR"), filename)?;
    }
    Ok(())
}

#[test]
fn portrait_splash_golden() -> Result<(), Box<dyn std::error::Error>> {
    let cyd_memory = CydMemory::new_with_orientation(
        Orientation::Portrait,
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    let mut display = cyd_memory.display();
    block_on(render_notice(
        &mut display,
        Orientation::Portrait,
        UiNotice::Splash,
    ))
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "dns_tester_portrait_splash.png",
    )?;
    Ok(())
}

#[test]
fn cyd_memory_routes_controls_in_each_orientation() -> Result<(), Box<dyn std::error::Error>> {
    for orientation in [
        Orientation::Landscape,
        Orientation::Portrait,
        Orientation::LandscapeInverted,
        Orientation::PortraitInverted,
    ] {
        let logical_points = match orientation {
            Orientation::Landscape | Orientation::LandscapeInverted => [
                Point::new(60, 216),
                Point::new(160, 216),
                Point::new(260, 216),
            ],
            Orientation::Portrait | Orientation::PortraitInverted => [
                Point::new(46, 294),
                Point::new(120, 294),
                Point::new(193, 294),
            ],
        };
        let expected_exits = [
            Exit::Calibrate,
            Exit::ResetWifi,
            Exit::Reorientate(orientation.next()),
        ];

        for (logical_point, expected_exit) in logical_points.into_iter().zip(expected_exits) {
            let mut cyd_memory = CydMemory::new_with_orientation(
                orientation,
                Rgb888::new(10, 10, 12),
                Rgb888::new(230, 230, 230),
                &FONT_6X10,
            );
            cyd_memory.push_touch_event(TouchEvent::Down {
                point: physical_point_for_logical_point(orientation, logical_point),
            });
            let mut button = cyd_memory.button_memory();
            let mut dns = SuccessfulDns;

            assert_eq!(
                block_on(dns_tester(&mut cyd_memory, &mut button, &mut dns))
                    .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
                expected_exit
            );
        }
    }
    Ok(())
}

#[test]
fn ordinary_touch_runs_dns_once_and_ignores_move_up() -> Result<(), Box<dyn std::error::Error>> {
    let mut cyd_memory = CydMemory::new_with_orientation(
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    cyd_memory.push_touch_event(TouchEvent::Down {
        point: Point::new(160, 120),
    });
    cyd_memory.push_touch_event(TouchEvent::Move {
        point: Point::new(161, 121),
    });
    cyd_memory.push_touch_event(TouchEvent::Up);
    let mut button = cyd_memory.button_memory();
    for frame_index in 7..100 {
        button.set_pressed_for_frame(frame_index, true);
    }
    let lookup_count = Rc::new(Cell::new(0));
    let mut dns = CountingDns {
        lookup_count: lookup_count.clone(),
    };

    assert_eq!(
        block_on(dns_tester(&mut cyd_memory, &mut button, &mut dns))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
        Exit::Calibrate
    );
    assert_eq!(lookup_count.get(), 1);
    Ok(())
}

fn physical_point_for_logical_point(orientation: Orientation, point: Point) -> Point {
    match orientation {
        Orientation::Landscape => point,
        Orientation::Portrait => Point::new(319 - point.y, point.x),
        Orientation::LandscapeInverted => Point::new(319 - point.x, 239 - point.y),
        Orientation::PortraitInverted => Point::new(point.y, 239 - point.x),
    }
}
