use device_envoy_conway_core::{
    Conway, ConwayCommand, ConwayStatus, DEFAULT_SEARCH_ITERATIONS_PER_STEP,
};
use device_envoy_core::{
    ir::kepler::{IrKepler, KeplerKeys},
    led2d::Led2d,
};
use embassy_futures::{
    select::{Either, select},
    yield_now,
};
use embassy_time::{Duration, Instant, Timer};

pub async fn conway_with_led2d_ir_kepler<const W: usize, const H: usize, L, I>(
    led2d: L,
    ir_kepler: I,
) -> !
where
    L: Led2d<W, H>,
    I: IrKepler,
{
    assert!(W > 0, "Conway width must be greater than zero");
    assert!(H > 0, "Conway height must be greater than zero");

    let mut conway = Conway::<H, W>::new(random_seed());

    loop {
        led2d.write_frame(conway.frame());

        let status = if conway.is_searching() {
            match select(advance_search_step(&mut conway), ir_kepler.wait_for_press()).await {
                Either::First(status) => status,
                Either::Second(kepler_key) => {
                    conway.command(conway_command_for_kepler_key(kepler_key))
                }
            }
        } else {
            let frame_duration = Duration::from_millis(conway.tick_interval_ms() as u64);
            match select(Timer::after(frame_duration), ir_kepler.wait_for_press()).await {
                Either::First(_) => conway.tick(),
                Either::Second(kepler_key) => {
                    conway.command(conway_command_for_kepler_key(kepler_key))
                }
            }
        };

        if matches!(
            status,
            ConwayStatus::Found | ConwayStatus::NotFound | ConwayStatus::Cancelled
        ) {
            led2d.write_frame(conway.frame());
        }
    }
}

async fn advance_search_step<const H: usize, const W: usize>(
    conway: &mut Conway<H, W>,
) -> ConwayStatus {
    let status = conway.advance_search(DEFAULT_SEARCH_ITERATIONS_PER_STEP);
    yield_now().await;
    status
}

fn conway_command_for_kepler_key(kepler_key: KeplerKeys) -> ConwayCommand {
    match kepler_key {
        KeplerKeys::Power => ConwayCommand::Power,
        KeplerKeys::Mode => ConwayCommand::Mode,
        KeplerKeys::PlayPause => ConwayCommand::PlayPause,
        KeplerKeys::Prev => ConwayCommand::Previous,
        KeplerKeys::Next => ConwayCommand::Next,
        KeplerKeys::Minus => ConwayCommand::SpeedDown,
        KeplerKeys::Plus => ConwayCommand::SpeedUp,
        KeplerKeys::Num(number) => ConwayCommand::Pattern(number as usize),
        KeplerKeys::Mute | KeplerKeys::Eq | KeplerKeys::Repeat | KeplerKeys::USd => {
            ConwayCommand::Noop
        }
    }
}

fn random_seed() -> u32 {
    (Instant::now().as_millis() ^ 0x9e37_79b9) as u32
}
