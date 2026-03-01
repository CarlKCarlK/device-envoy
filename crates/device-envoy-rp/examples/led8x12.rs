#![allow(missing_docs)]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy::button::{Button, PressedTo};
use device_envoy::led_strip::Current;
use device_envoy::led_strip::Gamma;
use device_envoy::led2d;
use device_envoy::led2d::Frame2d;
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use device_envoy::{Error, Result};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::init;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use panic_probe as _;
use smart_leds::{RGB8, colors};

// Two 12x4 panels stacked vertically and rotated 90° CW → 8×12 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X12: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

led2d! {
    Led8x12 {
        pio: PIO0,
        pin: PIN_4,
        dma: DMA_CH0,
        led_layout: LED_LAYOUT_8X12,
        max_current: Current::Milliamps(1000),
        gamma: Gamma::Linear,
        max_frames: 32,
        font: Led2dFont::Font4x6Trim,
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("LED 2D API Exploration (8x12 rotated display)");
    let p = init(Default::default());

    let led8x12 = Led8x12::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    loop {
        info!("Demo 1: Clock-style two-line text");
        demo_clock_text(&led8x12).await?;
        button.wait_for_press().await;

        info!("Demo 2: Colored corners (orientation test)");
        demo_colored_corners(&led8x12).await?;
        button.wait_for_press().await;

        info!("Demo 3: Blink text");
        demo_blink_text(&led8x12).await?;
        button.wait_for_press().await;

        info!("Demo 4: Blink pattern");
        demo_blink_pattern(&led8x12).await?;
        button.wait_for_press().await;

        info!("Demo 5: Rectangle with diagonals (embedded-graphics)");
        demo_rectangle_diagonals_embedded_graphics(&led8x12).await?;
        button.wait_for_press().await;

        info!("Demo 6: Bouncing dot (manual frames)");
        demo_bouncing_dot_manual(&led8x12, &mut button).await?;

        info!("Demo 7: Bouncing dot (animation)");
        demo_bouncing_dot_animation(&led8x12).await?;
        button.wait_for_press().await;
    }
}

/// Display time-like text using two lines (like "12" on top, "34" on bottom).
async fn demo_clock_text(led8x12: &Led8x12) -> Result<()> {
    let colors = [colors::CYAN, colors::MAGENTA, colors::ORANGE, colors::LIME];
    led8x12.write_text("12\n34", &colors).await
}

/// Blink text by constructing frames explicitly.
async fn demo_blink_text(led8x12: &Led8x12) -> Result<()> {
    let mut on_frame = Frame2d::new();
    led8x12.write_text_to_frame("HI", &[colors::YELLOW], &mut on_frame)?;
    led8x12.animate([
        (on_frame, Duration::from_millis(500)),
        (Frame2d::new(), Duration::from_millis(500)),
    ])
}

/// Display colored corners to demonstrate coordinate mapping.
async fn demo_colored_corners(led8x12: &Led8x12) -> Result<()> {
    // Four corners with different colors
    let mut frame = Frame2d::new();
    frame[(0, 0)] = colors::RED; // Top-left
    frame[(Led8x12::WIDTH - 1, 0)] = colors::GREEN; // Top-right
    frame[(0, Led8x12::HEIGHT - 1)] = colors::BLUE; // Bottom-left
    frame[(Led8x12::WIDTH - 1, Led8x12::HEIGHT - 1)] = colors::YELLOW; // Bottom-right

    led8x12.write_frame(frame)?;
    Ok(())
}

/// Blink a pattern by constructing frames explicitly.
async fn demo_blink_pattern(led8x12: &Led8x12) -> Result<()> {
    // Create checkerboard pattern
    let mut on_frame = Frame2d::new();
    for y_index in 0..Led8x12::HEIGHT {
        for x_index in 0..Led8x12::WIDTH {
            if (y_index + x_index) % 2 == 0 {
                on_frame[(x_index, y_index)] = colors::PURPLE;
            }
        }
    }

    led8x12.animate([
        (on_frame, Duration::from_millis(500)),
        (Frame2d::new(), Duration::from_millis(500)),
    ])
}

/// Create a red rectangle border with blue diagonals using embedded-graphics.
async fn demo_rectangle_diagonals_embedded_graphics(led8x12: &Led8x12) -> Result<()> {
    use device_envoy::led_strip::ToRgb888;
    use embedded_graphics::{
        Drawable,
        prelude::*,
        primitives::{Line, PrimitiveStyle, Rectangle},
    };

    let mut frame = Frame2d::new();

    // Use the embedded_graphics crate to draw an image.

    // Draw red rectangle border
    Rectangle::new(Led8x12::TOP_LEFT, Led8x12::SIZE)
        .into_styled(PrimitiveStyle::with_stroke(colors::RED.to_rgb888(), 1))
        .draw(&mut frame)?;

    // Draw blue diagonal lines from corner to corner
    Line::new(Led8x12::TOP_LEFT, Led8x12::BOTTOM_RIGHT)
        .into_styled(PrimitiveStyle::with_stroke(colors::BLUE.to_rgb888(), 1))
        .draw(&mut frame)?;

    Line::new(Led8x12::BOTTOM_LEFT, Led8x12::TOP_RIGHT)
        .into_styled(PrimitiveStyle::with_stroke(colors::BLUE.to_rgb888(), 1))
        .draw(&mut frame)?;

    led8x12.write_frame(frame)
}

struct BouncingDot {
    x_position: isize,
    y_position: isize,
    x_velocity: isize,
    y_velocity: isize,
    x_limit: isize,
    y_limit: isize,
    color_index: usize,
    colors: [RGB8; 3],
    frame_delay: Duration,
}

impl BouncingDot {
    fn new() -> Self {
        const FRAME_DELAY: Duration = Duration::from_millis(50);

        Self {
            x_position: 0,
            y_position: 0,
            x_velocity: 1,
            y_velocity: 1,
            x_limit: Led8x12::WIDTH as isize,
            y_limit: Led8x12::HEIGHT as isize,
            color_index: 0,
            colors: [colors::RED, colors::GREEN, colors::BLUE],
            frame_delay: FRAME_DELAY,
        }
    }

    fn step_axis(position: &mut isize, velocity: &mut isize, limit: isize) -> bool {
        *position += *velocity;
        if (0..limit).contains(position) {
            return false;
        }
        *velocity = -*velocity;
        *position += *velocity; // step back inside
        true
    }

    fn advance(&mut self) -> bool {
        let hit_x = Self::step_axis(&mut self.x_position, &mut self.x_velocity, self.x_limit);
        let hit_y = Self::step_axis(&mut self.y_position, &mut self.y_velocity, self.y_limit);
        hit_x | hit_y
    }

    fn current_color(&self) -> RGB8 {
        self.colors[self.color_index]
    }

    fn advance_color(&mut self) {
        self.color_index = (self.color_index + 1) % self.colors.len();
    }

    async fn run(&mut self, led8x12: &Led8x12) -> Result<Infallible> {
        loop {
            let mut frame = Frame2d::new();
            let x_position =
                usize::try_from(self.x_position).expect("x position must be nonnegative");
            let y_position =
                usize::try_from(self.y_position).expect("y position must be nonnegative");
            assert!(x_position < Led8x12::WIDTH);
            assert!(y_position < Led8x12::HEIGHT);
            frame[(x_position, y_position)] = self.current_color();
            led8x12.write_frame(frame)?;

            if self.advance() {
                self.advance_color();
            }

            Timer::after(self.frame_delay).await;
        }
    }
}

async fn demo_bouncing_dot_manual(led8x12: &Led8x12, button: &mut Button<'_>) -> Result<()> {
    let mut bouncing_dot = BouncingDot::new();
    match select(bouncing_dot.run(led8x12), button.wait_for_press()).await {
        Either::First(result) => result.map(|_| ()),
        Either::Second(_) => Ok(()),
    }
}

/// Bouncing dot using pre-built animation frames.
async fn demo_bouncing_dot_animation(led8x12: &Led8x12) -> Result<()> {
    let mut color_cycle = [colors::CYAN, colors::YELLOW, colors::LIME].iter().cycle();

    // Steps one position coordinate and reports if it hit an edge.
    fn step_and_hit(position: &mut isize, velocity: &mut isize, limit: isize) -> bool {
        *position += *velocity;
        if (0..limit).contains(position) {
            return false;
        }
        *velocity = -*velocity;
        *position += *velocity; // step back inside
        true
    }

    let mut frames = Vec::<_, { Led8x12::MAX_FRAMES }>::new();
    let (mut x, mut y) = (0isize, 0isize);
    let (mut vx, mut vy) = (1isize, 1isize);
    let (x_limit, y_limit) = (Led8x12::WIDTH as isize, Led8x12::HEIGHT as isize);
    let mut color = *color_cycle.next().unwrap();

    for _ in 0..Led8x12::MAX_FRAMES {
        let mut frame = Frame2d::new();
        frame[(x as usize, y as usize)] = color;
        frames
            .push((frame, Duration::from_millis(50)))
            .map_err(|_| Error::FormatError)?;

        if step_and_hit(&mut x, &mut vx, x_limit) | step_and_hit(&mut y, &mut vy, y_limit) {
            color = *color_cycle.next().unwrap();
        }
    }

    led8x12.animate(frames) // Run forever
}
