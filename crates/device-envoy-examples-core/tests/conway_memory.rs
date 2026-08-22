#![cfg(feature = "host")]

use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
use device_envoy_examples_core::conway_app::{ConwayApp, ConwayInput, ConwayStatus};
use embedded_graphics::{
    geometry::Size,
    mono_font::ascii::FONT_6X10,
    pixelcolor::{Rgb565, Rgb888},
    prelude::RgbColor,
};
use futures_executor::block_on;

#[test]
fn shared_conway_renderer_has_a_reviewed_memory_preview() -> Result<(), Box<dyn std::error::Error>>
{
    let cyd_memory = CydMemory::new(
        Size::new(320, 240),
        Rgb888::new(4, 4, 4),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    let mut display = cyd_memory.display();
    let mut app = ConwayApp::new();
    block_on(app.render(&mut display))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    assert_framebuffer_matches_expected_png(&cyd_memory, env!("CARGO_MANIFEST_DIR"), "conway.png")?;
    Ok(())
}

#[test]
fn shared_conway_memory_runs_public_input_tick_and_power_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let cyd_memory = CydMemory::new(
        Size::new(320, 240),
        Rgb888::new(4, 4, 4),
        Rgb888::new(230, 230, 230),
        &FONT_6X10,
    );
    let mut display = cyd_memory.display();
    let mut app = ConwayApp::new();

    block_on(app.render(&mut display))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let initial_pixels = framebuffer_pixels(&cyd_memory);

    assert_eq!(app.input(ConwayInput::PlayPause), ConwayStatus::Ok);
    assert_eq!(app.input(ConwayInput::Next), ConwayStatus::Ok);
    block_on(app.render(&mut display))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    assert_ne!(initial_pixels, framebuffer_pixels(&cyd_memory));

    assert_eq!(app.input(ConwayInput::Power), ConwayStatus::Ok);
    block_on(app.render(&mut display))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    assert!(
        framebuffer_pixels(&cyd_memory)
            .iter()
            .all(|pixel| *pixel == Rgb565::BLACK)
    );
    Ok(())
}

fn framebuffer_pixels(cyd_memory: &CydMemory) -> Vec<Rgb565> {
    (0..240)
        .flat_map(|position_y| {
            (0..320).map(move |position_x| cyd_memory.pixel(position_x, position_y))
        })
        .collect()
}
