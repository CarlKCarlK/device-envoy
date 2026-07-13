#![cfg(feature = "host")]

use device_envoy_core::{cyd::display::Orientation, memory::CydMemory};
use device_envoy_examples_core::dns_tester::{DnsTesterUiState, render};
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
