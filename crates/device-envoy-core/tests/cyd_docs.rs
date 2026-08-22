#![cfg(feature = "host")]

use device_envoy_core::UnwrapInfallible;
use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{CydFrame, DrawItem, Image565View},
    touch::TouchEvent,
};
use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
use embedded_graphics::{
    Drawable,
    mono_font::ascii::FONT_9X15_BOLD,
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
};
use std::error::Error;

const BITMAP_WIDTH: usize = 64;
const BITMAP_HEIGHT: usize = 64;
const BITMAP_PIXEL_COUNT: usize = BITMAP_WIDTH * BITMAP_HEIGHT;
const BITMAP_COLOR0: u16 = 0xfbe0;
const BITMAP_COLOR1: u16 = 0x051f;
const BITMAP_COLOR2: u16 = 0xffff;

#[test]
fn cyd_memory_bitmap_preview_matches_expected() -> Result<(), Box<dyn Error>> {
    let cyd_memory = futures_executor::block_on(async {
        let mut cyd_memory = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let (display, _) = cyd_memory.parts();
        let mut frame = display.full_frame_mut();

        frame.write_text("Hello CYD");
        DrawItem::Bitmap {
            view: bitmap_view(),
            top_left: Point::new(128, 88),
        }
        .draw(&mut frame);
        frame.flush().await?;

        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd_memory)
    })
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "cyd_memory_bitmap.png",
    )
}

#[test]
fn cyd_trait_preview_matches_expected() -> Result<(), Box<dyn Error>> {
    let cyd_memory = futures_executor::block_on(async {
        let mut cyd_memory = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        cyd_memory.push_touch_event(TouchEvent::Down {
            point: Point::new(160, 120),
        });
        let (display, touch) = cyd_memory.parts();
        let touch_event = touch.read()?;
        let mut frame = display.full_frame_mut();

        frame.write_text("Hello CYD");
        if let Some(TouchEvent::Down { point } | TouchEvent::Move { point }) = touch_event {
            DrawItem::Circle {
                center: (point.x as f32, point.y as f32),
                pixel_radius: 24.0,
                color: Rgb888::RED,
            }
            .draw(&mut frame);
        }
        frame.flush().await?;

        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd_memory)
    })
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "cyd_trait_preview.png",
    )
}

#[test]
fn cyd_frame_mut_with_tile_top_left_preview_matches_expected() -> Result<(), Box<dyn Error>> {
    let cyd_memory = futures_executor::block_on(async {
        let cyd_memory = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let mut display = cyd_memory.display();
        let mut frame = display.frame_mut_with_tile_top_left(
            Rectangle::new(Point::new(32, 24), Size::new(48, 32)),
            Point::new(32, 24),
        );
        frame.fill(Rgb565::GREEN);
        Rectangle::new(Point::new(36, 28), Size::new(6, 6))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(&mut frame)
            .unwrap_infallible();
        Rectangle::new(Point::new(70, 46), Size::new(6, 6))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(&mut frame)
            .unwrap_infallible();
        frame.flush().await?;

        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd_memory)
    })
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "cyd_frame_mut_with_tile_top_left_preview.png",
    )
}

#[test]
fn cyd_frame_mut_preview_matches_expected() -> Result<(), Box<dyn Error>> {
    let cyd_memory = futures_executor::block_on(async {
        let cyd_memory = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let mut display = cyd_memory.display();
        let mut frame = display.frame_mut(Rectangle::new(Point::new(10, 10), Size::new(50, 40)));
        frame.fill(Rgb565::RED);
        frame.flush().await?;

        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd_memory)
    })
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

    assert_framebuffer_matches_expected_png(
        &cyd_memory,
        env!("CARGO_MANIFEST_DIR"),
        "cyd_frame_mut_preview.png",
    )
}

const fn cyd_trait_bitmap_pixels() -> [u16; BITMAP_PIXEL_COUNT] {
    let mut pixels = [0u16; BITMAP_PIXEL_COUNT];
    let mut y = 0;
    while y < BITMAP_HEIGHT {
        let mut x = 0;
        while x < BITMAP_WIDTH {
            let edge = x < 2 || y < 2 || x >= BITMAP_WIDTH - 2 || y >= BITMAP_HEIGHT - 2;
            let diagonal = x == y || x + y == BITMAP_WIDTH - 1;
            pixels[y * BITMAP_WIDTH + x] = if edge {
                BITMAP_COLOR2
            } else if diagonal {
                BITMAP_COLOR1
            } else {
                BITMAP_COLOR0
            };
            x += 1;
        }
        y += 1;
    }
    pixels
}

static BITMAP_PIXELS: [u16; BITMAP_PIXEL_COUNT] = cyd_trait_bitmap_pixels();

fn bitmap_view() -> Image565View {
    Image565View::new(
        &BITMAP_PIXELS,
        Size::new(BITMAP_WIDTH as u32, BITMAP_HEIGHT as u32),
    )
}
