//! Browser front end: draws the display list on an HTML canvas, the way the
//! PDP-7's Graphic-II vector display drew it. The beam traced lines rather
//! than lighting pixels, at four intensities, on a phosphor that glowed and
//! faded. Coordinates are the display's own (0..1023, y up).

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent, Window};

use crate::app::{App, Key};
use crate::game::{Game, Mark};

/// Part of the display shown: the 768-unit view box (x 127..895,
/// y 255..1023) with the velocity bar and text line below it and a margin.
const X0: f64 = 107.0;
const X1: f64 = 915.0;
const Y0: f64 = -14.0;
const Y1: f64 = 1043.0;

/// Phosphor colour (R, G, B) and the brightness of intensities 0-3.
const PHOSPHOR: (u8, u8, u8) = (130, 255, 160);
const INTENS: [f64; 4] = [0.5, 0.66, 0.83, 1.0];
/// How much of the previous frame survives: a short phosphor afterglow.
const PERSIST: f64 = 0.45;

/// The animation-frame callback, which re-schedules itself.
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

fn err(msg: &str) -> std::io::Error {
    std::io::Error::other(msg.to_string())
}

pub fn run() -> std::io::Result<()> {
    let window = web_sys::window().ok_or_else(|| err("no window"))?;
    let canvas: HtmlCanvasElement = window
        .document()
        .and_then(|d| d.get_element_by_id("screen"))
        .ok_or_else(|| err("no #screen canvas"))?
        .dyn_into()
        .map_err(|_| err("#screen is not a canvas"))?;
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .ok_or_else(|| err("no 2d canvas context"))?
        .dyn_into()
        .map_err(|_| err("not a 2d context"))?;
    let app = Rc::new(RefCell::new(App::new()));

    let on_key = Closure::<dyn FnMut(KeyboardEvent)>::new({
        let app = app.clone();
        move |e: KeyboardEvent| {
            if e.ctrl_key() || e.alt_key() || e.meta_key() {
                return;
            }
            let name = e.key();
            if let Some(key) = map_key(&name) {
                if name.starts_with("Arrow") {
                    // Keep arrow keys from scrolling a page this is embedded in.
                    e.prevent_default();
                }
                app.borrow_mut().on_key(key);
            }
        }
    });
    window
        .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
        .map_err(|_| err("cannot listen for keys"))?;
    on_key.forget(); // lives for the whole page

    // One simulation update and redraw per animation frame.
    let frame: FrameCallback = Rc::new(RefCell::new(None));
    let first = frame.clone();
    let mut last: Option<f64> = None;
    let win = window.clone();
    *first.borrow_mut() = Some(Closure::new(move |t: f64| {
        // Clamp so a hidden tab does not fast-forward the game.
        let dt = last.map_or(0.0, |l| ((t - l) / 1000.0).clamp(0.0, 0.1));
        last = Some(t);
        app.borrow_mut().update(dt);
        draw(&win, &canvas, &ctx, &app.borrow().game, t);
        if let Some(f) = frame.borrow().as_ref() {
            let _ = win.request_animation_frame(f.as_ref().unchecked_ref());
        }
    }));
    if let Some(f) = first.borrow().as_ref() {
        window
            .request_animation_frame(f.as_ref().unchecked_ref())
            .map_err(|_| err("no animation frames"))?;
    }
    Ok(())
}

fn map_key(name: &str) -> Option<Key> {
    Some(match name {
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Enter" => Key::Enter,
        "Escape" => Key::Esc,
        _ => {
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Char(c),
                _ => return None,
            }
        }
    })
}

fn rgba(intensity: f64, alpha: f64) -> String {
    let (r, g, b) = PHOSPHOR;
    format!("rgba({r},{g},{b},{})", intensity * alpha)
}

/// Maps display coordinates to canvas pixels.
struct View {
    scale: f64,
    left: f64,
    top: f64,
    /// Device pixels per CSS pixel.
    dpr: f64,
}

impl View {
    fn x(&self, x: f64) -> f64 {
        self.left + (x - X0) * self.scale
    }
    fn y(&self, y: f64) -> f64 {
        self.top + (Y1 - y) * self.scale
    }
}

fn draw(
    win: &Window,
    canvas: &HtmlCanvasElement,
    ctx: &CanvasRenderingContext2d,
    g: &Game,
    t: f64,
) {
    // Match the canvas buffer to its on-screen size (this also clears it).
    let dpr = win.device_pixel_ratio().max(1.0);
    // Capped, so a layout glitch cannot ask for an enormous buffer.
    let w = (f64::from(canvas.client_width()) * dpr)
        .round()
        .clamp(1.0, 4096.0);
    let h = (f64::from(canvas.client_height()) * dpr)
        .round()
        .clamp(1.0, 4096.0);
    if f64::from(canvas.width()) != w || f64::from(canvas.height()) != h {
        canvas.set_width(w as u32);
        canvas.set_height(h as u32);
    }
    let scale = (w / (X1 - X0)).min(h / (Y1 - Y0));
    let v = View {
        scale,
        left: (w - (X1 - X0) * scale) / 2.0,
        top: (h - (Y1 - Y0) * scale) / 2.0,
        dpr,
    };

    // Fade the previous frame instead of erasing it: phosphor afterglow.
    ctx.set_fill_style_str(&format!("rgba(0,0,0,{})", 1.0 - PERSIST));
    ctx.fill_rect(0.0, 0.0, w, h);

    // The fixed part of the original display list: velocity bar, view box
    // and crosshair (st7.s, after `dssca`).
    let frame: [(f64, f64, f64, f64); 7] = [
        (127.0, 250.0, 895.0, 250.0),
        (895.0, 255.0, 895.0, 1023.0),
        (895.0, 1023.0, 127.0, 1023.0),
        (127.0, 1023.0, 127.0, 255.0),
        (127.0, 255.0, 895.0, 255.0),
        (511.0, 255.0, 511.0, 1022.0),
        (127.0, 639.0, 894.0, 639.0),
    ];
    // Drawn a little dimmer than intensity 0 so outlines stand out.
    strokes(ctx, &v, frame.iter().copied(), INTENS[0] * 0.6);

    // Planet outlines, all at intensity 0.
    let outlines = g.display.iter().filter_map(|m| match *m {
        Mark::Line { x1, y1, x2, y2 } => Some((x1.into(), y1.into(), x2.into(), y2.into())),
        Mark::Point { .. } => None,
    });
    strokes(ctx, &v, outlines, INTENS[0]);

    // Body centres, and the velocity indicator (drawn like the Sun's, br3).
    for m in &g.display {
        if let Mark::Point { x, y, br } = *m {
            dot(ctx, &v, x.into(), y.into(), INTENS[usize::from(br)]);
        }
    }
    if let Some(x) = g.vel_x {
        dot(ctx, &v, x.into(), 250.0, INTENS[3]);
    }

    // Text gets no afterglow (changing digits would smear): clear the band
    // below the velocity bar completely.
    ctx.set_fill_style_str("#000");
    ctx.fill_rect(0.0, v.y(244.0), w, h - v.y(244.0));

    // Text line, where the original put it: name, scale, and a blinking
    // "L" / "CL" at intensity 3.
    let font = |px: f64| format!("{px}px \"DejaVu Sans Mono\", Menlo, Consolas, monospace");
    ctx.set_font(&font((22.0 * scale).max(12.0 * dpr)));
    ctx.set_text_baseline("alphabetic");
    let base = v.y(20.0);
    ctx.set_fill_style_str(&rgba(INTENS[1], 1.0));
    ctx.set_text_align("left");
    ctx.fill_text(g.name(), v.x(127.0), base).ok();
    ctx.set_text_align("center");
    ctx.fill_text(&g.scale_text(), v.x(511.0), base).ok();
    if ((t / 400.0) as u64).is_multiple_of(2) {
        ctx.set_fill_style_str(&rgba(INTENS[3], 1.0));
        ctx.set_text_align("right");
        ctx.fill_text(g.status_text(), v.x(895.0), base).ok();
    }

    // Not in the original: altitude and speed, small and dim.
    let km = 6371.0;
    ctx.set_font(&font((15.0 * scale).max(10.0 * dpr)));
    ctx.set_fill_style_str(&rgba(INTENS[0], 0.8));
    ctx.set_text_align("left");
    let readout = format!(
        "alt {:.1} km   v {:.3} km/s",
        g.altitude() * km,
        g.speed() * km
    );
    ctx.fill_text(&readout, v.x(127.0), v.y(170.0)).ok();
}

/// Strokes a batch of segments as a beam would: a soft halo, then the line.
fn strokes(
    ctx: &CanvasRenderingContext2d,
    v: &View,
    segs: impl Iterator<Item = (f64, f64, f64, f64)>,
    intensity: f64,
) {
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    ctx.begin_path();
    for (x1, y1, x2, y2) in segs {
        ctx.move_to(v.x(x1), v.y(y1));
        ctx.line_to(v.x(x2), v.y(y2));
    }
    ctx.set_stroke_style_str(&rgba(intensity, 0.16));
    ctx.set_line_width(4.0 * v.dpr);
    ctx.stroke();
    ctx.set_stroke_style_str(&rgba(intensity, 1.0));
    ctx.set_line_width(1.1 * v.dpr);
    ctx.stroke();
}

/// A body's centre: `dsplanet` draws a tiny square of incremental vectors,
/// which on the screen is a bright dot.
fn dot(ctx: &CanvasRenderingContext2d, v: &View, x: f64, y: f64, intensity: f64) {
    let r = (2.0 * v.scale).max(1.4 * v.dpr) * (0.6 + 0.4 * intensity);
    let (cx, cy) = (v.x(x), v.y(y));
    ctx.set_fill_style_str(&rgba(intensity, 0.18));
    ctx.begin_path();
    ctx.arc(cx, cy, r * 3.0, 0.0, std::f64::consts::TAU).ok();
    ctx.fill();
    ctx.set_fill_style_str(&rgba(intensity, 1.0));
    ctx.begin_path();
    ctx.arc(cx, cy, r, 0.0, std::f64::consts::TAU).ok();
    ctx.fill();
}
