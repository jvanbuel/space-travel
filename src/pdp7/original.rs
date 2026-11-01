//! The original 1970 Space Travel, running on the emulated PDP-7.
//!
//! The host plays the parts of PDP-7 Unix and the Graphic-II display:
//!
//! - System calls: `sysloc` tells the game where the push-button word is,
//!   `capt` (take over the display) and `time` (the per-frame delay) do
//!   nothing, and `exit` ends the game. `save` (core dump) is what `fop.s`
//!   calls on a division by zero; here the program just carries on.
//! - Push buttons: the game reads a word with button k in bit 17-k.
//! - Display: the game builds a display list in memory each pass. After each
//!   pass, the per-frame part (from `dspl`) is decoded into the same lines
//!   and dots as the Rust translation produces, and the texts are read from
//!   the fixed header.

use std::collections::VecDeque;

use super::{Machine, Stop, image};
use crate::game::Mark;
use crate::sim::Sim;
use crate::tables;

/// Where the push-button flags live (free memory below the program).
const PBS: usize = 0o100;
/// Runaway guard: far more instructions than one pass needs.
const FRAME_LIMIT: u64 = 50_000_000;

pub struct Original {
    m: Machine,
    display: Vec<Mark>,
    vel_x: Option<i32>,
    exit: bool,
    /// Edge-triggered presses waiting their turn, one per frame.
    pending: VecDeque<u32>,
    pulsed: bool,
    /// Instructions run in the last pass.
    pub last_steps: u64,
}

impl Default for Original {
    fn default() -> Self {
        Self::new()
    }
}

impl Original {
    /// Load the program and run its start-up code up to the main loop.
    pub fn new() -> Self {
        let mut g = Original {
            m: Machine::new(),
            display: Vec::new(),
            vel_x: None,
            exit: false,
            pending: VecDeque::new(),
            pulsed: false,
            last_steps: 0,
        };
        g.run_from(image::START);
        g.decode();
        g
    }

    /// Run until the program is back at the top of `loop:`.
    fn run_from(&mut self, mut pc: usize) {
        let start = self.m.steps;
        loop {
            match self.m.run(pc, Some(image::LOOP), FRAME_LIMIT) {
                Stop::Break => break,
                Stop::Sys { pc: at, call } => {
                    self.syscall(call);
                    if self.exit {
                        break;
                    }
                    pc = at + 1;
                }
                other => {
                    // Should not happen; stop rather than spin.
                    debug_assert!(false, "PDP-7 stopped: {other:?}");
                    self.exit = true;
                    break;
                }
            }
        }
        self.last_steps = self.m.steps - start;
    }

    fn syscall(&mut self, call: usize) {
        match call {
            21 => {
                // sysloc: address of a system location; 13 = push buttons
                // (`law 13` loads the whole instruction word, so look at
                // the address bits only.)
                let what = self.m.ac() & 0o17777;
                self.m.set_ac(if what == 13 { PBS as u32 } else { 0 });
            }
            14 => self.exit = true, // exit
            15 => self.m.set_ac(0), // time
            23 | 24 | 1 => {}       // capt, rele, save
            _ => {}
        }
    }

    fn word(&self, addr: usize) -> u32 {
        self.m.mem[addr & 0o17777]
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn flag(&self, addr: usize) -> bool {
        self.word(addr) & 0o400000 != 0
    }

    fn float(&self, addr: usize) -> f64 {
        let e = super::word_to_i32(self.word(addr));
        tables::pdp7((e, self.word(addr + 1), self.word(addr + 2)))
    }

    /// Text from display-list character words (two 7-bit characters each).
    fn text(&self, addr: usize, words: usize) -> String {
        let mut s = String::new();
        for i in 0..words {
            let w = self.word(addr + i);
            for c in [(w >> 7) & 0o177, w & 0o177] {
                if c != 0 {
                    s.push(char::from(c as u8));
                }
            }
        }
        s
    }

    /// Decode the per-frame display list (Graphic-II commands as used by
    /// this program) into dots and line segments.
    fn decode(&mut self) {
        self.display.clear();
        self.vel_x = None;
        let (mut x, mut y, mut br) = (0i32, 0i32, 0u8);
        for addr in (image::DSPL..).take(8192) {
            let w = self.word(addr);
            if w & 0o400000 != 0 {
                break; // display halt: end of list
            }
            match w & 0o770000 {
                // dsetx / dsety (0o20000 is "wait for the beam" only)
                0o140000 | 0o160000 => {
                    if w & 0o4000 != 0 {
                        y = (w & 0o1777) as i32;
                    } else {
                        x = (w & 0o1777) as i32;
                    }
                }
                // vecx / vecy: long vector, 0o2000 = minus
                0o120000 => {
                    let len = (w & 0o1777) as i32 * if w & 0o2000 != 0 { -1 } else { 1 };
                    let (nx, ny) = if w & 0o4000 != 0 {
                        (x, y + len)
                    } else {
                        (x + len, y)
                    };
                    self.display.push(Mark::Line {
                        x1: x,
                        y1: y,
                        x2: nx,
                        y2: ny,
                    });
                    (x, y) = (nx, ny);
                }
                // short vector: 6-bit sign-magnitude dx, dy
                0o220000 => {
                    let sm = |v: u32| {
                        let m = (v & 0o37) as i32;
                        if v & 0o40 != 0 { -m } else { m }
                    };
                    let (nx, ny) = (x + sm(w >> 6), y + sm(w));
                    self.display.push(Mark::Line {
                        x1: x,
                        y1: y,
                        x2: nx,
                        y2: ny,
                    });
                    (x, y) = (nx, ny);
                }
                // incremental steps: `dsplanet`'s tiny square, a dot
                0o240000 | 0o250000 => {
                    if w == 0o246256 {
                        if y == 250 {
                            self.vel_x = Some(x);
                        } else {
                            self.display.push(Mark::Point { x, y, br });
                        }
                    }
                }
                // parameters: intensity when bit 0o4 is set
                _ if (0o40000..0o100000).contains(&w) && w & 0o4 != 0 => {
                    br = (w & 0o3) as u8;
                }
                _ => {}
            }
        }
    }
}

impl Sim for Original {
    fn tick(&mut self, held: u32) {
        if self.exit {
            return;
        }
        // An edge-triggered press shows for one frame, then a frame without
        // it, so the next press of the same button is a new edge.
        let pulse = if self.pulsed {
            0
        } else {
            self.pending.pop_front().unwrap_or(0)
        };
        self.pulsed = pulse != 0;
        // Button k is bit 17-k of the push-button word.
        let bits = held | pulse;
        let word = (0..18)
            .filter(|k| bits & (1 << k) != 0)
            .fold(0, |w, k| w | (1 << (17 - k)));
        self.m.mem[PBS] = word;
        self.run_from(image::LOOP);
        self.decode();
    }
    fn press(&mut self, button: u32) {
        self.pending.push_back(1 << button);
    }
    fn exit(&self) -> bool {
        self.exit
    }
    fn display(&self) -> &[Mark] {
        &self.display
    }
    fn vel_x(&self) -> Option<i32> {
        self.vel_x
    }
    fn name(&self) -> String {
        self.text(image::NAMEDSP, 10)
    }
    fn scale_text(&self) -> String {
        self.text(image::DSSCA, 3)
    }
    fn status_text(&self) -> String {
        self.text(image::DISPCL, 1)
    }
    fn crashed(&self) -> bool {
        self.flag(image::CRFLG)
    }
    fn altitude(&self) -> f64 {
        let (x, y) = (self.float(image::X), self.float(image::Y));
        x.hypot(y) - self.float(image::RPAR)
    }
    fn speed(&self) -> f64 {
        let dx = self.float(image::X) - self.float(image::OX);
        let dy = self.float(image::Y) - self.float(image::OY);
        dx.hypot(dy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, button};

    #[test]
    fn starts_landed_on_earth() {
        let mut g = Original::new();
        for _ in 0..5 {
            g.tick(0);
        }
        assert_eq!(g.name(), "earth");
        assert_eq!(g.scale_text(), "+00");
        assert_eq!(g.status_text(), "L");
        assert!(!g.crashed());
        assert_eq!(g.altitude(), 0.0);
    }

    #[test]
    fn zoom_and_outline() {
        let mut g = Original::new();
        for _ in 0..8 {
            g.press(button::ZOOM_IN);
        }
        for _ in 0..20 {
            g.tick(0);
        }
        assert_eq!(g.scale_text(), "-08");
        let lines = g
            .display()
            .iter()
            .filter(|m| matches!(m, Mark::Line { .. }))
            .count();
        assert!(lines > 20, "{lines}");
    }

    /// The same take-off in both engines, at scale -18 (thrust just over
    /// 1 g). They agree closely until the 1970 code's `fcp` quirk (see
    /// `pdp7::tests::fcp_low_word_quirk`) briefly lands the ship again at
    /// about 60 m; the Rust translation, comparing f64s, keeps climbing.
    #[test]
    fn agrees_with_the_rust_translation_until_the_fcp_quirk() {
        let mut orig = Original::new();
        let mut rust = Game::new();
        for _ in 0..18 {
            Sim::press(&mut orig, button::ZOOM_IN);
            Sim::press(&mut rust, button::ZOOM_IN);
        }
        for _ in 0..40 {
            orig.tick(0);
            Sim::tick(&mut rust, 0);
        }
        assert_eq!(orig.scale_text(), Sim::scale_text(&rust));
        let forward = 1 << button::FORWARD;
        for i in 0..7 {
            orig.tick(forward);
            Sim::tick(&mut rust, forward);
            let (a, b) = (orig.altitude(), Sim::altitude(&rust));
            assert!(
                (a - b).abs() <= 1e-3 * b,
                "tick {i}: {a} (1970) vs {b} (f64)"
            );
            assert!(!orig.flag(image::LANFLG));
        }
        // The next tick, in the listing's arithmetic: "landed" while
        // climbing at about 60 m.
        orig.tick(forward);
        Sim::tick(&mut rust, forward);
        assert!(orig.flag(image::LANFLG), "1970 code lands again");
        assert!(!Sim::status_text(&rust).contains('L'), "f64 keeps flying");
        assert!(orig.speed() == 0.0);
    }

    #[test]
    fn display_stays_in_bounds() {
        let mut g = Original::new();
        // Zoom far in and out; the list must decode without running away.
        for k in 0..30 {
            g.press(if k < 20 {
                button::ZOOM_IN
            } else {
                button::ZOOM_OUT
            });
        }
        for _ in 0..80 {
            g.tick(0);
            assert!(g.display().len() < 8192);
        }
    }

    #[test]
    #[ignore]
    fn frame_cost() {
        let mut g = Original::new();
        for zoom in [0, 2, 2, 2, 2, 4, 4, 4, 8] {
            for _ in 0..zoom {
                g.press(button::ZOOM_IN);
            }
            for _ in 0..2 * zoom + 1 {
                g.tick(0);
            }
            let t = std::time::Instant::now();
            let n = 50;
            let mut steps = 0;
            for _ in 0..n {
                g.tick(0);
                steps += g.last_steps;
            }
            let per = t.elapsed() / n;
            let len = (0..)
                .take_while(|i| g.word(image::DSPL + i) != 0o400000)
                .count();
            println!(
                "scale {}: {} instructions/frame, {:?}/frame, display list {len} words (room: {})",
                g.scale_text(),
                steps / u64::from(n),
                per,
                0o20000 - image::DSPL
            );
        }
    }
}
