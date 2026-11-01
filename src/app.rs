//! Platform-independent game shell: fixed-rate ticking, input mapping, and
//! the choice of engine. `tui.rs` (terminal) and `web.rs` (browser canvas)
//! draw whatever [`Sim`] is running.

use crate::game::{Game, button};
use crate::pdp7::original::Original;
use crate::sim::Sim;

/// Input, already translated from crossterm / browser key events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
}

/// Which engine runs the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The original 1970 machine code on the emulated PDP-7, with its own
    /// software floating point (`original/fop.s`).
    Original,
    /// The Rust translation in `game.rs`, with modern 64-bit floats.
    Modern,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Original => "1970 machine code",
            Mode::Modern => "Rust, 64-bit floats",
        }
    }
}

/// Simulation passes per real second. One pass is about one simulated
/// second, so this is also the time-warp factor.
const TICK_HZ: f64 = 20.0;
pub const TICK: f64 = 1.0 / TICK_HZ;
/// How long a key press keeps a "held" button down. OS key repeat refreshes
/// it, so holding a key gives continuous thrust or rotation.
const HOLD: f64 = 0.12;

pub struct App {
    pub sim: Box<dyn Sim>,
    pub mode: Mode,
    pub quit: bool,
    acc: f64,
    /// Remaining hold time for each level-triggered button.
    held: [f64; 8],
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn engine(mode: Mode) -> Box<dyn Sim> {
    match mode {
        Mode::Original => Box::new(Original::new()),
        Mode::Modern => Box::new(Game::new()),
    }
}

impl App {
    pub fn new() -> Self {
        Self::with_mode(Mode::Original)
    }

    pub fn with_mode(mode: Mode) -> Self {
        Self {
            sim: engine(mode),
            mode,
            quit: false,
            acc: 0.0,
            held: [0.0; 8],
        }
    }

    /// Start a new game with the given engine.
    pub fn restart(&mut self, mode: Mode) {
        self.sim = engine(mode);
        self.mode = mode;
        self.held = [0.0; 8];
    }

    /// Advance the simulation by `dt` real seconds.
    pub fn update(&mut self, dt: f64) {
        self.acc = (self.acc + dt).min(5.0 * TICK);
        while self.acc >= TICK {
            self.acc -= TICK;
            let mut bits = 0;
            for (b, t) in self.held.iter_mut().enumerate() {
                if *t > 0.0 {
                    bits |= 1 << b;
                    *t -= TICK;
                }
            }
            self.sim.tick(bits);
        }
        if self.sim.exit() {
            self.quit = true;
        }
    }

    pub fn on_key(&mut self, key: Key) {
        use button::*;
        let hold = |s: &mut Self, b: u32| s.held[b as usize] = HOLD;
        match key {
            Key::Char('q') | Key::Esc => self.sim.press(EXIT),
            // Forward thrust pushes the ship toward the bottom of the view
            // (see README), so Down/s is forward and Up/w is backward.
            Key::Down | Key::Char('s') | Key::Char('j') => hold(self, FORWARD),
            Key::Up | Key::Char('w') | Key::Char('k') => hold(self, BACKWARD),
            Key::Left | Key::Char('a') | Key::Char('h') => hold(self, ROT_CCW),
            Key::Right | Key::Char('d') | Key::Char('l') => hold(self, ROT_CW),
            Key::Char('+') | Key::Char('=') | Key::Char('x') => self.sim.press(ZOOM_IN),
            Key::Char('-') | Key::Char('_') | Key::Char('z') => self.sim.press(ZOOM_OUT),
            Key::Char('r') | Key::Enter => self.sim.press(RESET),
            // Not in the original: start over, parked on Earth.
            Key::Char('n') => self.restart(self.mode),
            // Not in the original: switch engine (starts a new game).
            Key::Char('m') => self.restart(match self.mode {
                Mode::Original => Mode::Modern,
                Mode::Modern => Mode::Original,
            }),
            _ => {}
        }
    }
}
