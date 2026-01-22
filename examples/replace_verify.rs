#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    led_strip::{Frame1d, RGB8, colors, led_strip},
    led2d,
    led2d::Frame2d,
    led2d::layout::LedLayout,
};
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    LedStrip8 {
        pin: PIN_0,
        len: 8,
        pio: PIO1,
        dma: DMA_CH1,
    }
}

// Two 12x4 panels stacked vertically to create a 12x8 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

led2d! {
    Led12x8 {
        pin: PIN_4,
        led_layout: LED_LAYOUT_12X8_ROTATED,
        font: Led2dFont::Font4x6Trim,
    }
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO1, p.DMA_CH1, spawner)?;
    let led12x8 = Led12x8::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    spawner.spawn(strip_task(led_strip8)).unwrap();
    spawner.spawn(panel_task(led12x8)).unwrap();

    future::pending().await
}

#[embassy_executor::task]
async fn strip_task(led_strip8: &'static LedStrip8) {
    let err = run_strip(led_strip8).await.unwrap_err();
    panic!("{err}");
}

#[embassy_executor::task]
async fn panel_task(led12x8: Led12x8) {
    let err = run_panel(led12x8).await.unwrap_err();
    panic!("{err}");
}

async fn run_strip(led_strip8: &'static LedStrip8) -> Result<Infallible> {
    let delays = [
        embassy_time::Duration::from_millis(0),
        embassy_time::Duration::from_millis(1),
        embassy_time::Duration::from_millis(10),
        embassy_time::Duration::from_millis(250),
    ];
    let animation_delay = embassy_time::Duration::from_millis(120);

    loop {
        for delay in delays {
            let frame1 = strip_dots(1, colors::YELLOW);
            led_strip8.write_frame(frame1).await?;
            embassy_time::Timer::after(delay).await;

            let frame2 = strip_dots(2, colors::YELLOW);
            led_strip8.write_frame(frame2).await?;
            embassy_time::Timer::after(delay).await;

            let (frame3a, frame3b) =
                strip_dots_two_colors(3, colors::YELLOW, colors::BLUE);
            led_strip8
                .animate([(frame3a, animation_delay), (frame3b, animation_delay)])
                .await?;
            embassy_time::Timer::after(delay).await;

            let (frame4a, frame4b) =
                strip_dots_two_colors(4, colors::ORANGE, colors::PURPLE);
            led_strip8
                .animate([(frame4a, animation_delay), (frame4b, animation_delay)])
                .await?;
            embassy_time::Timer::after(delay).await;

            let frame5 = strip_dots(5, colors::WHITE);
            led_strip8.write_frame(frame5).await?;
            embassy_time::Timer::after(delay).await;
        }
    }
}

fn strip_dots<const N: usize>(count: usize, color: RGB8) -> Frame1d<N> {
    assert!(count <= N);
    let mut frame1d = Frame1d::filled(colors::BLACK);
    for dot_index in 0..count {
        frame1d[dot_index] = color;
    }
    frame1d
}

fn strip_dots_two_colors<const N: usize>(
    count: usize,
    first_color: RGB8,
    second_color: RGB8,
) -> (Frame1d<N>, Frame1d<N>) {
    assert!(count <= N);
    let mut frame_a = Frame1d::filled(colors::BLACK);
    let mut frame_b = Frame1d::filled(colors::BLACK);
    for dot_index in 0..count {
        let color_a = if dot_index % 2 == 0 { first_color } else { second_color };
        let color_b = if dot_index % 2 == 0 { second_color } else { first_color };
        frame_a[dot_index] = color_a;
        frame_b[dot_index] = color_b;
    }
    (frame_a, frame_b)
}

async fn run_panel(led12x8: Led12x8) -> Result<Infallible> {
    let delays = [
        embassy_time::Duration::from_millis(0),
        embassy_time::Duration::from_millis(1),
        embassy_time::Duration::from_millis(10),
        embassy_time::Duration::from_millis(250),
    ];
    let animation_delay = embassy_time::Duration::from_millis(120);

    loop {
        for delay in delays {
            let frame1 = panel_text_frame(&led12x8, "1", [colors::GREEN, colors::GREEN])?;
            led12x8.write_frame(frame1).await?;
            embassy_time::Timer::after(delay).await;

            let frame2 = panel_text_frame(&led12x8, "2", [colors::CYAN, colors::CYAN])?;
            led12x8.write_frame(frame2).await?;
            embassy_time::Timer::after(delay).await;

            let frame3a = panel_text_frame(&led12x8, "3", [colors::YELLOW, colors::BLUE])?;
            let frame3b = panel_text_frame(&led12x8, "3", [colors::BLUE, colors::YELLOW])?;
            led12x8
                .animate([(frame3a, animation_delay), (frame3b, animation_delay)])
                .await?;
            embassy_time::Timer::after(delay).await;

            let frame4a = panel_text_frame(&led12x8, "4", [colors::ORANGE, colors::PURPLE])?;
            let frame4b = panel_text_frame(&led12x8, "4", [colors::PURPLE, colors::ORANGE])?;
            led12x8
                .animate([(frame4a, animation_delay), (frame4b, animation_delay)])
                .await?;
            embassy_time::Timer::after(delay).await;

            let frame5 = panel_text_frame(&led12x8, "5", [colors::WHITE, colors::WHITE])?;
            led12x8.write_frame(frame5).await?;
            embassy_time::Timer::after(delay).await;
        }
    }
}

fn panel_text_frame(
    led12x8: &Led12x8,
    text: &str,
    text_colors: [RGB8; 2],
) -> Result<Frame2d<8, 12>> {
    let mut frame2d = Frame2d::new();
    led12x8.write_text_to_frame(text, &text_colors, &mut frame2d)?;
    Ok(frame2d)
}
