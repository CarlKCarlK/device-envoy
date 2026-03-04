#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_example_common::conway::{run_conway, ConwayIrReceiver, ConwayLed16x16};
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    esp_hal::gpio::{Level, Output, OutputConfig},
    init_and_start,
    ir::{IrKepler, IrKeplerStatic, KeplerButton},
    led2d,
    led2d::{layout::LedLayout, Frame2d, Led2dFont},
    led_strip::Current,
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_row_major();
const PANEL_16X16_PIN_NUM: u8 = 2;
const IR_PIN_NUM: u8 = 7;

led2d! {
    Led16x16Conway {
        len: 256,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(700),
        font: Led2dFont::Font4x6Trim,
        engine: Engine::Spi,
        max_frames: 30,
    }
}

struct Led16x16ConwayAdapter {
    led16x16_conway: &'static Led16x16Conway,
}

impl ConwayLed16x16 for Led16x16ConwayAdapter {
    fn write_frame16x16(&self, frame16x16: Frame2d<16, 16>) {
        self.led16x16_conway.write_frame2d(frame16x16);
    }
}

struct IrKeplerConwayAdapter<'a> {
    ir_kepler: IrKepler<'a>,
}

impl ConwayIrReceiver for IrKeplerConwayAdapter<'_> {
    async fn wait_for_press(&self) -> KeplerButton {
        self.ir_kepler.wait_for_press().await
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!(
        "Conway: 16x16 SPI on GPIO{}, IR receiver on GPIO{}",
        PANEL_16X16_PIN_NUM, IR_PIN_NUM
    );

    // TODO0 Keep MAX98357A I2S inputs quiet in this non-audio example.
    let _audio_idle_pins = (
        Output::new(p.GPIO21, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO11, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO12, Level::Low, OutputConfig::default()),
    );

    let led16x16_conway = Led16x16Conway::new(p.GPIO2, p.SPI2, spawner)?;
    let led16x16_conway_adapter = Led16x16ConwayAdapter { led16x16_conway };

    static IR_KEPLER_STATIC: IrKeplerStatic = IrKepler::new_static();
    // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0–3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let ir_rmt_channel = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let ir_rmt_channel = rmt80.channel2;
    let ir_kepler = IrKepler::new(&IR_KEPLER_STATIC, p.GPIO7, ir_rmt_channel, spawner)?;
    let ir_kepler_conway_adapter = IrKeplerConwayAdapter { ir_kepler };

    run_conway(&led16x16_conway_adapter, &ir_kepler_conway_adapter).await
}
