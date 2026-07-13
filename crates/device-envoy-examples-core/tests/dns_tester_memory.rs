#![cfg(feature = "host")]

use device_envoy_core::cyd::touch::TouchEvent;
use device_envoy_core::{
    cyd::display::Orientation,
    memory::{CydMemory, assert_framebuffer_matches_expected_png},
};
use device_envoy_examples_core::dns_tester::{
    DnsResult, DnsTesterAction, DnsTesterApp, DnsTesterInput, DnsTesterUiState, render, render_app,
};
use embedded_graphics::geometry::Point;
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888, prelude::Size};
use futures_executor::block_on;

#[test]
fn shared_dns_tester_renderer_uses_memory_cyd() -> Result<(), Box<dyn std::error::Error>> {
    let cyd_memory = CydMemory::new(
        Size::new(320, 240),
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    let mut display = cyd_memory.display();
    block_on(render(
        &mut display,
        Orientation::Landscape,
        DnsTesterUiState {
            target: "example.com",
            queries: 1,
            successes: 1,
            failures: 0,
            last_latency_millis: 22,
        },
    ))
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    assert_eq!(cyd_memory.flush_count(), 6);
    Ok(())
}

#[test]
fn shared_dns_tester_orientation_goldens() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = DnsTesterApp::new("example.com", Orientation::Landscape);
    app.input(DnsTesterInput::WifiReady);
    app.input(DnsTesterInput::DnsFinished(DnsResult {
        succeeded: true,
        latency_millis: 22,
    }));
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
        let cyd_memory = CydMemory::new_with_orientation(
            orientation,
            Rgb888::new(10, 10, 12),
            Rgb888::new(230, 230, 230),
            &FONT_6X10,
        );
        let mut display = cyd_memory.display();
        assert_eq!(app.orientation(), orientation);
        block_on(render_app(&mut display, &app))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        if matches!(
            orientation,
            Orientation::LandscapeInverted | Orientation::PortraitInverted
        ) {
            cyd_memory.rotate_framebuffer_180();
        }
        assert_framebuffer_matches_expected_png(&cyd_memory, env!("CARGO_MANIFEST_DIR"), filename)?;

        if orientation != Orientation::PortraitInverted {
            let (x, y) = if orientation.width() > orientation.height() {
                (260, 216)
            } else {
                (193, 294)
            };
            let next_orientation = orientation.next();
            assert_eq!(
                app.input(DnsTesterInput::Touch(TouchEvent::Down {
                    point: Point::new(x, y),
                })),
                DnsTesterAction::SaveOrientationAndRestart(next_orientation)
            );
        }
    }
    let landscape = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/assets/dns_tester_landscape.png"),
    )?;
    let landscape_inverted = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/assets/dns_tester_landscape_inverted.png"),
    )?;
    let portrait = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/assets/dns_tester_portrait.png"),
    )?;
    let portrait_inverted = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/assets/dns_tester_portrait_inverted.png"),
    )?;
    assert_ne!(landscape, landscape_inverted);
    assert_ne!(portrait, portrait_inverted);
    Ok(())
}

#[test]
fn inverted_dashboard_presentations_are_physical_rotations()
-> Result<(), Box<dyn std::error::Error>> {
    for (orientation, inverted_orientation) in [
        (Orientation::Landscape, Orientation::LandscapeInverted),
        (Orientation::Portrait, Orientation::PortraitInverted),
    ] {
        let normal = render_dashboard(orientation)?;
        let inverted = render_dashboard(inverted_orientation)?;
        let width = orientation.width() as usize;
        let height = orientation.height() as usize;
        for position_y in 0..height {
            for position_x in 0..width {
                assert_eq!(
                    normal.pixel(position_x, position_y),
                    inverted.pixel(width - position_x - 1, height - position_y - 1),
                    "inverted presentation differs at ({position_x}, {position_y}) for {orientation:?}",
                );
            }
        }
    }
    Ok(())
}

fn render_dashboard(orientation: Orientation) -> Result<CydMemory, Box<dyn std::error::Error>> {
    let cyd_memory = CydMemory::new_with_orientation(
        orientation,
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    let mut app = DnsTesterApp::new("example.com", orientation);
    app.input(DnsTesterInput::WifiReady);
    app.input(DnsTesterInput::DnsFinished(DnsResult {
        succeeded: true,
        latency_millis: 22,
    }));
    let mut display = cyd_memory.display();
    block_on(render_app(&mut display, &app))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    if matches!(
        orientation,
        Orientation::LandscapeInverted | Orientation::PortraitInverted
    ) {
        cyd_memory.rotate_framebuffer_180();
    }
    Ok(cyd_memory)
}

#[test]
fn shared_dns_tester_state_goldens() -> Result<(), Box<dyn std::error::Error>> {
    let states: [(&str, fn(&mut DnsTesterApp)); 6] = [
        ("dns_tester_splash.png", |_app: &mut DnsTesterApp| {}),
        ("dns_tester_connecting.png", |app: &mut DnsTesterApp| {
            app.input(DnsTesterInput::WifiConnecting);
        }),
        ("dns_tester_setup.png", |app: &mut DnsTesterApp| {
            app.input(DnsTesterInput::WifiSetup);
        }),
        ("dns_tester_unavailable.png", |app: &mut DnsTesterApp| {
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: Point::new(150, 216),
            }));
        }),
        ("dns_tester_success.png", |app: &mut DnsTesterApp| {
            app.input(DnsTesterInput::WifiReady);
            app.input(DnsTesterInput::DnsFinished(DnsResult {
                succeeded: true,
                latency_millis: 22,
            }));
        }),
        ("dns_tester_failure.png", |app: &mut DnsTesterApp| {
            app.input(DnsTesterInput::WifiReady);
            app.input(DnsTesterInput::DnsFinished(DnsResult {
                succeeded: false,
                latency_millis: 91,
            }));
        }),
    ];

    for (filename, apply) in states {
        let cyd_memory = CydMemory::new_with_orientation(
            Orientation::Landscape,
            Rgb888::new(10, 10, 12),
            Rgb888::new(230, 230, 230),
            &FONT_6X10,
        );
        let mut display = cyd_memory.display();
        let mut app = DnsTesterApp::new("example.com", Orientation::Landscape);
        apply(&mut app);
        block_on(render_app(&mut display, &app))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        assert_framebuffer_matches_expected_png(&cyd_memory, env!("CARGO_MANIFEST_DIR"), filename)?;
    }
    Ok(())
}

#[test]
fn shared_dns_tester_notices_and_controls_use_public_app_api() {
    for orientation in [
        Orientation::Landscape,
        Orientation::Portrait,
        Orientation::LandscapeInverted,
        Orientation::PortraitInverted,
    ] {
        let mut app = DnsTesterApp::new("example.com", orientation);
        assert!(app.notice().is_some());
        app.input(DnsTesterInput::WifiConnecting);
        app.input(DnsTesterInput::WifiSetup);
        assert!(app.notice().is_some());
        app.input(DnsTesterInput::WifiReady);
        assert!(app.notice().is_none());

        let (x, y) = if orientation.width() > orientation.height() {
            (260, 216)
        } else {
            (193, 294)
        };
        assert_eq!(
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: Point::new(x, y),
            })),
            DnsTesterAction::SaveOrientationAndRestart(orientation.next())
        );
    }
}

#[test]
fn shared_dns_tester_rotation_and_calibration_actions_are_explicit() {
    let mut app = DnsTesterApp::new("example.com", Orientation::Landscape);
    for expected_orientation in [
        Orientation::Portrait,
        Orientation::LandscapeInverted,
        Orientation::PortraitInverted,
        Orientation::Landscape,
    ] {
        let (x, y) = if app.orientation().width() > app.orientation().height() {
            (260, 216)
        } else {
            (193, 294)
        };
        assert_eq!(
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: Point::new(x, y),
            })),
            DnsTesterAction::SaveOrientationAndRestart(expected_orientation)
        );
        assert_eq!(app.orientation(), expected_orientation);
    }
    assert_eq!(
        app.input(DnsTesterInput::Boot),
        DnsTesterAction::ClearCalibrationAndRestart
    );
    assert_eq!(
        DnsTesterApp::display_orientation_for_calibration(Orientation::PortraitInverted, false,),
        Orientation::Landscape
    );
    assert_eq!(
        DnsTesterApp::display_orientation_for_calibration(Orientation::PortraitInverted, true),
        Orientation::PortraitInverted
    );
    assert_eq!(
        DnsTesterApp::orientation_after_calibration(Orientation::PortraitInverted),
        Orientation::PortraitInverted
    );

    for orientation in [
        Orientation::Landscape,
        Orientation::Portrait,
        Orientation::LandscapeInverted,
        Orientation::PortraitInverted,
    ] {
        let mut app = DnsTesterApp::new("example.com", orientation);
        let (calibration_point, wifi_point, test_point) =
            if orientation.width() > orientation.height() {
                (
                    Point::new(20, 216),
                    Point::new(120, 216),
                    Point::new(160, 120),
                )
            } else {
                (
                    Point::new(20, 294),
                    Point::new(100, 294),
                    Point::new(120, 120),
                )
            };
        assert_eq!(
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: calibration_point,
            })),
            DnsTesterAction::ClearCalibrationAndRestart
        );
        assert_eq!(
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: wifi_point
            })),
            DnsTesterAction::ResetWifiAndRestart
        );
        assert_eq!(
            app.input(DnsTesterInput::Touch(TouchEvent::Down {
                point: test_point
            })),
            DnsTesterAction::StartDnsLookup
        );
    }
}
