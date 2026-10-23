//! Compile-only check that the code the declaration macros generate builds
//! cleanly in a user crate that denies warnings and missing docs.
//!
//! Each macro is declared once publicly and once privately, and each device is
//! constructed, as a typical application would.
//! Run via: `cargo check-all` (xtask compiles this for thumbv6m-none-eabi)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![deny(warnings, missing_docs)]

use defmt_rtt as _;
use device_envoy_rp::{
    Result,
    audio_player::{adpcm_clip, pcm_clip},
    led::{OnLevel, led},
    led_strip::{Current, led_strips},
    led2d::{Led2dFont, layout::LedLayout},
    servo_player,
};
use embassy_executor::Spawner;
use panic_probe as _;

const LED_LAYOUT_4X3: LedLayout<12, 4, 3> = LedLayout::serpentine_column_major();

led! {
    /// A public status LED.
    pub StatusLed { pin: PIN_2 }
}

led! {
    /// A private busy LED.
    BusyLed { pin: PIN_3 }
}

led_strips! {
    /// Public strips on PIO1.
    pub PublicStrips {
        pio: PIO1,

        /// A plain strip.
        PublicStrip { pin: PIN_6, len: 8, max_current: Current::Milliamps(100) },
        /// A panel.
        PublicPanel {
            pin: PIN_7,
            len: 12,
            max_current: Current::Milliamps(100),
            led2d: { led_layout: LED_LAYOUT_4X3, font: Led2dFont::Font3x4Trim },
        },
    }
}

led_strips! {
    /// Private strips on PIO0.
    PrivateStrips {
        /// A plain strip.
        PrivateStrip { pin: PIN_8, len: 8, max_current: Current::Milliamps(100), dma: DMA_CH2 },
        /// A panel.
        PrivatePanel {
            pin: PIN_9,
            len: 12,
            max_current: Current::Milliamps(100),
            dma: DMA_CH3,
            led2d: { led_layout: LED_LAYOUT_4X3, font: Led2dFont::Font3x4Trim },
        },
    }
}

servo_player! {
    /// A public servo player.
    pub PublicServo { pin: PIN_10 }
}

servo_player! {
    /// A private servo player.
    PrivateServo { pin: PIN_12 }
}

pcm_clip! {
    /// A public PCM clip.
    pub PublicPcm {
        file: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../device-envoy-examples-rp/examples/data/audio/0_22050.s16"
        ),
        source_sample_rate_hz: 22_050,
    }
}

adpcm_clip! {
    /// A private ADPCM clip.
    PrivateAdpcm {
        file: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../device-envoy-examples-rp/examples/data/audio/nasa_22k_adpcm.wav"
        ),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<core::convert::Infallible> {
    let p = embassy_rp::init(Default::default());

    let status_led = StatusLed::new(p.PIN_2, OnLevel::High, spawner)?;
    let busy_led = BusyLed::new(p.PIN_3, OnLevel::High, spawner)?;
    let (public_strip, public_panel) =
        PublicStrips::new(p.PIO1, p.PIN_6, p.DMA_CH0, p.PIN_7, p.DMA_CH1, spawner)?;
    let (private_strip, private_panel) =
        PrivateStrips::new(p.PIO0, p.PIN_8, p.DMA_CH2, p.PIN_9, p.DMA_CH3, spawner)?;
    let public_servo = PublicServo::new(p.PIN_10, p.PWM_SLICE5, spawner)?;
    let private_servo = PrivateServo::new(p.PIN_12, p.PWM_SLICE6, spawner)?;
    let public_pcm = PublicPcm::pcm_clip();
    let private_adpcm = PrivateAdpcm::adpcm_clip();

    loop {
        defmt::info!(
            "{}",
            size_of_val(&status_led)
                + size_of_val(&busy_led)
                + size_of_val(public_strip)
                + size_of_val(&public_panel)
                + size_of_val(private_strip)
                + size_of_val(&private_panel)
                + size_of_val(&public_servo)
                + size_of_val(&private_servo)
                + size_of_val(&public_pcm)
                + size_of_val(&private_adpcm),
        );
        embassy_time::Timer::after_secs(1).await;
    }
}
