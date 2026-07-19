#![cfg(feature = "host")]

use device_envoy_core::{
    cyd::{display::Orientation, touch::TouchEvent},
    dns::{Addresses, Dns, IpAddress},
    memory::{CydMemory, assert_framebuffer_matches_expected_png},
};
use device_envoy_examples_core::dns_tester::{Exit, UiNotice, render_notice, run};
use embedded_graphics::{
    geometry::Point, mono_font::ascii::FONT_6X10, pixelcolor::Rgb888, prelude::Size,
};
use futures_executor::block_on;
use std::{cell::Cell, rc::Rc};

struct SuccessfulDns;

impl Dns for SuccessfulDns {
    type Error = core::convert::Infallible;

    async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
        Ok([IpAddress::Ipv4([127, 0, 0, 1].into())]
            .into_iter()
            .collect())
    }
}

struct CountingDns {
    lookup_count: Rc<Cell<u32>>,
}

impl Dns for CountingDns {
    type Error = core::convert::Infallible;

    async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
        self.lookup_count
            .set(self.lookup_count.get().saturating_add(1));
        Ok([IpAddress::Ipv4([127, 0, 0, 1].into())]
            .into_iter()
            .collect())
    }
}

#[test]
fn scripted_runtime_owns_dns_and_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let mut cyd_memory = CydMemory::new(
        Size::new(320, 240),
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    cyd_memory.push_touch_event(TouchEvent::Down {
        point: Point::new(260, 216),
    });
    let mut dns = SuccessfulDns;
    let mut button = cyd_memory.button_memory();
    assert_eq!(
        block_on(run(&mut cyd_memory, &mut button, &mut dns))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
        Exit::Reorientate(Orientation::Portrait)
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
        cyd_memory.push_touch_event(TouchEvent::Down {
            point: physical_point_for_logical_point(
                orientation,
                orientation_control_point(orientation),
            ),
        });
        let mut dns = SuccessfulDns;
        let mut button = cyd_memory.button_memory();
        assert_eq!(
            block_on(run(&mut cyd_memory, &mut button, &mut dns))
                .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
            Exit::Reorientate(orientation.next())
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
    block_on(render_notice(
        &mut cyd_memory.display(),
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
    cyd_memory.push_touch_event(TouchEvent::Down {
        point: physical_point_for_logical_point(
            Orientation::Landscape,
            orientation_control_point(Orientation::Landscape),
        ),
    });
    let lookup_count = Rc::new(Cell::new(0));
    let mut dns = CountingDns {
        lookup_count: lookup_count.clone(),
    };

    let mut button = cyd_memory.button_memory();
    assert_eq!(
        block_on(run(&mut cyd_memory, &mut button, &mut dns))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
        Exit::Reorientate(Orientation::Portrait)
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

fn orientation_control_point(orientation: Orientation) -> Point {
    match orientation {
        Orientation::Landscape | Orientation::LandscapeInverted => Point::new(260, 216),
        Orientation::Portrait | Orientation::PortraitInverted => Point::new(193, 294),
    }
}
