#![cfg(feature = "host")]

use device_envoy_core::{
    cyd::{display::Orientation, touch::TouchEvent},
    dns_lookup::{DnsLookup, DnsLookupResult},
    memory::{CydMemory, assert_framebuffer_matches_expected_png},
};
use device_envoy_examples_core::dns_tester::{Exit, dns_tester};
use embedded_graphics::{
    geometry::Point, mono_font::ascii::FONT_6X10, pixelcolor::Rgb888, prelude::Size,
};
use futures_executor::block_on;

struct SuccessfulDnsLookup;

impl DnsLookup for SuccessfulDnsLookup {
    type Error = core::convert::Infallible;

    async fn lookup(&mut self, _hostname: &str) -> Result<DnsLookupResult, Self::Error> {
        Ok(DnsLookupResult {
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
    let mut dns_lookup = SuccessfulDnsLookup;
    assert_eq!(
        block_on(dns_tester(
            &mut cyd_memory,
            &mut button,
            "example.com",
            &mut dns_lookup
        ))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
        Exit::CalibrationRequested
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
        let mut dns_lookup = SuccessfulDnsLookup;
        assert_eq!(
            block_on(dns_tester(
                &mut cyd_memory,
                &mut button,
                "example.com",
                &mut dns_lookup,
            ))
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
            Exit::CalibrationRequested
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
