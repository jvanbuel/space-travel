//! Platform-independent game state. Everything game-specific goes here;
//! `native.rs` and `web.rs` only feed it input and time, and draw it.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Stylize},
    symbols::Marker,
    widgets::{
        Block, Paragraph,
        canvas::{Canvas, Points},
    },
};

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

#[derive(Default)]
pub struct App {
    /// Simulated seconds since start.
    pub time: f64,
    pub last_key: Option<Key>,
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the simulation by `dt` real seconds.
    pub fn update(&mut self, dt: f64) {
        self.time += dt;
    }

    pub fn on_key(&mut self, key: Key) {
        self.last_key = Some(key);
        if key == Key::Char('q') || key == Key::Esc {
            self.quit = true;
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let [main, status] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

        // World coordinates: x and y both span -100..100 here. Terminal cells are
        // about twice as tall as wide; scale one axis by the area's aspect ratio if
        // you need circles to look round.
        let t = self.time;
        let canvas = Canvas::default()
            .block(Block::bordered().title(" space-travel "))
            .marker(Marker::Braille)
            .x_bounds([-100.0, 100.0])
            .y_bounds([-100.0, 100.0])
            .paint(move |ctx| {
                // Placeholder: one point moving in a circle, so you can see the loop run.
                let p = [(60.0 * t.cos(), 60.0 * t.sin())];
                ctx.draw(&Points {
                    coords: &p,
                    color: Color::Yellow,
                });
            });
        frame.render_widget(canvas, main);

        let msg = format!(
            " t = {:7.2}s   last key: {:?}   q/Esc: quit",
            self.time, self.last_key
        );
        frame.render_widget(Paragraph::new(msg).dark_gray(), status);
    }
}
