//! Terminal renderer: draws the display list with ratatui's Braille canvas
//! (2x4 dots per character cell, the finest a terminal can do).

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::Marker,
    text::{Line as TextLine, Span},
    widgets::{
        Block, Paragraph,
        canvas::{Canvas, Line, Points},
    },
};

use crate::app::App;
use crate::game::Mark;

/// Display intensities 0-3, as green phosphor.
const INTENS: [Color; 4] = [
    Color::Rgb(40, 130, 60),
    Color::Rgb(70, 170, 90),
    Color::Rgb(120, 215, 135),
    Color::Rgb(200, 255, 205),
];
const FRAME: Color = Color::Rgb(30, 80, 40);

pub fn draw(app: &App, frame: &mut Frame) {
    let g = app.sim.as_ref();
    // The view is square in screen units; cells are about twice as tall
    // as wide, so it needs twice as many columns as rows. Below it: the
    // velocity indicator, the readout and the help line. Everything sits
    // at the top so a phone's lower screen is free for touch controls.
    let area = frame.area();
    let rows = area.height.saturating_sub(3).min(area.width / 2).max(3);
    let cols = (rows * 2).min(area.width);
    let x0 = area.x + (area.width - cols) / 2;
    let view = Rect::new(x0, area.y, cols, rows).intersection(area);
    let bar = Rect::new(x0, view.bottom(), cols, 1).intersection(area);
    let readout = Rect::new(area.x, bar.bottom(), area.width, 1).intersection(area);
    let help = Rect::new(area.x, readout.bottom(), area.width, 1).intersection(area);

    // Inner size in Braille dots, to size the planet markers.
    let dots_w = f64::from(cols.saturating_sub(2).max(1)) * 2.0;
    let dot = 768.0 / dots_w;

    let display = g.display();
    let canvas = Canvas::default()
        .block(
            Block::bordered()
                .border_style(Style::new().fg(FRAME))
                .title(" space travel ".fg(INTENS[2])),
        )
        .marker(Marker::Braille)
        .x_bounds([127.0, 895.0])
        .y_bounds([255.0, 1023.0])
        .paint(move |ctx| {
            // Crosshair through the ship.
            ctx.draw(&Line::new(511.0, 255.0, 511.0, 1022.0, FRAME));
            ctx.draw(&Line::new(127.0, 639.0, 894.0, 639.0, FRAME));
            ctx.layer();
            for m in display {
                if let Mark::Line { x1, y1, x2, y2 } = *m {
                    ctx.draw(&Line::new(
                        x1.into(),
                        y1.into(),
                        x2.into(),
                        y2.into(),
                        INTENS[0],
                    ));
                }
            }
            ctx.layer();
            for m in display {
                if let Mark::Point { x, y, br } = *m {
                    let (x, y) = (f64::from(x), f64::from(y));
                    let mut pts = vec![(x, y)];
                    if br >= 1 {
                        pts.extend([(x + dot, y), (x, y + dot)]);
                    }
                    if br >= 2 {
                        pts.extend([(x - dot, y), (x, y - dot)]);
                    }
                    ctx.draw(&Points {
                        coords: &pts,
                        color: INTENS[usize::from(br)],
                    });
                }
            }
        });
    frame.render_widget(canvas, view);

    // Horizontal-velocity indicator.
    let vel_x = g.vel_x();
    let bar_canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([127.0, 895.0])
        .y_bounds([0.0, 1.0])
        .paint(move |ctx| {
            ctx.draw(&Line::new(127.0, 0.5, 895.0, 0.5, FRAME));
            ctx.layer();
            if let Some(x) = vel_x {
                let x = f64::from(x);
                ctx.draw(&Points {
                    coords: &[(x - dot, 0.5), (x, 0.0), (x, 0.5), (x, 1.0), (x + dot, 0.5)],
                    color: INTENS[3],
                });
            }
        });
    frame.render_widget(bar_canvas, bar);

    // Readouts, laid out like the original's bottom line: name at the
    // left, scale in the middle, landed / crashed at the right.
    let [left, mid, right] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(Rect::new(x0, readout.y, cols, 1).intersection(readout));
    let bright = Style::new().fg(INTENS[3]);
    frame.render_widget(Paragraph::new(g.name()).style(bright), left);
    frame.render_widget(Paragraph::new(g.scale_text()).style(bright).centered(), mid);
    let status = g.status_text();
    let status_style = if g.crashed() {
        Style::new().fg(Color::LightRed).bold()
    } else {
        bright
    };
    frame.render_widget(
        Paragraph::new(status).style(status_style).right_aligned(),
        right,
    );

    let km = 6371.0;
    let help_line = TextLine::from(vec![
        Span::raw(format!(
            " alt {:>10.1} km  v {:>9.3} km/s ",
            g.altitude() * km,
            g.speed() * km
        )),
        Span::raw(format!(
            "  ↓/s fwd  ↑/w back  ←→ turn  +/- zoom  r clear crash  n new  m engine ({})  q quit",
            app.mode.label()
        ))
        .dark_gray(),
    ]);
    frame.render_widget(Paragraph::new(help_line).dark_gray(), help);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::app::{Key, TICK};

    fn render(app: &App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| draw(app, f)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect()
    }

    #[test]
    fn renders_at_any_size() {
        let mut app = App::new();
        app.update(1.0);
        for (w, h) in [(1, 1), (10, 3), (80, 24), (200, 60)] {
            render(&app, w, h);
        }
    }

    #[test]
    #[ignore]
    fn show() {
        let mut app = App::new();
        for k in std::env::var("KEYS").unwrap_or_default().chars() {
            app.on_key(Key::Char(k));
            app.update(TICK);
        }
        app.update(TICK);
        println!("{}", render(&app, 100, 34));
    }
}
