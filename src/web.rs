//! Browser front end (ratzilla + WebAssembly).

use std::{cell::RefCell, rc::Rc};

use ratzilla::{DomBackend, WebRenderer, event::KeyCode, ratatui::Terminal};

use crate::app::{App, Key};

pub fn run() -> std::io::Result<()> {
    let app = Rc::new(RefCell::new(App::new()));
    let mut terminal = Terminal::new(DomBackend::new()?)?;

    terminal
        .on_key_event({
            let app = app.clone();
            move |k| {
                if let Some(key) = map_key(k.code) {
                    app.borrow_mut().on_key(key);
                }
            }
        })
        .map_err(std::io::Error::other)?;

    // Called once per browser animation frame.
    let mut last = js_sys::Date::now();
    terminal.draw_web(move |f| {
        let now = js_sys::Date::now();
        let dt = ((now - last) / 1000.0).min(0.1); // clamp after a hidden tab
        last = now;
        let mut app = app.borrow_mut();
        app.update(dt);
        app.draw(f);
    });
    Ok(())
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
