#![cfg(feature = "host")]

use device_envoy_core::UnwrapInfallible;
use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{
        CydFrame, DrawItem, Image565View,
        tiling::{TileGrid, max_rectangle_pixel_count},
    },
    touch::TouchEvent,
};
use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
use embedded_graphics::{
    Drawable, Pixel,
    mono_font::ascii::FONT_9X15_BOLD,
    pixelcolor::{Rgb565, Rgb888},
    prelude::{IntoStorage, Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
};
use std::error::Error;

fn comparison_scene<F: CydFrame>(frame: &mut F) {
    comparison_scene_local(frame, Point::zero());
}

fn comparison_scene_local<F: CydFrame>(frame: &mut F, screen_origin: Point) {
    // `frame_mut` gives a regional frame local to its rectangle, so translate
    // this screen-coordinate scene explicitly for that strategy. The tiled
    // callback uses screen coordinates and therefore calls `comparison_scene`.
    let translate = |point: Point| point - screen_origin;
    CydFrame::clear(frame);
    Rectangle::new(translate(Point::new(21, 17)), Size::new(211, 67))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(frame)
        .unwrap_infallible();
    Rectangle::new(translate(Point::new(143, 91)), Size::new(119, 83))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(frame)
        .unwrap_infallible();
    Rectangle::new(translate(Point::new(271, 19)), Size::new(17, 39))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        .draw(frame)
        .unwrap_infallible();
    Pixel(translate(Point::new(306, 211)), Rgb565::WHITE)
        .draw(frame)
        .unwrap_infallible();
}

fn comparison_scene_pixel(position_x: usize, position_y: usize) -> Rgb565 {
    let point = Point::new(position_x as i32, position_y as i32);
    if point == Point::new(306, 211) {
        Rgb565::WHITE
    } else if Rectangle::new(Point::new(271, 19), Size::new(17, 39)).contains(point) {
        Rgb565::BLUE
    } else if Rectangle::new(Point::new(143, 91), Size::new(119, 83)).contains(point) {
        Rgb565::GREEN
    } else if Rectangle::new(Point::new(21, 17), Size::new(211, 67)).contains(point) {
        Rgb565::RED
    } else {
        Rgb565::BLACK
    }
}

fn framebuffer(cyd: &CydMemory) -> Vec<u16> {
    (0..240)
        .flat_map(|position_y| {
            (0..320).map(move |position_x| cyd.pixel(position_x, position_y).into_storage())
        })
        .collect()
}

#[test]
fn cyd_drawing_strategies_produce_identical_framebuffers()
-> Result<(), device_envoy_core::memory::Error> {
    let full_frame = futures_executor::block_on(async {
        let cyd = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let mut display = cyd.display();
        let mut frame = display.full_frame_mut();
        let frame_size = frame.rectangle().size;
        assert_eq!((frame_size.width, frame_size.height), (320, 240));
        comparison_scene(&mut frame);
        frame.flush().await?;
        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd)
    })?;

    let regional_frames = futures_executor::block_on(async {
        let cyd = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let mut display = cyd.display();
        let mut actual_dimensions = Vec::new();
        for top_left in [
            Point::new(0, 0),
            Point::new(160, 0),
            Point::new(0, 120),
            Point::new(160, 120),
        ] {
            let rectangle = Rectangle::new(top_left, Size::new(160, 120));
            let mut frame = display.frame_mut(rectangle);
            let frame_size = frame.rectangle().size;
            actual_dimensions.push((frame_size.width, frame_size.height));
            comparison_scene_local(&mut frame, top_left);
            frame.flush().await?;
        }
        assert_eq!(actual_dimensions, [(160, 120); 4]);
        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd)
    })?;

    let tiled_frames = futures_executor::block_on(async {
        let cyd = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let mut display = cyd.display();
        let mut maximum_dimensions = (0, 0);
        display
            .for_each_tile(
                TileGrid::new(Point::zero(), Size::new(320, 240), 4, 3),
                |frame| {
                    let frame_size = frame.rectangle().size;
                    maximum_dimensions.0 = maximum_dimensions.0.max(frame_size.width);
                    maximum_dimensions.1 = maximum_dimensions.1.max(frame_size.height);
                    comparison_scene(frame);
                },
            )
            .await?;
        assert_eq!(maximum_dimensions, (80, 80));
        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd)
    })?;

    let contiguous = futures_executor::block_on(async {
        let cyd = CydMemory::new(
            Size::new(320, 240),
            Rgb888::BLACK,
            Rgb888::WHITE,
            &FONT_9X15_BOLD,
        );
        let mut display = cyd.display();
        let pixels = (0..240).flat_map(|position_y| {
            (0..320).map(move |position_x| comparison_scene_pixel(position_x, position_y))
        });
        display.fill_contiguous_full(pixels)?;
        Ok::<CydMemory, device_envoy_core::memory::Error>(cyd)
    })?;

    let expected = framebuffer(&full_frame);
    assert_eq!(framebuffer(&regional_frames), expected);
    assert_eq!(framebuffer(&tiled_frames), expected);
    assert_eq!(framebuffer(&contiguous), expected);

    let full_pixels = 320 * 240;
    let largest_region_pixels = max_rectangle_pixel_count(
        Rectangle::new(Point::zero(), Size::new(160, 120)),
        Rectangle::new(Point::new(160, 120), Size::new(160, 120)),
    );
    let tile_pixels =
        TileGrid::new(Point::zero(), Size::new(320, 240), 4, 3).max_tile_pixel_count();
    assert_eq!(
        (full_pixels, largest_region_pixels, tile_pixels),
        (76800, 19200, 6400)
    );
    Ok(())
}

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
        let mut frame =
            device_envoy_core::cyd::backend::DisplayBackend::frame_mut_with_tile_top_left(
                &mut display,
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
