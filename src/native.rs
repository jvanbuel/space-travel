//! Terminal front end (crossterm).

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::app::{App, Key};

const FRAME: Duration = Duration::from_millis(33); // ~30 fps

pub fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    let mut last = Instant::now();

    let result = loop {
        if let Err(e) = terminal.draw(|f| app.draw(f)) {
            break Err(e);
        }
        // Wait for input until the next frame is due.
        let timeout = FRAME.saturating_sub(last.elapsed());
        match event::poll(timeout) {
            Ok(true) => {
                if let Ok(Event::Key(k)) = event::read()
                    && k.kind == KeyEventKind::Press
                    && let Some(key) = map_key(k.code)
                {
                    app.on_key(key);
                }
            }
            Ok(false) => {}
            Err(e) => break Err(e),
        }
        let now = Instant::now();
        app.update((now - last).as_secs_f64());
        last = now;
        if app.quit {
            break Ok(());
        }
    };

    ratatui::restore();
    result
}

fn map_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        _ => return None,
    })
}
