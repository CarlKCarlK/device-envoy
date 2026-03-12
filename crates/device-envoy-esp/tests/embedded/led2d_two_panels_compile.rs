//! Embedded compile-only test target for two LED2D panels on distinct RMT channels.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, led2d,
    led2d::{layout::LedLayout, Frame2d, Led2d as _, Led2dFont},
    led_strip::Current,
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_4X2: LedLayout<8, 4, 2> = LedLayout::serpentine_column_major();

led2d! {
    Led2dPanelA {
        font: Led2dFont::Font4x6,
        led_layout: LED_LAYOUT_4X2,
        pin: GPIO10,
        max_frames: 2,
        max_current: Current::Milliamps(120),
        len: 8,
    }
}

#[cfg(target_arch = "xtensa")]
led2d! {
    Led2dPanelB {
        font: Led2dFont::Font4x6,
        max_frames: 2,
        len: 8,
        pin: GPIO48,
        led_layout: LED_LAYOUT_4X2,
    }
}

#[cfg(not(target_arch = "xtensa"))]
led2d! {
    Led2dPanelB {
        led_layout: LED_LAYOUT_4X2,
        max_frames: 2,
        len: 8,
        font: Led2dFont::Font4x6,
        pin: GPIO8,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: init_and_start::rmt_mode::Blocking);

    let led2d_panel_a = Led2dPanelA::new(p.GPIO10, rmt80.channel0, spawner)?;
    #[cfg(target_arch = "xtensa")]
    let led2d_panel_b = Led2dPanelB::new(p.GPIO48, rmt80.channel1, spawner)?;
    #[cfg(not(target_arch = "xtensa"))]
    let led2d_panel_b = Led2dPanelB::new(p.GPIO8, rmt80.channel1, spawner)?;

    led2d_panel_a.write_frame(Frame2d::<4, 2>::new());
    led2d_panel_b.write_frame(Frame2d::<4, 2>::new());

    core::future::pending().await
}
