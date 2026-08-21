use device_envoy_core::{
    UnwrapInfallible,
    cyd::{CydDisplay, display::CydFrame},
    led2d::Frame2d,
};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Primitive, RgbColor},
    primitives::{PrimitiveStyle, Rectangle},
};
use smart_leds::{RGB8, colors};

use crate::conway::{
    AutoResetTracker, Board, Pattern, PredecessorSearch, RandomSymmetryMode, SearchOutcome,
    SearchStep,
};

/// Default Conway board width used by the browser and CYD examples.
pub const WIDTH: usize = 16;
/// Default Conway board height used by the browser and CYD examples.
pub const HEIGHT: usize = 16;

const PATTERNS: [Pattern; 10] = [
    Pattern::Glider,
    Pattern::Random,
    Pattern::Blinker,
    Pattern::Toad,
    Pattern::Beacon,
    Pattern::Lwss,
    Pattern::Block,
    Pattern::Pentadecathlon,
    Pattern::Cross,
    Pattern::Custom9,
];

const ALIVE_COLORS: [RGB8; 6] = [
    colors::LIME,
    colors::CYAN,
    colors::MAGENTA,
    colors::ORANGE,
    colors::YELLOW,
    colors::WHITE,
];

/// Inputs accepted by the platform-neutral Conway application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConwayInput {
    /// Begin predecessor search.
    Previous,
    /// Advance one generation while paused.
    Next,
    /// Toggle automatic stepping.
    PlayPause,
    /// Cancel predecessor search.
    Cancel,
    /// Increase simulation speed.
    SpeedUp,
    /// Decrease simulation speed.
    SpeedDown,
    /// Select a pattern preset by index.
    Pattern(u8),
    /// Regenerate the current pattern or cycle random symmetry.
    Repeat,
    /// Return from random symmetry to plain random.
    UndoSymmetry,
    /// Cycle the alive-cell color.
    Mode,
    /// Toggle the LED display power state.
    Power,
}

/// Result of applying a Conway input or advancing the application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConwayStatus {
    /// The application is idle or accepted the input.
    Ok,
    /// A predecessor search is running.
    Searching,
    /// A predecessor was found.
    Found,
    /// No predecessor was found.
    NotFound,
    /// A search was cancelled.
    Cancelled,
    /// The simulation is paused.
    Paused,
    /// The input was not recognized.
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Speed {
    Slow,
    Medium,
    Fast,
}

impl Speed {
    const fn slower(self) -> Self {
        match self {
            Self::Slow => Self::Slow,
            Self::Medium => Self::Slow,
            Self::Fast => Self::Medium,
        }
    }

    const fn faster(self) -> Self {
        match self {
            Self::Slow => Self::Medium,
            Self::Medium => Self::Fast,
            Self::Fast => Self::Fast,
        }
    }

    const fn duration_ms(self) -> u32 {
        match self {
            Self::Slow => 500,
            Self::Medium => 160,
            Self::Fast => 50,
        }
    }
}

/// Shared Conway state machine used by browser, hardware, and memory adapters.
pub struct ConwayApp {
    board: Board<HEIGHT, WIDTH>,
    pattern_index: usize,
    random_symmetry_mode: RandomSymmetryMode,
    search: Option<PredecessorSearch<HEIGHT, WIDTH>>,
    paused: bool,
    speed: Speed,
    auto_reset_tracker: AutoResetTracker,
    random_seed_state: u32,
    color_index: usize,
    alive_color: RGB8,
    display_powered: bool,
}

impl ConwayApp {
    /// Construct the deterministic default simulation.
    #[must_use]
    pub fn new() -> Self {
        let mut board = Board::new();
        let pattern_index = 1;
        board.add_pattern_with_seed_and_random_symmetry(
            PATTERNS[pattern_index],
            0x9e37_79b9,
            RandomSymmetryMode::None,
        );
        Self {
            auto_reset_tracker: AutoResetTracker::new(&board),
            board,
            pattern_index,
            random_symmetry_mode: RandomSymmetryMode::None,
            search: None,
            paused: false,
            speed: Speed::Medium,
            random_seed_state: 0x9e37_79b9,
            color_index: 1,
            alive_color: colors::CYAN,
            display_powered: true,
        }
    }

    /// Apply one user input.
    pub fn input(&mut self, input: ConwayInput) -> ConwayStatus {
        match input {
            ConwayInput::Previous => {
                self.search = Some(PredecessorSearch::new(self.board));
                ConwayStatus::Searching
            }
            ConwayInput::Next => {
                if self.paused {
                    self.step_board();
                }
                ConwayStatus::Ok
            }
            ConwayInput::PlayPause => {
                self.paused = !self.paused;
                ConwayStatus::Ok
            }
            ConwayInput::Cancel => {
                self.search = None;
                ConwayStatus::Cancelled
            }
            ConwayInput::SpeedUp => {
                self.speed = self.speed.faster();
                ConwayStatus::Ok
            }
            ConwayInput::SpeedDown => {
                self.speed = self.speed.slower();
                ConwayStatus::Ok
            }
            ConwayInput::Pattern(index) => {
                self.set_pattern(index as usize);
                ConwayStatus::Ok
            }
            ConwayInput::Repeat => {
                if PATTERNS[self.pattern_index] == Pattern::Random {
                    self.random_symmetry_mode = self.random_symmetry_mode.next();
                }
                self.reseed_current_pattern();
                ConwayStatus::Ok
            }
            ConwayInput::UndoSymmetry => {
                if PATTERNS[self.pattern_index] == Pattern::Random
                    && self.random_symmetry_mode != RandomSymmetryMode::None
                {
                    self.pattern_index = 1;
                    self.reseed_current_pattern();
                }
                ConwayStatus::Ok
            }
            ConwayInput::Mode => {
                self.color_index = (self.color_index + 1) % ALIVE_COLORS.len();
                self.alive_color = ALIVE_COLORS[self.color_index];
                ConwayStatus::Ok
            }
            ConwayInput::Power => {
                self.display_powered = !self.display_powered;
                ConwayStatus::Ok
            }
        }
    }

    /// Advance a search batch or one simulation generation.
    pub fn tick(&mut self) -> ConwayStatus {
        if let Some(search) = &mut self.search {
            return match search.advance(256) {
                SearchStep::Progress { .. } => ConwayStatus::Searching,
                SearchStep::Outcome(SearchOutcome::Found(board)) => {
                    self.board = board;
                    self.search = None;
                    self.auto_reset_tracker.reset(&self.board);
                    ConwayStatus::Found
                }
                SearchStep::Outcome(SearchOutcome::NotFound) => {
                    self.search = None;
                    ConwayStatus::NotFound
                }
                SearchStep::Outcome(SearchOutcome::Cancelled) => {
                    self.search = None;
                    ConwayStatus::Cancelled
                }
            };
        }
        if self.paused {
            ConwayStatus::Paused
        } else {
            self.step_board();
            ConwayStatus::Ok
        }
    }

    /// Render the current board or predecessor-search preview.
    #[must_use]
    pub fn frame(&mut self) -> Frame2d<WIDTH, HEIGHT> {
        if !self.display_powered {
            return Frame2d::new();
        }
        if let Some(search) = &mut self.search
            && let SearchStep::Progress {
                candidate,
                assigned,
                target,
            } = search.advance(1)
        {
            return search_frame(&candidate, &assigned, &target);
        }
        self.board.to_frame(self.alive_color)
    }

    /// Render the current Conway frame onto a generic CYD display.
    pub async fn render<D>(&mut self, display: &mut D) -> Result<(), D::Error>
    where
        D: CydDisplay,
    {
        let led_frame = self.frame();
        let mut frame = display.frame_mut(Rectangle::new(Point::zero(), display.screen_size()));
        frame.fill(Rgb565::BLACK);
        const CELL_SIZE: usize = 12;
        let origin = Point::new(64, 24);
        for row_index in 0..HEIGHT {
            for column_index in 0..WIDTH {
                let color = led_frame[(column_index, row_index)];
                if color == RGB8::default() {
                    continue;
                }
                let color = Rgb888::new(color.r, color.g, color.b);
                Rectangle::new(
                    origin
                        + Point::new(
                            (column_index * CELL_SIZE) as i32,
                            (row_index * CELL_SIZE) as i32,
                        ),
                    Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(Rgb565::from(color)))
                .draw(&mut frame)
                .unwrap_infallible();
            }
        }
        frame.flush().await
    }

    /// Return whether the LED display is currently powered.
    #[must_use]
    pub const fn display_powered(&self) -> bool {
        self.display_powered
    }

    /// Return the current animation interval in milliseconds.
    #[must_use]
    pub const fn tick_interval_ms(&self) -> u32 {
        self.speed.duration_ms()
    }

    fn step_board(&mut self) {
        self.board.step();
        if self
            .auto_reset_tracker
            .observe_generation(&self.board, PATTERNS[self.pattern_index])
        {
            self.reseed_current_pattern();
            self.auto_reset_tracker.reset(&self.board);
        }
    }

    fn set_pattern(&mut self, index: usize) {
        if index < PATTERNS.len() {
            self.pattern_index = index;
            if PATTERNS[index] == Pattern::Random {
                self.random_symmetry_mode = RandomSymmetryMode::None;
            }
            self.reseed_current_pattern();
            self.search = None;
            self.auto_reset_tracker.reset(&self.board);
        }
    }

    fn reseed_current_pattern(&mut self) {
        self.random_seed_state = self
            .random_seed_state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.board = Board::new();
        self.board.add_pattern_with_seed_and_random_symmetry(
            PATTERNS[self.pattern_index],
            self.random_seed_state,
            self.random_symmetry_mode,
        );
    }
}

impl Default for ConwayApp {
    fn default() -> Self {
        Self::new()
    }
}

fn search_frame(
    candidate: &Board<HEIGHT, WIDTH>,
    assigned: &[[bool; WIDTH]; HEIGHT],
    target: &Board<HEIGHT, WIDTH>,
) -> Frame2d<WIDTH, HEIGHT> {
    let mut frame = Frame2d::new();
    for row_index in 0..HEIGHT {
        for col_index in 0..WIDTH {
            if target.cells[row_index][col_index] {
                frame[(col_index, row_index)] = RGB8 { r: 0, g: 10, b: 0 };
            }
            if assigned[row_index][col_index] {
                frame[(col_index, row_index)] = if candidate.cells[row_index][col_index] {
                    colors::RED
                } else {
                    RGB8 { r: 0, g: 0, b: 12 }
                };
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_simulation_is_deterministic_and_steps_when_playing() {
        let mut first = ConwayApp::new();
        let mut second = ConwayApp::new();
        assert_eq!(first.frame().0, second.frame().0);
        assert_eq!(first.tick_interval_ms(), 160);
        assert_eq!(first.tick(), ConwayStatus::Ok);
        assert_eq!(second.tick(), ConwayStatus::Ok);
        assert_eq!(first.frame().0, second.frame().0);
    }

    #[test]
    fn controls_share_the_same_state_machine() {
        let mut app = ConwayApp::new();
        assert_eq!(app.input(ConwayInput::PlayPause), ConwayStatus::Ok);
        assert_eq!(app.tick(), ConwayStatus::Paused);
        assert_eq!(app.input(ConwayInput::Next), ConwayStatus::Ok);
        assert_eq!(app.input(ConwayInput::SpeedDown), ConwayStatus::Ok);
        assert_eq!(app.tick_interval_ms(), 500);
        assert_eq!(app.input(ConwayInput::SpeedUp), ConwayStatus::Ok);
        assert_eq!(app.tick_interval_ms(), 160);
        assert_eq!(app.input(ConwayInput::Previous), ConwayStatus::Searching);
        assert_eq!(app.input(ConwayInput::Cancel), ConwayStatus::Cancelled);
        assert_eq!(app.input(ConwayInput::Pattern(2)), ConwayStatus::Ok);
    }
}
