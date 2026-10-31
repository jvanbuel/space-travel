//! The simulation: a translation of `original/st1.s`–`st5.s`.
//!
//! Routine names and variable names follow the assembly so the two can be
//! read side by side. The original keeps nearly everything in globals that
//! routines pass to each other implicitly (`absx`, `dpar`, `spx`, ...); those
//! stay fields here rather than being turned into return values, because
//! several routines depend on what an earlier one left behind.
//!
//! Each [`Game::tick`] is one pass of the original `loop:` and produces a
//! display list in GRAPHIC-2 screen coordinates (0..1023, y up). The view is
//! the box x 127..895, y 255..1023, centred on the ship at (511, 639).

use crate::tables::{self, N, pdp7};

/// Push buttons, as bit numbers of the original `pbson` word (`pbson[k]`).
pub mod button {
    /// Exit the program.
    pub const EXIT: u32 = 0;
    /// Clear the crashed / game-over state.
    pub const RESET: u32 = 1;
    /// Thrust along the heading.
    pub const FORWARD: u32 = 2;
    /// Thrust against the heading.
    pub const BACKWARD: u32 = 3;
    /// Rotate the heading clockwise.
    pub const ROT_CW: u32 = 4;
    /// Rotate the heading counter-clockwise.
    pub const ROT_CCW: u32 = 5;
    /// `scale++`: zoom out.
    pub const ZOOM_OUT: u32 = 6;
    /// `scale--`: zoom in.
    pub const ZOOM_IN: u32 = 7;
}

/// One item of the display list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mark {
    /// A body's centre point, with brightness 0-3.
    Point { x: i32, y: i32, br: u8 },
    /// One arc segment of a body's outline (drawn at intensity 0).
    Line { x1: i32, y1: i32, x2: i32, y2: i32 },
}

/// `inscr`: is the (negated) value inside the 768-unit view? Returns the
/// offset `AC` that the original adds to `dsetx 895` / `dsety 1023`, so
/// screen x = 895 + AC and screen y = 1023 + AC.
fn inscr(fac: f64) -> Option<i32> {
    // `fix` floors (it shifts two's-complement values right).
    let v = (-fac).floor();
    if !(-1.0e9..=1.0e9).contains(&v) {
        return None;
    }
    let ac = v as i32 + 383;
    if ac < 0 {
        return None;
    }
    let ac = ac - 768;
    if ac >= 0 { None } else { Some(ac) }
}

fn pow2(e: i32) -> f64 {
    2f64.powi(e)
}

pub struct Game {
    // ---- bodies ----
    px: [f64; N],
    py: [f64; N],
    pw: [f64; N],
    pww: [f64; N],
    prsq: [f64; N],
    accl: [f64; N],

    // ---- constants ----
    fardst: f64,
    f2pi: f64,
    pid10: f64,
    thrs: f64,
    f400: f64,
    crash: f64,
    ascale: f64,
    sdphi: f64,
    cdphi: f64,

    // ---- ship ----
    /// Body the ship's coordinates are relative to.
    pub par: usize,
    x: f64,
    y: f64,
    ox: f64,
    oy: f64,
    /// Radius of `par`.
    rpar: f64,
    /// Heading as set by the controls.
    sphi: f64,
    cphi: f64,
    /// Heading used for the view (copied from phi once per pass).
    stheta: f64,
    ctheta: f64,
    /// View scale: one screen unit is 2^scale Earth radii.
    pub scale: i32,
    /// Horizontal (tangential) velocity relative to `par`.
    horizv: f64,

    // ---- flags ----
    /// Landed.
    pub lanflg: bool,
    /// Game over: the simulation is frozen.
    pub goflg: bool,
    /// Crash landing.
    pub crflg: bool,
    forflg: bool,
    bacflg: bool,
    /// Current body is too far away to matter.
    grvflg: bool,
    /// `spx` has been computed for the current body.
    inflg: bool,
    /// Never set in the original ("how to set?" in the listing).
    accflg: bool,
    /// Heading lock onto `locpar`; never set in the original either.
    locflg: bool,
    locpar: usize,

    // ---- gravity ----
    ax: f64,
    ay: f64,
    /// Strongest gravitational acceleration this pass, and whose it is.
    maxa: f64,
    maxj: usize,

    // ---- scratch shared between routines ----
    absx: f64,
    absy: f64,
    shipx: f64,
    shipy: f64,
    dpar: f64,
    dtmp1: f64,
    spx: f64,
    spy: f64,
    wx: f64,
    wy: f64,
    v: f64,
    vv: f64,
    setx: i32,
    sety: i32,
    narcs: usize,

    // ---- input ----
    /// Buttons held down (bit k = `pbson[k]`).
    pub pbson: u32,
    /// Button state at the previous control poll.
    pbsint: u32,
    /// Buttons pressed since the last poll; seen by exactly one poll.
    pulse: u32,
    /// Set by the exit button.
    pub exit: bool,

    // ---- output ----
    pub display: Vec<Mark>,
    /// Screen x of the horizontal-velocity indicator, if on screen.
    pub vel_x: Option<i32>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// `start:`
    pub fn new() -> Self {
        let dec = |t: &[tables::Word3; N]| t.map(pdp7);
        let pww = dec(&tables::PWW);
        // pw[i] = 1 - pww[i]^2 / 2, the matching cosine.
        let pw = pww.map(|w| 1.0 - w * w / 2.0);
        let prsq = dec(&tables::PRSQ);
        let par = 1;
        let rpar = prsq[par].sqrt();
        Game {
            px: dec(&tables::PX),
            py: dec(&tables::PY),
            pw,
            pww,
            prsq,
            accl: dec(&tables::ACCL),
            fardst: pdp7(tables::FARDST),
            f2pi: pdp7(tables::F2PI),
            pid10: pdp7(tables::PID10),
            thrs: pdp7(tables::THRS),
            f400: pdp7(tables::F400),
            crash: pdp7(tables::CRASH),
            ascale: pdp7(tables::ASCALE),
            sdphi: pdp7(tables::SDPHI),
            cdphi: pdp7(tables::CDPHI),

            // Start landed on top of Earth, pointing straight up.
            par,
            x: 0.0,
            y: rpar,
            ox: 0.0,
            oy: rpar,
            rpar,
            sphi: 1.0,
            cphi: 0.0,
            stheta: 1.0,
            ctheta: 0.0,
            scale: 0,
            horizv: 0.0,

            lanflg: true,
            goflg: false,
            crflg: false,
            forflg: false,
            bacflg: false,
            grvflg: false,
            inflg: false,
            accflg: false,
            locflg: false,
            locpar: 0,

            ax: 0.0,
            ay: 0.0,
            maxa: 0.0,
            maxj: par,

            absx: 0.0,
            absy: 0.0,
            shipx: 0.0,
            shipy: 0.0,
            dpar: 0.0,
            dtmp1: 0.0,
            spx: 0.0,
            spy: 0.0,
            wx: 0.0,
            wy: 0.0,
            v: 0.0,
            vv: 0.0,
            setx: 0,
            sety: 0,
            narcs: 0,

            pbson: 0,
            pbsint: 0,
            pulse: 0,
            exit: false,

            display: Vec::new(),
            vel_x: None,
        }
    }

    /// Register a single press of an edge-triggered button (exit, reset,
    /// zoom). The next control poll sees it as newly pressed.
    pub fn press(&mut self, b: u32) {
        self.pulse |= 1 << b;
    }

    /// Name of the body the ship is relative to (`dispname`).
    pub fn name(&self) -> &'static str {
        tables::NAMES[self.par]
    }

    /// The scale readout (`dspsca`): sign and two digits.
    pub fn scale_text(&self) -> String {
        let sign = if self.scale < 0 { '-' } else { '+' };
        let a = self.scale.unsigned_abs();
        let tens = char::from(b'0'.wrapping_add((a / 10) as u8));
        let ones = char::from(b'0' + (a % 10) as u8);
        format!("{sign}{tens}{ones}")
    }

    /// The status readout (`dispcl`): "CL" after a crash, "L" when landed.
    pub fn status_text(&self) -> &'static str {
        if self.crflg {
            "CL"
        } else if self.lanflg {
            "L"
        } else {
            ""
        }
    }

    /// Speed relative to the current body, in Earth radii per tick.
    pub fn speed(&self) -> f64 {
        (self.x - self.ox).hypot(self.y - self.oy)
    }

    /// Distance from the centre of the current body, in Earth radii.
    pub fn altitude(&self) -> f64 {
        self.x.hypot(self.y) - self.rpar
    }

    /// One pass of `loop:`.
    pub fn tick(&mut self) {
        self.display.clear();
        self.contrl();
        self.shipxy();
        if !self.goflg {
            self.ax = 0.0;
            self.ay = 0.0;
            self.maxa = 0.0;
            for cplan in (0..N).rev() {
                self.updacc(cplan);
                self.displa(cplan);
                if cplan != 0 {
                    self.updpln(cplan);
                }
                if cplan & 7 == 7 {
                    self.contrl();
                }
            }
            if !self.lanflg {
                self.updshp();
            }
        }
        // loop3: the horizontal-velocity indicator, scaled by 2^(vscale-scale-1).
        let fac = self.horizv * pow2(tables::VSCALE - self.scale - 1);
        self.vel_x = inscr(fac).map(|ac| 895 + ac);
    }

    /// `contrl`: poll the push buttons.
    fn contrl(&mut self) {
        let has = |w: u32, b: u32| w & (1 << b) != 0;
        let now = self.pbson | self.pulse;
        self.pulse = 0;

        // Edge-triggered buttons: pressed now but not at the last poll.
        let edge = (now ^ self.pbsint) & now;
        if edge != 0 {
            if has(edge, button::EXIT) {
                self.exit = true;
            }
            if has(edge, button::RESET) {
                self.goflg = false;
                self.crflg = false;
            }
            match (has(edge, button::ZOOM_OUT), has(edge, button::ZOOM_IN)) {
                (true, false) => self.scale += 1,
                (false, true) => self.scale -= 1,
                _ => {}
            }
        }

        // noneon: level-triggered buttons.
        self.forflg = false;
        self.bacflg = false;
        self.pbsint = now;
        if has(now, button::FORWARD) {
            self.forflg = true;
            if !self.goflg {
                self.lanflg = false;
            }
        }
        if has(now, button::BACKWARD) {
            self.bacflg = true;
            if !self.goflg {
                self.lanflg = false;
            }
        }
        let s = match (has(now, button::ROT_CW), has(now, button::ROT_CCW)) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => return,
        };
        // Rotate (cphi, sphi) by ±1.2 degrees.
        let sphi = self.sphi * self.cdphi + s * self.cphi * self.sdphi;
        let cphi = self.cphi * self.cdphi - s * self.sphi * self.sdphi;
        self.sphi = sphi;
        self.cphi = cphi;
    }

    /// `shipxy` (always preceded by `absxy(par)` in the original): the
    /// negated absolute position of the ship.
    fn shipxy(&mut self) {
        let (ax, ay) = self.absxy(self.par);
        self.shipx = -(ax + self.x);
        self.shipy = -(ay + self.y);
    }

    /// `absxy`: absolute position of body `i` (sum of positions up the
    /// chain of parents).
    fn absxy(&self, mut i: usize) -> (f64, f64) {
        let (mut x, mut y) = (0.0, 0.0);
        while i != 0 {
            x += self.px[i];
            y += self.py[i];
            i = tables::PPAR[i];
        }
        (x, y)
    }

    /// `absv`: absolute velocity of body `i`, per tick. The original gets
    /// this by stepping each body in the chain back one tick (`invert`, which
    /// negates `pww` and calls `updpln`) and forward again.
    fn absv(&self, mut i: usize) -> (f64, f64) {
        let (mut vx, mut vy) = (0.0, 0.0);
        while i != 0 {
            let (x, y, c, s) = (self.px[i], self.py[i], self.pw[i], self.pww[i]);
            let bx = x * c + y * s;
            let by = -x * s + y * c;
            vx += x - bx;
            vy += y - by;
            i = tables::PPAR[i];
        }
        (vx, vy)
    }

    /// `updpln`: advance body `i` one step along its orbit.
    fn updpln(&mut self, i: usize) {
        let (x, y) = (self.px[i], self.py[i]);
        self.px[i] = x * self.pw[i] - y * self.pww[i];
        self.py[i] = x * self.pww[i] + y * self.pw[i];
    }

    /// `updacc`: gravity from body `cplan`, plus landing detection when it is
    /// the current body. Leaves the ship-to-body vector in `absx`/`absy` and
    /// the distance in `dpar` for `displa`.
    fn updacc(&mut self, cplan: usize) {
        let is_par = cplan == self.par;
        if !is_par {
            let (bx, by) = self.absxy(cplan);
            self.absx = bx + self.shipx;
            self.absy = by + self.shipy;
        }
        loop {
            // upda2
            if is_par {
                self.absx = -self.x;
                self.absy = -self.y;
            }
            self.dtmp1 = self.absx * self.absx + self.absy * self.absy;
            self.dpar = self.dtmp1.sqrt();
            if !is_par {
                break;
            }
            let vx = self.x - self.ox;
            let vy = self.oy - self.y;
            self.horizv = (vy * self.x + vx * self.y) / self.dpar;
            if self.dpar < self.rpar && !self.lanflg {
                // Touchdown.
                if vx * vx + vy * vy >= self.crash {
                    self.goflg = true;
                    self.crflg = true;
                }
                self.lanflg = true;
                // Put the ship on the surface, at rest.
                let k = self.rpar / self.dpar;
                self.x *= k;
                self.ox = self.x;
                self.y *= k;
                self.oy = self.y;
                self.shipxy();
                continue;
            }
            break;
        }
        // upda5
        if self.dpar > self.fardst && cplan != 0 {
            self.grvflg = true;
            return;
        }
        self.grvflg = false;
        let a = self.accl[cplan] / self.dtmp1;
        if a >= self.maxa {
            self.maxa = a;
            self.maxj = cplan;
        }
        let f = a / self.dpar;
        self.ax += f * self.absx;
        self.ay += f * self.absy;
    }

    /// `updshp`: thrust, integrate the ship, and switch to the body with the
    /// strongest pull when that changes.
    fn updshp(&mut self) {
        if self.forflg || self.bacflg {
            // ascale is about -0.5; thrust is 0.5 screen units per tick^2.
            let mut fac = if self.forflg && self.bacflg {
                0.0
            } else {
                let t = -self.ascale * pow2(self.scale);
                if self.forflg { t } else { -t }
            };
            if self.forflg && !self.bacflg && self.accflg {
                fac -= self.maxa;
            }
            self.ax += fac * self.ctheta;
            self.ay += fac * self.stheta;
        }
        // Verlet step: new = 2*now - old + a.
        let nx = 2.0 * self.x - self.ox + self.ax;
        self.ox = self.x;
        self.x = nx;
        let ny = 2.0 * self.y - self.oy + self.ay;
        self.oy = self.y;
        self.y = ny;

        if self.par == self.maxj {
            return;
        }
        // Re-express position and velocity relative to the new body.
        self.shipxy();
        let (vx, vy) = self.absv(self.par);
        self.ox = self.x - self.ox + vx; // absolute ship velocity
        self.oy = self.y - self.oy + vy;
        self.par = self.maxj;
        let (vx, vy) = self.absv(self.par);
        self.ox = vx - self.ox; // minus velocity relative to the new body
        self.oy = vy - self.oy;
        let (bx, by) = self.absxy(self.par);
        self.x = -(bx + self.shipx);
        self.ox += self.x;
        self.y = -(by + self.shipy);
        self.oy += self.y;
        self.rpar = self.prsq[self.par].sqrt();
    }

    /// `displa`: put body `cplan` on the display list.
    fn displa(&mut self, cplan: usize) {
        if cplan == self.locpar {
            if self.locflg {
                // Lock the view onto the direction of locpar.
                self.stheta = (self.sphi * self.absx + self.cphi * self.absy) / self.dpar;
                self.ctheta = (self.cphi * self.absx - self.sphi * self.absy) / self.dpar;
            } else {
                self.stheta = self.sphi;
                self.ctheta = self.cphi;
            }
        }
        let k = pow2(-self.scale);
        self.spy = (self.absx * self.ctheta + self.absy * self.stheta) * k;
        self.inflg = false;
        if let Some(sy) = inscr(self.spy) {
            self.rotx();
            self.inflg = true;
            if let Some(sx) = inscr(self.spx) {
                // dsplanet
                self.display.push(Mark::Point {
                    x: 895 + sx,
                    y: 1023 + sy,
                    br: tables::POINTS[cplan],
                });
            }
        }
        self.drcirc(cplan);
    }

    /// `rotx`: the screen x component of the ship-to-body vector.
    fn rotx(&mut self) {
        self.spx = (self.absx * self.stheta - self.absy * self.ctheta) * pow2(-self.scale);
    }

    /// `drcirc`: draw the outline of body `cplan`, starting from the point
    /// nearest the ship and working round both ways until it leaves the view.
    fn drcirc(&mut self, cplan: usize) {
        if self.grvflg {
            return;
        }
        let k = pow2(-self.scale);
        let r = self.prsq[cplan].sqrt() * k;
        if r < self.thrs {
            return;
        }
        self.dpar *= k;
        let dtmp2 = (self.dpar - r) / self.dpar;
        self.wy = dtmp2 * self.spy;
        let Some(sety) = inscr(self.wy) else { return };
        self.sety = sety;
        if !self.inflg {
            self.rotx();
        }
        self.wx = dtmp2 * self.spx;
        let Some(setx) = inscr(self.wx) else { return };
        self.setx = setx;
        // From centre to the nearest surface point.
        self.wy -= self.spy;
        self.wx -= self.spx;

        // Arc count: r*pi/10, at least 20. Past 400 only part of the outline
        // (400 arcs of 20/r radians) is drawn.
        let fac = r * self.pid10;
        let steps = if fac >= self.f400 {
            self.narcs = 400;
            fac
        } else {
            self.narcs = (fac.floor() as usize).max(20);
            self.narcs as f64
        };
        self.v = self.f2pi / steps;
        self.vv = 1.0 - self.v * self.v / 2.0;
        if !self.surf() {
            self.surf();
        }
    }

    /// `surf`: walk the outline in one direction (reversing `v` first).
    /// Returns false if it ran off the view.
    fn surf(&mut self) -> bool {
        let (mut cx, mut cy) = (895 + self.setx, 1023 + self.sety);
        let (mut twx, mut twy) = (self.wx, self.wy);
        self.v = -self.v;
        let (v, vv) = (self.v, self.vv);
        for _ in 0..self.narcs {
            let nx = vv * twx - v * twy;
            twy = vv * twy + v * twx;
            let Some(sy) = inscr(twy + self.spy) else {
                return false;
            };
            twx = nx;
            let Some(sx) = inscr(twx + self.spx) else {
                return false;
            };
            let (x, y) = (895 + sx, 1023 + sy);
            self.display.push(Mark::Line {
                x1: cx,
                y1: cy,
                x2: x,
                y2: y,
            });
            (cx, cy) = (x, y);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inscr_matches_view_box() {
        // A body at the ship's position lands next to the crosshair (511, 639).
        assert_eq!(inscr(0.0).map(|a| 895 + a), Some(510));
        assert_eq!(inscr(0.0).map(|a| 1023 + a), Some(638));
        assert!(inscr(383.0).is_some());
        assert!(inscr(384.0).is_none());
        assert!(inscr(-384.0).is_some());
        assert!(inscr(-385.0).is_none());
    }

    #[test]
    fn sits_still_while_landed() {
        let mut g = Game::new();
        for _ in 0..100 {
            g.tick();
        }
        assert!(g.lanflg && !g.crflg);
        assert_eq!(g.par, 1);
        assert_eq!(g.altitude(), 0.0);
    }

    #[test]
    fn earth_gravity_is_one_g() {
        // Surface gravity of Earth: about 9.8 m/s^2 = 1.54e-6 R/s^2.
        let mut g = Game::new();
        g.tick();
        let earth = g.accl[1] / g.rpar.powi(2);
        assert!((earth * 6.371e6 - 9.8).abs() < 0.2, "{}", earth * 6.371e6);
    }

    #[test]
    fn launch_and_fall_back() {
        let mut g = Game::new();
        // At scale -18 thrust is ~1.2 g, just enough to lift off.
        g.scale = -18;
        g.pbson = 1 << button::FORWARD;
        for _ in 0..30 {
            g.tick();
        }
        assert!(!g.lanflg);
        assert!(g.altitude() > 0.0);
        g.pbson = 0;
        let mut landed = false;
        for _ in 0..10_000 {
            g.tick();
            if g.lanflg {
                landed = true;
                break;
            }
        }
        assert!(landed, "should fall back to Earth");
        assert_eq!(g.par, 1);
        assert!(!g.crflg, "slow fall should not crash");
    }

    #[test]
    fn fast_landing_crashes() {
        let mut g = Game::new();
        g.scale = -10; // strong thrust
        g.pbson = 1 << button::FORWARD;
        for _ in 0..3 {
            g.tick();
        }
        g.pbson = 1 << button::BACKWARD;
        for _ in 0..200 {
            g.tick();
            if g.lanflg {
                break;
            }
        }
        assert!(g.crflg && g.goflg);
        // Reset clears the crash; the ship stays landed.
        g.pbson = 0;
        g.press(button::RESET);
        g.tick();
        assert!(!g.crflg && !g.goflg && g.lanflg);
    }

    #[test]
    fn rotation_is_one_point_two_degrees_per_poll() {
        let mut g = Game::new();
        g.goflg = true; // only one poll per tick
        g.pbson = 1 << button::ROT_CCW;
        g.tick();
        let ang = g.sphi.atan2(g.cphi).to_degrees();
        assert!((ang - 91.2).abs() < 1e-6, "{ang}");
    }

    #[test]
    fn zoom_is_edge_triggered() {
        let mut g = Game::new();
        g.press(button::ZOOM_IN);
        g.tick();
        g.tick();
        assert_eq!(g.scale, -1);
        assert_eq!(g.scale_text(), "-01");
    }

    #[test]
    fn earth_outline_drawn_when_zoomed() {
        let mut g = Game::new();
        g.scale = -8; // Earth radius = 256 screen units
        g.tick();
        let lines = g
            .display
            .iter()
            .filter(|m| matches!(m, Mark::Line { .. }))
            .count();
        assert!(lines > 20, "{lines}");
    }

    #[test]
    fn low_earth_orbit_is_stable() {
        let mut g = Game::new();
        let r = 1.1;
        let v = (g.accl[1] / r).sqrt();
        g.lanflg = false;
        (g.x, g.y, g.ox, g.oy) = (0.0, r, v, r);
        // One revolution: 2*pi*r/v ticks (about 1.7 hours).
        let n = (std::f64::consts::TAU * r / v) as usize;
        let mut lo = f64::MAX;
        let mut hi = 0f64;
        for _ in 0..n {
            g.tick();
            let d = g.x.hypot(g.y);
            lo = lo.min(d);
            hi = hi.max(d);
        }
        assert_eq!(g.par, 1);
        assert!(!g.lanflg);
        assert!(hi - lo < 0.01, "radius drifted {lo}..{hi}");
        assert!(
            g.x.abs() < 0.05 && (g.y - r).abs() < 0.05,
            "{} {}",
            g.x,
            g.y
        );
    }

    #[test]
    fn switches_to_moon_near_moon() {
        let mut g = Game::new();
        g.lanflg = false;
        // Park the ship 2 Earth radii from the Moon, at rest relative to Earth.
        let (mx, my) = (g.px[4], g.py[4]);
        (g.x, g.y) = (mx, my + 2.0);
        (g.ox, g.oy) = (g.x, g.y);
        g.tick();
        assert_eq!(g.name(), "moon");
        // Position is now relative to the Moon, velocity preserved (tiny).
        assert!((g.x.hypot(g.y) - 2.0).abs() < 1e-6, "{} {}", g.x, g.y);
        assert!(g.speed() < 1e-5);
        // And it falls onto the Moon.
        for _ in 0..20_000 {
            g.tick();
            if g.lanflg {
                break;
            }
        }
        assert!(g.lanflg && g.name() == "moon");
    }
}
