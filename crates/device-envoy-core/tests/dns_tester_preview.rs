#![cfg(feature = "host")]

use std::error::Error;

use device_envoy_core::{
    cyd::display::Orientation,
    dns_tester::{DnsTesterUiError, DnsTesterUiState, render},
    memory::{CydMemory, assert_framebuffer_matches_expected_png},
};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888, prelude::Size};
use futures_executor::block_on;

#[test]
fn dns_tester_landscape_preview_matches_expected() -> Result<(), Box<dyn Error>> {
    let mut cyd_memory = CydMemory::new(
        Size::new(320, 240),
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    cyd_memory.set_frame_budget(16);
    let mut display = cyd_memory.display();
    block_on(render(
        &mut display,
        Orientation::Landscape,
        DnsTesterUiState {
            queries: 1,
            successes: 1,
            failures: 0,
            last_latency_millis: 22,
        },
    ))
    .map_err(|error: DnsTesterUiError<_>| format!("DNS tester render failed: {error:?}"))?;

    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "dns_tester.png",
    )?;
    Ok(())
}

#[test]
fn dns_tester_portrait_preview_matches_expected() -> Result<(), Box<dyn Error>> {
    let mut cyd_memory = CydMemory::new(
        Size::new(240, 320),
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    cyd_memory.set_frame_budget(16);
    let mut display = cyd_memory.display();
    block_on(render(
        &mut display,
        Orientation::Portrait,
        DnsTesterUiState {
            queries: 1,
            successes: 1,
            failures: 0,
            last_latency_millis: 22,
        },
    ))
    .map_err(|error: DnsTesterUiError<_>| format!("DNS tester render failed: {error:?}"))?;

    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "dns_tester_portrait.png",
    )?;
    Ok(())
}
