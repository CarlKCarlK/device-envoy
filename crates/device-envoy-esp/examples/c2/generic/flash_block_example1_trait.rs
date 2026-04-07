#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    Error, Result,
    flash_block::{FlashBlock, FlashBlockEsp},
    init_and_start,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
struct BootCounter(u8);

impl BootCounter {
    const fn new(value: u8) -> Self {
        Self(value)
    }

    fn increment(self) -> Self {
        Self((self.0 + 1) % 10)
    }
}

fn update_boot_counter_and_clear_scratch(
    boot_counter_flash_block: &mut impl FlashBlock<Error = Error>,
    scratch_flash_block: &mut impl FlashBlock<Error = Error>,
) -> Result<BootCounter> {
    // Load the typed value, defaulting to 0 when the block is empty.
    let boot_counter = boot_counter_flash_block
        .load()?
        .unwrap_or(BootCounter::new(0))
        .increment();

    // Save the updated value back to flash.
    boot_counter_flash_block.save(&boot_counter)?;

    // Clear the extra scratch block.
    scratch_flash_block.clear()?;

    Ok(boot_counter)
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let [mut boot_counter_flash_block, mut scratch_flash_block] =
        FlashBlockEsp::new_array::<2>(p.FLASH)?;

    let boot_counter = update_boot_counter_and_clear_scratch(
        &mut boot_counter_flash_block,
        &mut scratch_flash_block,
    )?;
    info!("boot counter: {}", boot_counter.0);

    core::future::pending().await
}
