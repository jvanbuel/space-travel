//! What the front ends need from a running game, whichever engine runs it:
//! the Rust translation ([`crate::game::Game`], modern `f64` arithmetic) or
//! the original 1970 machine code on the PDP-7 emulator
//! ([`crate::pdp7::original::Original`]).

use crate::game::{Game, Mark};

pub trait Sim {
    /// One pass of the main loop, with these push buttons held
    /// (bit `k` = button `k`, see [`crate::game::button`]).
    fn tick(&mut self, held: u32);
    /// One press of an edge-triggered button (exit, reset, zoom).
    fn press(&mut self, button: u32);
    /// The game asked to exit.
    fn exit(&self) -> bool;
    /// Body centres and outline segments, in display coordinates.
    fn display(&self) -> &[Mark];
    /// Screen x of the horizontal-velocity indicator.
    fn vel_x(&self) -> Option<i32>;
    /// The texts of the original's bottom line.
    fn name(&self) -> String;
    fn scale_text(&self) -> String;
    fn status_text(&self) -> String;
    /// Crashed (the terminal colours `CL`; the canvas only blinks it, like
    /// the original).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn crashed(&self) -> bool;
    /// Height above the current body and speed relative to it, in Earth
    /// radii (per tick). Not in the original display.
    fn altitude(&self) -> f64;
    fn speed(&self) -> f64;
}

impl Sim for Game {
    fn tick(&mut self, held: u32) {
        self.pbson = held;
        Game::tick(self);
    }
    fn press(&mut self, button: u32) {
        Game::press(self, button);
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
        Game::name(self).to_string()
    }
    fn scale_text(&self) -> String {
        Game::scale_text(self)
    }
    fn status_text(&self) -> String {
        Game::status_text(self).to_string()
    }
    fn crashed(&self) -> bool {
        self.crflg
    }
    fn altitude(&self) -> f64 {
        Game::altitude(self)
    }
    fn speed(&self) -> f64 {
        Game::speed(self)
    }
}
