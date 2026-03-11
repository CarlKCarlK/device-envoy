#![allow(missing_docs)]
//! Wi-Fi enabled 4-digit clock that provisions credentials through `WifiAutoRp`.
//!
//! This example demonstrates how to pair the shared captive-portal workflow with the
//! `ClockLed4` state machine. The `WifiAutoRp` helper owns Wi-Fi onboarding while the
//! clock display reflects progress and, once connected, continues handling user input.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;
use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::button::{Button as _, PressDuration, PressedTo};
use device_envoy_rp::button_watch;
use device_envoy_rp::clock_sync::{
    ClockSync as _, ClockSyncRp, ClockSyncStatic, ONE_DAY, ONE_MINUTE, ONE_SECOND, h12_m_s,
};
use device_envoy_rp::flash_block::FlashBlockRp;
use device_envoy_rp::led4::{
    BlinkState, Led4 as _, Led4Rp, Led4RpStatic, OutputArray, circular_outline_animation,
};
use device_envoy_rp::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy_rp::wifi_auto::{WifiAutoEvent, WifiAutoRp};
use device_envoy_rp::{Error, Result};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::gpio::{self, Level};
use panic_probe as _;

const FAST_MODE_SPEED: f32 = 720.0;
button_watch! {
    ButtonWatch13 {
        pin: PIN_13,
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting Wi-Fi 4-digit clock (WifiAutoRp)");
    let p = embassy_rp::init(Default::default());

    // Use two blocks of flash storage: Wi-Fi credentials + timezone
    let [wifi_credentials_flash_block, timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    // Define HTML to ask for timezone on the captive portal.
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up Wifi via a captive portal. The button pin is used to reset stored credentials.
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 clock
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 data pin
        p.PIO0,    // CYW43 PIO interface
        p.DMA_CH0, // CYW43 DMA channel
        wifi_credentials_flash_block,
        p.PIN_13, // Reset button pin
        PressedTo::Ground,
        "www.picoclock.net", // Captive-portal SSID
        [timezone_field],    // Custom fields to ask for
        spawner,
    )?;

    // Set up the LED4 display.
    let cell_pins = OutputArray::new([
        gpio::Output::new(p.PIN_1, Level::High),
        gpio::Output::new(p.PIN_2, Level::High),
        gpio::Output::new(p.PIN_3, Level::High),
        gpio::Output::new(p.PIN_4, Level::High),
    ]);

    let segment_pins = OutputArray::new([
        gpio::Output::new(p.PIN_5, Level::Low),
        gpio::Output::new(p.PIN_6, Level::Low),
        gpio::Output::new(p.PIN_7, Level::Low),
        gpio::Output::new(p.PIN_8, Level::Low),
        gpio::Output::new(p.PIN_9, Level::Low),
        gpio::Output::new(p.PIN_10, Level::Low),
        gpio::Output::new(p.PIN_11, Level::Low),
        gpio::Output::new(p.PIN_12, Level::Low),
    ]);

    static LED4_STATIC: Led4RpStatic = Led4Rp::new_static();
    let led4 = Led4Rp::new(&LED4_STATIC, cell_pins, segment_pins, spawner)?;

    // Connect Wi-Fi, using the clock display for status.
    let led4_ref = &led4;
    // TODO00 review this possible material change: use WifiAuto's returned trait button directly
    // instead of converting into ButtonWatch13.
    let (stack, button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    led4_ref.write_text(['j', 'o', 'i', 'n'], BlinkState::BlinkingAndOn);
                }
                WifiAutoEvent::Connecting { .. } => {
                    led4_ref.animate_text(circular_outline_animation(true));
                }
                WifiAutoEvent::ConnectionFailed => {
                    led4_ref.write_text(['F', 'A', 'I', 'L'], BlinkState::BlinkingButOff);
                }
            }
            Ok(())
        })
        .await?;
    let button_watch13 = ButtonWatch13::from_button(button, spawner)?;

    led4.write_text(['D', 'O', 'N', 'E'], BlinkState::Solid);
    info!("WiFi connected");

    // Read the timezone offset, an extra field that WiFi portal saved to flash.
    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    // Create a ClockSync device that knows its timezone offset.
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncRp::new_static();
    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    // Start in HH:MM mode
    let mut state = State::HoursMinutes { speed: 1.0 };
    loop {
        state = match state {
            State::HoursMinutes { speed } => {
                state
                    .execute_hours_minutes(speed, &clock_sync, &mut *button_watch13, &led4)
                    .await?
            }
            State::MinutesSeconds => {
                state
                    .execute_minutes_seconds(&clock_sync, &mut *button_watch13, &led4)
                    .await?
            }
            State::EditOffset => {
                state
                    .execute_edit_offset(
                        &clock_sync,
                        &mut *button_watch13,
                        &timezone_field,
                        &led4,
                    )
                    .await?
            }
        };
    }
}

// State machine for 4-digit LED clock display modes and transitions.

#[derive(Debug, defmt::Format, Clone, Copy, PartialEq)]
enum State {
    HoursMinutes { speed: f32 },
    MinutesSeconds,
    EditOffset,
}

impl State {
    async fn execute_hours_minutes(
        self,
        speed: f32,
        clock_sync: &ClockSyncRp,
        button: &mut ButtonWatch13,
        led4: &Led4Rp<'_>,
    ) -> Result<Self> {
        clock_sync.set_speed(speed);
        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        led4.write_text(
            [
                Self::tens_hours(hours),
                Self::ones_digit(hours),
                Self::tens_digit(minutes),
                Self::ones_digit(minutes),
            ],
            BlinkState::Solid,
        );
        clock_sync.set_tick_interval(Some(ONE_MINUTE));
        loop {
            match select(button.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
                // Button pushes
                Either::First(press_duration) => match (press_duration, speed.to_bits()) {
                    (PressDuration::Short, bits) if bits == 1.0f32.to_bits() => {
                        return Ok(Self::MinutesSeconds);
                    }
                    (PressDuration::Short, _) => {
                        return Ok(Self::HoursMinutes { speed: 1.0 });
                    }
                    (PressDuration::Long, _) => {
                        return Ok(Self::EditOffset);
                    }
                },
                // Clock tick
                Either::Second(tick) => {
                    let (hours, minutes, _) = h12_m_s(&tick.local_time);
                    led4.write_text(
                        [
                            Self::tens_hours(hours),
                            Self::ones_digit(hours),
                            Self::tens_digit(minutes),
                            Self::ones_digit(minutes),
                        ],
                        BlinkState::Solid,
                    );
                }
            }
        }
    }

    async fn execute_minutes_seconds(
        self,
        clock_sync: &ClockSyncRp,
        button: &mut ButtonWatch13,
        led4: &Led4Rp<'_>,
    ) -> Result<Self> {
        clock_sync.set_speed(1.0);
        let (_, minutes, seconds) = h12_m_s(&clock_sync.now_local());
        led4.write_text(
            [
                Self::tens_digit(minutes),
                Self::ones_digit(minutes),
                Self::tens_digit(seconds),
                Self::ones_digit(seconds),
            ],
            BlinkState::Solid,
        );
        clock_sync.set_tick_interval(Some(ONE_SECOND));
        loop {
            match select(button.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
                // Button pushes
                Either::First(PressDuration::Short) => {
                    return Ok(Self::HoursMinutes {
                        speed: FAST_MODE_SPEED,
                    });
                }
                Either::First(PressDuration::Long) => {
                    return Ok(Self::EditOffset);
                }
                // Clock tick
                Either::Second(tick) => {
                    let (_, minutes, seconds) = h12_m_s(&tick.local_time);
                    led4.write_text(
                        [
                            Self::tens_digit(minutes),
                            Self::ones_digit(minutes),
                            Self::tens_digit(seconds),
                            Self::ones_digit(seconds),
                        ],
                        BlinkState::Solid,
                    );
                }
            }
        }
    }

    async fn execute_edit_offset(
        self,
        clock_sync: &ClockSyncRp,
        button: &mut ButtonWatch13,
        timezone_field: &TimezoneField,
        led4: &Led4Rp<'_>,
    ) -> Result<Self> {
        info!("Entering edit offset mode");

        // Blink current hours and minutes
        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        led4.write_text(
            [
                Self::tens_hours(hours),
                Self::ones_digit(hours),
                Self::tens_digit(minutes),
                Self::ones_digit(minutes),
            ],
            BlinkState::BlinkingAndOn,
        );

        // Get the current offset minutes from clock (source of truth)
        let mut offset_minutes = clock_sync.offset_minutes();
        info!("Current offset: {} minutes", offset_minutes);

        clock_sync.set_tick_interval(None); // Disable ticks in edit mode
        clock_sync.set_speed(1.0);
        loop {
            info!("Waiting for button press in edit mode");
            match button.wait_for_press_duration().await {
                PressDuration::Short => {
                    info!("Short press detected - incrementing offset");
                    // Increment the offset by 1 hour
                    offset_minutes += 60;
                    const ONE_DAY_MINUTES: i32 = ONE_DAY.as_secs() as i32 / 60;
                    if offset_minutes >= ONE_DAY_MINUTES {
                        offset_minutes -= ONE_DAY_MINUTES;
                    }
                    clock_sync.set_offset_minutes(offset_minutes);
                    info!("New offset: {} minutes", offset_minutes);

                    // Update display (atomic already updated, can use now_local)
                    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
                    info!(
                        "Updated time after offset change: {:02}:{:02}",
                        hours, minutes
                    );
                    led4.write_text(
                        [
                            Self::tens_hours(hours),
                            Self::ones_digit(hours),
                            Self::tens_digit(minutes),
                            Self::ones_digit(minutes),
                        ],
                        BlinkState::BlinkingAndOn,
                    );
                }
                PressDuration::Long => {
                    info!("Long press detected - saving and exiting edit mode");
                    // Save to flash and exit edit mode
                    timezone_field.set_offset_minutes(offset_minutes)?;
                    info!("Offset saved to flash: {} minutes", offset_minutes);
                    return Ok(Self::HoursMinutes { speed: 1.0 });
                }
            }
        }
    }

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        clippy::integer_division_remainder_used,
        reason = "Value < 60 ensures division is safe"
    )]
    const fn tens_digit(value: u8) -> char {
        ((value / 10) + b'0') as char
    }

    #[inline]
    const fn tens_hours(value: u8) -> char {
        if value >= 10 { '1' } else { ' ' }
    }

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        clippy::integer_division_remainder_used,
        reason = "Value < 60 ensures division is safe"
    )]
    const fn ones_digit(value: u8) -> char {
        ((value % 10) + b'0') as char
    }
}
