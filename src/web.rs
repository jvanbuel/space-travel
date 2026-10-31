//! Browser front end (ratzilla + WebAssembly).

use std::{cell::RefCell, rc::Rc};

use ratzilla::{DomBackend, WebRenderer, event::KeyCode, ratatui::Terminal, web_sys};
use wasm_bindgen::{JsCast, closure::Closure};

use crate::app::{App, Key};

pub fn run() -> std::io::Result<()> {
    let app = Rc::new(RefCell::new(App::new()));
    // Render into its own container: on resize ratzilla clears the grid's
    // parent, which would otherwise wipe the touch controls and help panel.
    let mut terminal = Terminal::new(DomBackend::new_by_id("game")?)?;

    // Listen for keys on the window, not through `on_key_event`: ratzilla
    // rebuilds its grid element on resize without re-attaching that listener,
    // so input would stop after a phone rotation or window resize.
    let on_key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new({
        let app = app.clone();
        move |e: web_sys::KeyboardEvent| {
            if e.ctrl_key() || e.alt_key() || e.meta_key() {
                return;
            }
            if let Some(key) = map_key(KeyCode::from(e.clone())) {
                // Keep arrow keys from scrolling a page this is embedded in.
                e.prevent_default();
                app.borrow_mut().on_key(key);
            }
        }
    });
    web_sys::window()
        .ok_or_else(|| std::io::Error::other("no window"))?
        .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
        .map_err(|_| std::io::Error::other("cannot listen for keys"))?;
    on_key.forget(); // lives for the whole page

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
